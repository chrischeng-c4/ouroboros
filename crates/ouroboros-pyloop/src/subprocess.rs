//! Subprocess support for ouroboros-pyloop
//!
//! This module provides async subprocess execution primitives that integrate
//! with the Tokio runtime used by PyLoop.
//!
//! # Features
//!
//! - Process spawning via `create_subprocess_exec` and `create_subprocess_shell`
//! - Async stdin/stdout/stderr streaming
//! - Exit code handling
//! - Signal handling (kill, terminate)
//! - Environment variable configuration
//! - Working directory support
//! - Process timeout
//!
//! # Example
//!
//! ```rust,no_run
//! use ouroboros_pyloop::subprocess::{Process, create_subprocess_exec};
//!
//! # async fn example() -> std::io::Result<()> {
//! let mut proc = create_subprocess_exec("ls", &["-l"]).await?;
//! let output = proc.communicate().await?;
//! println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use std::process::ExitStatus;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;

// ============================================================================
// Process Output
// ============================================================================

/// Output from a subprocess
#[derive(Debug, Clone, Default)]
pub struct ProcessOutput {
    /// Standard output bytes
    pub stdout: Vec<u8>,
    /// Standard error bytes
    pub stderr: Vec<u8>,
    /// Exit code (None if process was killed)
    pub returncode: Option<i32>,
}

impl ProcessOutput {
    /// Get stdout as string (lossy UTF-8 conversion)
    pub fn stdout_str(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }

    /// Get stderr as string (lossy UTF-8 conversion)
    pub fn stderr_str(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }

    /// Check if the process completed successfully (exit code 0)
    pub fn success(&self) -> bool {
        self.returncode == Some(0)
    }
}

// ============================================================================
// Process Configuration
// ============================================================================

/// Configuration for subprocess execution
#[derive(Debug, Clone, Default)]
pub struct ProcessConfig {
    /// Working directory for the subprocess
    pub cwd: Option<PathBuf>,
    /// Environment variables (overrides)
    pub env: HashMap<String, String>,
    /// Whether to clear the environment before applying `env`
    pub env_clear: bool,
    /// Capture stdout
    pub capture_stdout: bool,
    /// Capture stderr
    pub capture_stderr: bool,
    /// Allow stdin writes
    pub pipe_stdin: bool,
}

impl ProcessConfig {
    /// Create a new default configuration
    pub fn new() -> Self {
        Self {
            capture_stdout: true,
            capture_stderr: true,
            ..Default::default()
        }
    }

    /// Set working directory
    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.cwd = Some(path.into());
        self
    }

    /// Set an environment variable
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Set multiple environment variables
    pub fn envs<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (k, v) in vars {
            self.env.insert(k.into(), v.into());
        }
        self
    }

    /// Clear environment before setting variables
    pub fn env_clear(mut self) -> Self {
        self.env_clear = true;
        self
    }

    /// Enable stdin piping
    pub fn stdin(mut self, pipe: bool) -> Self {
        self.pipe_stdin = pipe;
        self
    }

    /// Configure stdout capture
    pub fn stdout(mut self, capture: bool) -> Self {
        self.capture_stdout = capture;
        self
    }

    /// Configure stderr capture
    pub fn stderr(mut self, capture: bool) -> Self {
        self.capture_stderr = capture;
        self
    }
}

// ============================================================================
// Process
// ============================================================================

/// An async subprocess
///
/// Wraps a Tokio Child process and provides asyncio-compatible methods.
pub struct Process {
    /// The underlying child process
    child: Arc<Mutex<Option<Child>>>,
    /// Standard input stream (if piped)
    stdin: Arc<Mutex<Option<ChildStdin>>>,
    /// Standard output stream (if captured)
    stdout: Arc<Mutex<Option<ChildStdout>>>,
    /// Standard error stream (if captured)
    stderr: Arc<Mutex<Option<ChildStderr>>>,
    /// Process ID
    pid: u32,
    /// Whether the process has been killed
    killed: AtomicBool,
    /// Cached return code
    returncode: Arc<Mutex<Option<i32>>>,
}

impl Process {
    /// Get the process ID
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Check if the process has been killed
    pub fn killed(&self) -> bool {
        self.killed.load(Ordering::Acquire)
    }

    /// Get the return code (None if still running or killed)
    pub async fn get_returncode(&self) -> Option<i32> {
        *self.returncode.lock().await
    }

    /// Write data to stdin
    pub async fn write_stdin(&self, data: &[u8]) -> io::Result<()> {
        let mut stdin_guard = self.stdin.lock().await;
        if let Some(ref mut stdin) = *stdin_guard {
            stdin.write_all(data).await?;
            stdin.flush().await?;
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "stdin not available",
            ))
        }
    }

    /// Close stdin (signals EOF to the process)
    pub async fn close_stdin(&self) -> io::Result<()> {
        let mut stdin_guard = self.stdin.lock().await;
        *stdin_guard = None; // Drop the stdin handle
        Ok(())
    }

    /// Read all data from stdout
    pub async fn read_stdout(&self) -> io::Result<Vec<u8>> {
        let mut stdout_guard = self.stdout.lock().await;
        if let Some(ref mut stdout) = *stdout_guard {
            let mut buf = Vec::new();
            stdout.read_to_end(&mut buf).await?;
            Ok(buf)
        } else {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "stdout not available",
            ))
        }
    }

    /// Read all data from stderr
    pub async fn read_stderr(&self) -> io::Result<Vec<u8>> {
        let mut stderr_guard = self.stderr.lock().await;
        if let Some(ref mut stderr) = *stderr_guard {
            let mut buf = Vec::new();
            stderr.read_to_end(&mut buf).await?;
            Ok(buf)
        } else {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "stderr not available",
            ))
        }
    }

    /// Wait for the process to complete and return exit status
    pub async fn wait(&self) -> io::Result<ExitStatus> {
        let mut child_guard = self.child.lock().await;
        if let Some(ref mut child) = *child_guard {
            let status = child.wait().await?;
            *self.returncode.lock().await = status.code();
            Ok(status)
        } else {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "process already completed",
            ))
        }
    }

    /// Send data to stdin, close stdin, read stdout/stderr, and wait for completion
    ///
    /// This is the asyncio-style `communicate()` method.
    pub async fn communicate(&self) -> io::Result<ProcessOutput> {
        self.communicate_with_input(None).await
    }

    /// Like `communicate()` but with optional input data
    pub async fn communicate_with_input(&self, input: Option<&[u8]>) -> io::Result<ProcessOutput> {
        // Write input if provided
        if let Some(data) = input {
            // Try to write, ignore error if stdin not piped
            let _ = self.write_stdin(data).await;
        }

        // Close stdin to signal EOF
        let _ = self.close_stdin().await;

        // Read stdout and stderr concurrently
        let stdout_handle = {
            let mut stdout_guard = self.stdout.lock().await;
            if let Some(mut stdout) = stdout_guard.take() {
                Some(tokio::spawn(async move {
                    let mut buf = Vec::new();
                    stdout.read_to_end(&mut buf).await.map(|_| buf)
                }))
            } else {
                None
            }
        };

        let stderr_handle = {
            let mut stderr_guard = self.stderr.lock().await;
            if let Some(mut stderr) = stderr_guard.take() {
                Some(tokio::spawn(async move {
                    let mut buf = Vec::new();
                    stderr.read_to_end(&mut buf).await.map(|_| buf)
                }))
            } else {
                None
            }
        };

        // Wait for process completion
        let status = self.wait().await?;

        // Collect outputs
        let stdout = if let Some(handle) = stdout_handle {
            handle.await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))??
        } else {
            Vec::new()
        };

        let stderr = if let Some(handle) = stderr_handle {
            handle.await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))??
        } else {
            Vec::new()
        };

        Ok(ProcessOutput {
            stdout,
            stderr,
            returncode: status.code(),
        })
    }

    /// Kill the process with SIGKILL
    pub async fn kill(&self) -> io::Result<()> {
        let mut child_guard = self.child.lock().await;
        if let Some(ref mut child) = *child_guard {
            child.kill().await?;
            self.killed.store(true, Ordering::Release);
        }
        Ok(())
    }

    /// Terminate the process with SIGTERM (Unix) or TerminateProcess (Windows)
    #[cfg(unix)]
    pub async fn terminate(&self) -> io::Result<()> {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        let pid = Pid::from_raw(self.pid as i32);
        kill(pid, Signal::SIGTERM)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        Ok(())
    }

    #[cfg(not(unix))]
    pub async fn terminate(&self) -> io::Result<()> {
        // On non-Unix, fall back to kill
        self.kill().await
    }

    /// Send a signal to the process (Unix only)
    #[cfg(unix)]
    pub fn send_signal(&self, signal: i32) -> io::Result<()> {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        let pid = Pid::from_raw(self.pid as i32);
        let sig = Signal::try_from(signal)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
        kill(pid, sig).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        Ok(())
    }

    #[cfg(not(unix))]
    pub fn send_signal(&self, _signal: i32) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "send_signal not supported on this platform",
        ))
    }

    /// Poll the process for completion without blocking
    ///
    /// Returns the exit code if completed, None if still running.
    pub async fn poll(&self) -> io::Result<Option<i32>> {
        let mut child_guard = self.child.lock().await;
        if let Some(ref mut child) = *child_guard {
            match child.try_wait()? {
                Some(status) => {
                    let code = status.code();
                    *self.returncode.lock().await = code;
                    Ok(code)
                }
                None => Ok(None),
            }
        } else {
            // Process already consumed
            Ok(*self.returncode.lock().await)
        }
    }
}

impl std::fmt::Debug for Process {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Process")
            .field("pid", &self.pid)
            .field("killed", &self.killed.load(Ordering::Relaxed))
            .finish()
    }
}

// ============================================================================
// Process Creation Functions
// ============================================================================

/// Create a subprocess from an executable and arguments
///
/// This is equivalent to `asyncio.create_subprocess_exec()`.
pub async fn create_subprocess_exec(
    program: &str,
    args: &[&str],
) -> io::Result<Process> {
    create_subprocess_exec_with_config(program, args, ProcessConfig::new()).await
}

/// Create a subprocess with custom configuration
pub async fn create_subprocess_exec_with_config(
    program: &str,
    args: &[&str],
    config: ProcessConfig,
) -> io::Result<Process> {
    let mut cmd = Command::new(program);
    cmd.args(args);

    // Apply configuration
    if let Some(ref cwd) = config.cwd {
        cmd.current_dir(cwd);
    }

    if config.env_clear {
        cmd.env_clear();
    }

    for (key, value) in &config.env {
        cmd.env(key, value);
    }

    // Configure stdio
    if config.pipe_stdin {
        cmd.stdin(std::process::Stdio::piped());
    } else {
        cmd.stdin(std::process::Stdio::null());
    }

    if config.capture_stdout {
        cmd.stdout(std::process::Stdio::piped());
    } else {
        cmd.stdout(std::process::Stdio::null());
    }

    if config.capture_stderr {
        cmd.stderr(std::process::Stdio::piped());
    } else {
        cmd.stderr(std::process::Stdio::null());
    }

    // Spawn the process
    let mut child = cmd.spawn()?;
    let pid = child.id().unwrap_or(0);

    // Extract stdio handles
    let stdin = child.stdin.take();
    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    Ok(Process {
        child: Arc::new(Mutex::new(Some(child))),
        stdin: Arc::new(Mutex::new(stdin)),
        stdout: Arc::new(Mutex::new(stdout)),
        stderr: Arc::new(Mutex::new(stderr)),
        pid,
        killed: AtomicBool::new(false),
        returncode: Arc::new(Mutex::new(None)),
    })
}

/// Create a subprocess using shell interpretation
///
/// This is equivalent to `asyncio.create_subprocess_shell()`.
/// The command is passed to the shell (/bin/sh on Unix, cmd.exe on Windows).
pub async fn create_subprocess_shell(cmd: &str) -> io::Result<Process> {
    create_subprocess_shell_with_config(cmd, ProcessConfig::new()).await
}

/// Create a subprocess using shell interpretation with custom configuration
pub async fn create_subprocess_shell_with_config(
    cmd: &str,
    config: ProcessConfig,
) -> io::Result<Process> {
    #[cfg(unix)]
    {
        create_subprocess_exec_with_config("/bin/sh", &["-c", cmd], config).await
    }
    #[cfg(windows)]
    {
        create_subprocess_exec_with_config("cmd.exe", &["/C", cmd], config).await
    }
}

/// Run a subprocess and wait for completion
///
/// Convenience function that spawns a process, waits for completion,
/// and returns the output. Similar to `subprocess.run()` in Python.
pub async fn run(program: &str, args: &[&str]) -> io::Result<ProcessOutput> {
    let proc = create_subprocess_exec(program, args).await?;
    proc.communicate().await
}

/// Run a subprocess with input data
pub async fn run_with_input(
    program: &str,
    args: &[&str],
    input: &[u8],
) -> io::Result<ProcessOutput> {
    let config = ProcessConfig::new().stdin(true);
    let proc = create_subprocess_exec_with_config(program, args, config).await?;
    proc.communicate_with_input(Some(input)).await
}

/// Run a shell command and wait for completion
pub async fn run_shell(cmd: &str) -> io::Result<ProcessOutput> {
    let proc = create_subprocess_shell(cmd).await?;
    proc.communicate().await
}

/// Run a subprocess with timeout
pub async fn run_with_timeout(
    program: &str,
    args: &[&str],
    timeout_ms: u64,
) -> io::Result<ProcessOutput> {
    let proc = create_subprocess_exec(program, args).await?;
    let duration = std::time::Duration::from_millis(timeout_ms);

    match tokio::time::timeout(duration, proc.communicate()).await {
        Ok(result) => result,
        Err(_) => {
            proc.kill().await?;
            Err(io::Error::new(io::ErrorKind::TimedOut, "Process timed out"))
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_echo() {
        let output = run("echo", &["hello"]).await.unwrap();
        assert!(output.success());
        assert!(output.stdout_str().contains("hello"));
    }

    #[tokio::test]
    async fn test_run_shell() {
        let output = run_shell("echo 'hello world'").await.unwrap();
        assert!(output.success());
        assert!(output.stdout_str().contains("hello"));
    }

    #[tokio::test]
    async fn test_run_with_input() {
        let output = run_with_input("cat", &[], b"test input").await.unwrap();
        assert!(output.success());
        assert_eq!(output.stdout, b"test input");
    }

    #[tokio::test]
    async fn test_exit_code() {
        let output = run_shell("exit 42").await.unwrap();
        assert_eq!(output.returncode, Some(42));
        assert!(!output.success());
    }

    #[tokio::test]
    async fn test_stderr_capture() {
        let output = run_shell("echo error >&2").await.unwrap();
        assert!(output.success());
        assert!(output.stderr_str().contains("error"));
    }

    #[tokio::test]
    async fn test_process_pid() {
        let proc = create_subprocess_exec("sleep", &["0.1"]).await.unwrap();
        assert!(proc.pid() > 0);
        let _ = proc.wait().await;
    }

    #[tokio::test]
    async fn test_process_poll() {
        let proc = create_subprocess_exec("sleep", &["0.5"]).await.unwrap();

        // Should not be complete yet
        let poll_result = proc.poll().await.unwrap();
        assert!(poll_result.is_none());

        // Wait for completion
        let _ = proc.wait().await.unwrap();

        // Now should have exit code
        let poll_result = proc.poll().await.unwrap();
        assert_eq!(poll_result, Some(0));
    }

    #[tokio::test]
    async fn test_process_config_cwd() {
        let config = ProcessConfig::new().cwd("/tmp");
        let proc = create_subprocess_exec_with_config("pwd", &[], config)
            .await
            .unwrap();
        let output = proc.communicate().await.unwrap();

        // macOS uses /private/tmp as the real path
        let stdout = output.stdout_str();
        assert!(stdout.contains("tmp"), "Expected /tmp in output: {}", stdout);
    }

    #[tokio::test]
    async fn test_process_config_env() {
        let config = ProcessConfig::new().env("MY_VAR", "my_value");
        let proc = create_subprocess_exec_with_config("env", &[], config)
            .await
            .unwrap();
        let output = proc.communicate().await.unwrap();
        assert!(output.stdout_str().contains("MY_VAR=my_value"));
    }

    #[tokio::test]
    async fn test_run_with_timeout_success() {
        let output = run_with_timeout("echo", &["fast"], 5000).await.unwrap();
        assert!(output.success());
    }

    #[tokio::test]
    async fn test_run_with_timeout_exceeded() {
        let result = run_with_timeout("sleep", &["10"], 100).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::TimedOut);
    }

    #[tokio::test]
    async fn test_process_debug() {
        let proc = create_subprocess_exec("echo", &["test"]).await.unwrap();
        let debug_str = format!("{:?}", proc);
        assert!(debug_str.contains("Process"));
        assert!(debug_str.contains("pid"));
        let _ = proc.communicate().await;
    }
}
