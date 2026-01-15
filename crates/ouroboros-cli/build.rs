//! Build script to configure Python linking for embedding

fn main() {
    // Get Python configuration
    let python = std::process::Command::new("python3")
        .args(["-c", "import sysconfig; print(sysconfig.get_config_var('LIBDIR'))"])
        .output()
        .expect("Failed to run python3");

    let libdir = String::from_utf8_lossy(&python.stdout)
        .trim()
        .to_string();

    if !libdir.is_empty() && libdir != "None" {
        println!("cargo:rustc-link-search=native={}", libdir);
    }

    // Get Python library name
    let python = std::process::Command::new("python3")
        .args(["-c", "import sysconfig; v = sysconfig.get_config_var('VERSION'); print(f'python{v}')"])
        .output()
        .expect("Failed to run python3");

    let libname = String::from_utf8_lossy(&python.stdout)
        .trim()
        .to_string();

    if !libname.is_empty() {
        println!("cargo:rustc-link-lib={}", libname);
    }

    // On macOS, we need the framework path
    #[cfg(target_os = "macos")]
    {
        // Add rpath for finding the dynamic library at runtime
        let python = std::process::Command::new("python3")
            .args(["-c", "import sysconfig; print(sysconfig.get_config_var('LIBDIR'))"])
            .output()
            .expect("Failed to run python3");

        let libdir = String::from_utf8_lossy(&python.stdout)
            .trim()
            .to_string();

        if !libdir.is_empty() && libdir != "None" {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libdir);
        }
    }
}
