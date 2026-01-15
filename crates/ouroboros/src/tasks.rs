//! PyO3 bindings for ouroboros-tasks
//!
//! Provides Python API for distributed task queue - a high-performance
//! Celery replacement built in Rust.
//!
//! # Architecture
//!
//! - **Task Registration**: Tasks registered via @task decorator
//! - **Broker**: NATS for message passing
//! - **Backend**: Redis for result storage
//! - **Worker**: Rust-based task executor
//! - **Workflows**: Chain, Group, Chord primitives
//!
//! # Example
//!
//! ```python
//! from ouroboros.tasks import task, init
//!
//! # Initialize
//! await init(nats_url="nats://localhost:4222", redis_url="redis://localhost:6379")
//!
//! # Define task
//! @task(name="add", queue="math")
//! async def add(x: int, y: int) -> int:
//!     return x + y
//!
//! # Execute
//! result = await add.delay(1, 2)
//! print(await result.get())  # 3
//! ```

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple};
use pyo3_async_runtimes::tokio::future_into_py;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

// Import from ouroboros-tasks
use ouroboros_tasks::{
    Broker, DelayedBroker, RedisBackend, RedisBackendConfig, ResultBackend,
    TaskError, TaskId, TaskMessage, TaskOptions, TaskSignature, TaskState,
    Chain, Group, Chord, GroupResult,
};
use ouroboros_tasks::broker::{BrokerConfig, BrokerInstance};

#[cfg(feature = "tasks-nats")]
use ouroboros_tasks::broker::nats::NatsBrokerConfig;

#[cfg(feature = "tasks-pubsub")]
use ouroboros_tasks::broker::pubsub::PubSubPullConfig;

// Global state for broker and backend
static BROKER: RwLock<Option<Arc<BrokerInstance>>> = RwLock::const_new(None);
static BACKEND: RwLock<Option<Arc<RedisBackend>>> = RwLock::const_new(None);

/// Initialize the task queue system.
///
/// # Arguments
/// * `redis_url` - Redis URL for result backend
/// * `broker_type` - Optional broker type: "nats" (default) or "pubsub"
/// * `nats_url` - NATS URL (used when broker_type is "nats")
/// * `pubsub_project_id` - GCP project ID (used when broker_type is "pubsub")
/// * `pubsub_topic` - Pub/Sub topic name (default: "tasks")
/// * `pubsub_subscription` - Pub/Sub subscription (default: "task-worker")
///
/// # Examples
/// ```python
/// # NATS (default)
/// await init(redis_url="redis://localhost:6379", nats_url="nats://localhost:4222")
///
/// # Google Cloud Pub/Sub
/// await init(
///     redis_url="redis://localhost:6379",
///     broker_type="pubsub",
///     pubsub_project_id="my-project",
/// )
///
/// # From environment variables
/// await init(redis_url="redis://localhost:6379")
/// # Reads BROKER_TYPE, NATS_URL, PUBSUB_PROJECT_ID, etc.
/// ```
#[pyfunction]
#[pyo3(signature = (
    redis_url,
    broker_type = None,
    nats_url = None,
    pubsub_project_id = None,
    pubsub_topic = None,
    pubsub_subscription = None,
))]
fn init<'py>(
    py: Python<'py>,
    redis_url: String,
    broker_type: Option<String>,
    nats_url: Option<String>,
    pubsub_project_id: Option<String>,
    pubsub_topic: Option<String>,
    pubsub_subscription: Option<String>,
) -> PyResult<Bound<'py, PyAny>> {
    future_into_py(py, async move {
        // Determine broker config
        let broker_config = if let Some(bt) = broker_type {
            match bt.to_lowercase().as_str() {
                #[cfg(feature = "tasks-nats")]
                "nats" => {
                    let url = nats_url.unwrap_or_else(|| "nats://localhost:4222".to_string());
                    BrokerConfig::Nats(NatsBrokerConfig {
                        url,
                        ..Default::default()
                    })
                }
                #[cfg(not(feature = "tasks-nats"))]
                "nats" => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        "NATS support not compiled. Enable 'tasks-nats' feature."
                    ));
                }
                #[cfg(feature = "tasks-pubsub")]
                "pubsub" | "gcp" => {
                    BrokerConfig::PubSub(PubSubPullConfig {
                        project_id: pubsub_project_id,
                        topic_name: pubsub_topic.unwrap_or_else(|| "tasks".to_string()),
                        subscription_name: pubsub_subscription.unwrap_or_else(|| "task-worker".to_string()),
                        ..Default::default()
                    })
                }
                #[cfg(not(feature = "tasks-pubsub"))]
                "pubsub" | "gcp" => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        "Pub/Sub support not compiled. Enable 'tasks-pubsub' feature."
                    ));
                }
                other => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        format!("Unknown broker type: '{}'. Available: {:?}", other, BrokerConfig::available_types())
                    ));
                }
            }
        } else if let Some(url) = nats_url {
            // If nats_url provided without broker_type, use NATS
            #[cfg(feature = "tasks-nats")]
            {
                BrokerConfig::Nats(NatsBrokerConfig {
                    url,
                    ..Default::default()
                })
            }
            #[cfg(not(feature = "tasks-nats"))]
            {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "NATS support not compiled. Enable 'tasks-nats' feature."
                ));
            }
        } else {
            // Try from environment
            BrokerConfig::from_env().map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string())
            })?
        };

        // Create broker
        let mut broker = broker_config.into_broker();

        // Connect broker based on type
        match &mut broker {
            #[cfg(feature = "tasks-nats")]
            BrokerInstance::Nats(b) => {
                b.connect().await.map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Failed to connect to NATS: {}",
                        e
                    ))
                })?;
            }
            #[cfg(feature = "tasks-pubsub")]
            BrokerInstance::PubSub(b) => {
                b.connect().await.map_err(|e| {
                    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                        "Failed to connect to Pub/Sub: {}",
                        e
                    ))
                })?;
            }
            #[allow(unreachable_patterns)]
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    "Broker type not supported in this build. Check feature flags."
                ));
            }
        }

        // Create Redis backend
        let backend_config = RedisBackendConfig {
            url: redis_url,
            ..Default::default()
        };
        let backend = RedisBackend::new(backend_config).await.map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyConnectionError, _>(format!(
                "Failed to connect to Redis: {}",
                e
            ))
        })?;

        // Store in globals
        *BROKER.write().await = Some(Arc::new(broker));
        *BACKEND.write().await = Some(Arc::new(backend));

        Ok(())
    })
}

/// A task that can be executed asynchronously
///
/// Created via the @task decorator. Provides methods to execute tasks
/// and create workflow signatures.
#[pyclass(name = "Task")]
pub struct PyTask {
    name: String,
    queue: String,
    max_retries: u32,
    retry_delay_secs: f64,
}

#[pymethods]
impl PyTask {
    #[new]
    #[pyo3(signature = (name, queue = "default".to_string(), max_retries = 3, retry_delay_secs = 1.0))]
    fn new(name: String, queue: String, max_retries: u32, retry_delay_secs: f64) -> Self {
        Self {
            name,
            queue,
            max_retries,
            retry_delay_secs,
        }
    }

    /// Send task for async execution with positional args
    ///
    /// # Arguments
    ///
    /// * `args` - Positional arguments (must be JSON-serializable)
    /// * `kwargs` - Keyword arguments (must be JSON-serializable)
    ///
    /// # Returns
    ///
    /// AsyncResult handle to track task execution
    ///
    /// # Example
    ///
    /// ```python
    /// result = await add.delay(1, 2)
    /// ```
    #[pyo3(signature = (*args, **kwargs))]
    fn delay<'py>(
        &self,
        py: Python<'py>,
        args: &Bound<'_, PyTuple>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.apply_async_impl(py, args, kwargs, None, None)
    }

    /// Send task with options
    ///
    /// # Arguments
    ///
    /// * `args` - Positional arguments
    /// * `countdown` - Delay in seconds before execution
    /// * `eta` - ISO 8601 timestamp for scheduled execution
    /// * `kwargs` - Keyword arguments
    ///
    /// # Example
    ///
    /// ```python
    /// # Delay by 10 seconds
    /// result = await add.apply_async(1, 2, countdown=10)
    ///
    /// # Schedule for specific time
    /// result = await add.apply_async(1, 2, eta="2026-01-05T10:00:00Z")
    /// ```
    #[pyo3(signature = (*args, countdown = None, eta = None, **kwargs))]
    fn apply_async<'py>(
        &self,
        py: Python<'py>,
        args: &Bound<'_, PyTuple>,
        countdown: Option<f64>,
        eta: Option<String>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        self.apply_async_impl(py, args, kwargs, countdown, eta)
    }

    /// Create a signature for this task (for workflows)
    ///
    /// # Example
    ///
    /// ```python
    /// # Create signature
    /// sig = add.s(1, 2)
    ///
    /// # Use in chain
    /// chain = Chain([add.s(1, 2), multiply.s(3)])
    /// ```
    #[pyo3(signature = (*args, **kwargs))]
    fn s(
        &self,
        args: &Bound<'_, PyTuple>,
        kwargs: Option<&Bound<'_, PyDict>>,
    ) -> PyResult<PyTaskSignature> {
        // Convert args to JSON
        let args_json = python_to_json(args)?;
        let kwargs_json = kwargs
            .map(|k| python_to_json(k))
            .transpose()?
            .unwrap_or(serde_json::Value::Null);

        Ok(PyTaskSignature {
            task_name: self.name.clone(),
            args: args_json,
            kwargs: kwargs_json,
            queue: self.queue.clone(),
            max_retries: self.max_retries,
            retry_delay_secs: self.retry_delay_secs,
        })
    }

    /// Get task name
    #[getter]
    fn name(&self) -> String {
        self.name.clone()
    }

    /// Get queue name
    #[getter]
    fn queue(&self) -> String {
        self.queue.clone()
    }
}

impl PyTask {
    /// Internal implementation of apply_async
    fn apply_async_impl<'py>(
        &self,
        py: Python<'py>,
        args: &Bound<'_, PyTuple>,
        kwargs: Option<&Bound<'_, PyDict>>,
        countdown: Option<f64>,
        eta: Option<String>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // Convert args and kwargs to JSON
        let args_json = python_to_json(args)?;
        let kwargs_json = kwargs
            .map(|k| python_to_json(k))
            .transpose()?
            .unwrap_or(serde_json::Value::Null);

        let task_name = self.name.clone();
        let queue = self.queue.clone();
        let max_retries = self.max_retries;
        let retry_delay_secs = self.retry_delay_secs;

        future_into_py(py, async move {
            let broker = get_broker().await?;
            let backend = get_backend().await?;

            // Create task ID
            let task_id = TaskId::new();

            // Create task message
            let mut message = TaskMessage::new(task_name.clone(), args_json);
            message.id = task_id.clone();
            if kwargs_json != serde_json::Value::Null {
                message = message.with_kwargs(kwargs_json);
            }

            // Set ETA for delayed or scheduled tasks
            if let Some(delay) = countdown {
                let eta_time = chrono::Utc::now() + chrono::Duration::milliseconds((delay * 1000.0) as i64);
                message = message.with_eta(eta_time);
            } else if let Some(eta_str) = eta {
                if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(&eta_str) {
                    message = message.with_eta(parsed.with_timezone(&chrono::Utc));
                }
            }

            // Set initial state
            backend
                .set_state(&task_id, TaskState::Pending)
                .await
                .map_err(task_error_to_pyerr)?;

            // Send to broker based on type
            match broker.as_ref() {
                #[cfg(feature = "tasks-nats")]
                BrokerInstance::Nats(b) => {
                    if message.eta.is_some() {
                        let delay = Duration::from_millis(
                            (message.eta.unwrap().timestamp_millis() - chrono::Utc::now().timestamp_millis()) as u64
                        );
                        b.publish_delayed(&queue, message, delay)
                            .await
                            .map_err(task_error_to_pyerr)?;
                    } else {
                        b.publish(&queue, message)
                            .await
                            .map_err(task_error_to_pyerr)?;
                    }
                }
                #[cfg(feature = "tasks-pubsub")]
                BrokerInstance::PubSub(b) => {
                    // Pub/Sub doesn't support native delayed publishing
                    // The worker will check the ETA field and delay execution
                    b.publish(&queue, message)
                        .await
                        .map_err(task_error_to_pyerr)?;
                }
                #[allow(unreachable_patterns)]
                _ => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        "Broker type not supported in this build"
                    ));
                }
            }

            Python::with_gil(|py| Ok(PyAsyncResult { task_id }.into_py(py)))
        })
    }
}

/// Handle to track async task execution
///
/// Provides methods to check status and retrieve results.
#[pyclass(name = "AsyncResult")]
pub struct PyAsyncResult {
    task_id: TaskId,
}

#[pymethods]
impl PyAsyncResult {
    /// Get task ID as string
    #[getter]
    fn task_id(&self) -> String {
        self.task_id.to_string()
    }

    /// Check if task is complete
    ///
    /// # Returns
    ///
    /// True if task is in terminal state (SUCCESS or FAILURE)
    ///
    /// # Example
    ///
    /// ```python
    /// if await result.ready():
    ///     value = await result.get()
    /// ```
    fn ready<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let task_id = self.task_id.clone();
        future_into_py(py, async move {
            let backend = get_backend().await?;
            let state = backend
                .get_state(&task_id)
                .await
                .map_err(task_error_to_pyerr)?;
            Ok(state.map(|s| s.is_terminal()).unwrap_or(false))
        })
    }

    /// Get result (waits for completion)
    ///
    /// # Arguments
    ///
    /// * `timeout` - Optional timeout in seconds
    ///
    /// # Returns
    ///
    /// Task result (deserialized from JSON)
    ///
    /// # Raises
    ///
    /// * `RuntimeError` - If task failed or timeout exceeded
    ///
    /// # Example
    ///
    /// ```python
    /// # Wait indefinitely
    /// result = await async_result.get()
    ///
    /// # With timeout
    /// result = await async_result.get(timeout=10.0)
    /// ```
    #[pyo3(signature = (timeout = None))]
    fn get<'py>(&self, py: Python<'py>, timeout: Option<f64>) -> PyResult<Bound<'py, PyAny>> {
        let task_id = self.task_id.clone();
        let timeout_dur = timeout.map(Duration::from_secs_f64);

        future_into_py(py, async move {
            let backend = get_backend().await?;
            let result = backend
                .wait_for_result(
                    &task_id,
                    timeout_dur,
                    Duration::from_millis(100), // poll interval
                )
                .await
                .map_err(task_error_to_pyerr)?;

            match result.state {
                TaskState::Success => {
                    // Convert result to Python
                    Python::with_gil(|py| {
                        json_to_python(py, result.result.unwrap_or(serde_json::Value::Null))
                    })
                }
                TaskState::Failure => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    result
                        .error
                        .unwrap_or_else(|| "Unknown error".to_string()),
                )),
                _ => Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    format!("Unexpected state: {:?}", result.state),
                )),
            }
        })
    }

    /// Get current state without waiting
    ///
    /// # Returns
    ///
    /// State as string: PENDING, RUNNING, SUCCESS, FAILURE, or UNKNOWN
    ///
    /// # Example
    ///
    /// ```python
    /// state = await result.state()
    /// print(state)  # "RUNNING"
    /// ```
    fn state<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let task_id = self.task_id.clone();
        future_into_py(py, async move {
            let backend = get_backend().await?;
            let state = backend
                .get_state(&task_id)
                .await
                .map_err(task_error_to_pyerr)?;
            Ok(state
                .map(|s| format!("{:?}", s).to_uppercase())
                .unwrap_or_else(|| "UNKNOWN".to_string()))
        })
    }

    /// Get full result object (includes state, result, error, timestamps)
    ///
    /// # Returns
    ///
    /// Dictionary with keys: state, result, error, started_at, completed_at
    fn info<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let task_id = self.task_id.clone();
        future_into_py(py, async move {
            let backend = get_backend().await?;
            let result_opt = backend
                .get_result(&task_id)
                .await
                .map_err(task_error_to_pyerr)?;

            let result = result_opt.ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Task result not found")
            })?;

            Python::with_gil(|py| -> PyResult<PyObject> {
                let dict = PyDict::new(py);
                dict.set_item("state", format!("{:?}", result.state).to_uppercase())?;
                dict.set_item(
                    "result",
                    json_to_python(py, result.result.unwrap_or(serde_json::Value::Null))?,
                )?;
                dict.set_item("error", result.error)?;
                dict.set_item(
                    "started_at",
                    result.started_at.map(|t| t.to_rfc3339()),
                )?;
                dict.set_item(
                    "completed_at",
                    result.completed_at.map(|t| t.to_rfc3339()),
                )?;
                dict.set_item("retries", result.retries)?;
                dict.set_item("worker_id", result.worker_id)?;
                Ok(dict.into())
            })
        })
    }
}

/// Python wrapper for TaskSignature
///
/// Represents a task invocation that can be used in workflows.
#[pyclass(name = "TaskSignature")]
#[derive(Clone)]
pub struct PyTaskSignature {
    task_name: String,
    args: serde_json::Value,
    kwargs: serde_json::Value,
    queue: String,
    max_retries: u32,
    retry_delay_secs: f64,
}

#[pymethods]
impl PyTaskSignature {
    /// Get task name
    #[getter]
    fn task_name(&self) -> String {
        self.task_name.clone()
    }

    /// Execute this signature
    fn apply_async<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let sig = self.clone();
        future_into_py(py, async move {
            let broker = get_broker().await?;
            let backend = get_backend().await?;

            let task_id = TaskId::new();
            let mut message = TaskMessage::new(sig.task_name.clone(), sig.args.clone());
            message.id = task_id.clone();
            if sig.kwargs != serde_json::Value::Null {
                message = message.with_kwargs(sig.kwargs.clone());
            }

            backend
                .set_state(&task_id, TaskState::Pending)
                .await
                .map_err(task_error_to_pyerr)?;

            match broker.as_ref() {
                #[cfg(feature = "tasks-nats")]
                BrokerInstance::Nats(b) => {
                    b.publish(&sig.queue, message)
                        .await
                        .map_err(task_error_to_pyerr)?;
                }
                #[cfg(feature = "tasks-pubsub")]
                BrokerInstance::PubSub(b) => {
                    b.publish(&sig.queue, message)
                        .await
                        .map_err(task_error_to_pyerr)?;
                }
                #[allow(unreachable_patterns)]
                _ => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        "Broker type not supported in this build"
                    ));
                }
            }

            Python::with_gil(|py| Ok(PyAsyncResult { task_id }.into_py(py)))
        })
    }
}

impl PyTaskSignature {
    /// Convert to Rust TaskSignature
    fn to_rust_signature(&self) -> TaskSignature {
        let mut sig = TaskSignature::new(self.task_name.clone(), self.args.clone());
        if self.kwargs != serde_json::Value::Null {
            sig = sig.with_kwargs(self.kwargs.clone());
        }
        // Set queue in options
        sig.with_options(TaskOptions {
            queue: Some(self.queue.clone()),
            ..Default::default()
        })
    }
}

/// Python wrapper for Chain workflow
///
/// Executes tasks sequentially, passing output of each task as input to the next.
#[pyclass(name = "Chain")]
pub struct PyChain {
    tasks: Vec<PyTaskSignature>,
}

#[pymethods]
impl PyChain {
    #[new]
    fn new(tasks: Vec<PyRef<PyTaskSignature>>) -> Self {
        Self {
            tasks: tasks.iter().map(|t| (*t).clone()).collect(),
        }
    }

    /// Execute the chain
    ///
    /// # Returns
    ///
    /// AsyncResult for the final task in the chain
    fn apply_async<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let tasks = self.tasks.clone();
        future_into_py(py, async move {
            let broker = get_broker().await?;

            // Convert to Rust signatures
            let signatures: Vec<TaskSignature> = tasks
                .into_iter()
                .map(|t| t.to_rust_signature())
                .collect();

            let chain = Chain::new(signatures);
            let result = match broker.as_ref() {
                #[cfg(feature = "tasks-nats")]
                BrokerInstance::Nats(b) => {
                    chain.apply_async(b.as_ref())
                        .await
                        .map_err(task_error_to_pyerr)?
                }
                #[cfg(feature = "tasks-pubsub")]
                BrokerInstance::PubSub(b) => {
                    chain.apply_async(b.as_ref())
                        .await
                        .map_err(task_error_to_pyerr)?
                }
                #[allow(unreachable_patterns)]
                _ => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        "Broker type not supported in this build"
                    ));
                }
            };

            Python::with_gil(|py| {
                Ok(PyAsyncResult {
                    task_id: result.last_task_id,
                }
                .into_py(py))
            })
        })
    }

    /// Get number of tasks in chain
    fn __len__(&self) -> usize {
        self.tasks.len()
    }
}

/// Python wrapper for Group workflow
///
/// Executes tasks in parallel.
#[pyclass(name = "Group")]
pub struct PyGroup {
    tasks: Vec<PyTaskSignature>,
}

#[pymethods]
impl PyGroup {
    #[new]
    fn new(tasks: Vec<PyRef<PyTaskSignature>>) -> Self {
        Self {
            tasks: tasks.iter().map(|t| (*t).clone()).collect(),
        }
    }

    /// Execute the group
    ///
    /// # Returns
    ///
    /// GroupResult containing AsyncResult for each task
    fn apply_async<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let tasks = self.tasks.clone();
        future_into_py(py, async move {
            let broker = get_broker().await?;

            // Convert to Rust signatures
            let signatures: Vec<TaskSignature> = tasks
                .into_iter()
                .map(|t| t.to_rust_signature())
                .collect();

            let group = Group::new(signatures);
            let result = match broker.as_ref() {
                #[cfg(feature = "tasks-nats")]
                BrokerInstance::Nats(b) => {
                    group.apply_async(b.as_ref())
                        .await
                        .map_err(task_error_to_pyerr)?
                }
                #[cfg(feature = "tasks-pubsub")]
                BrokerInstance::PubSub(b) => {
                    group.apply_async(b.as_ref())
                        .await
                        .map_err(task_error_to_pyerr)?
                }
                #[allow(unreachable_patterns)]
                _ => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        "Broker type not supported in this build"
                    ));
                }
            };

            Python::with_gil(|py| Ok(PyGroupResult { inner: result }.into_py(py)))
        })
    }

    /// Get number of tasks in group
    fn __len__(&self) -> usize {
        self.tasks.len()
    }
}

/// Result handle for a Group workflow
#[pyclass(name = "GroupResult")]
pub struct PyGroupResult {
    inner: GroupResult,
}

#[pymethods]
impl PyGroupResult {
    /// Get list of task IDs
    #[getter]
    fn task_ids(&self) -> Vec<String> {
        self.inner.task_ids.iter().map(|id| id.to_string()).collect()
    }

    /// Get results for all tasks (waits for all to complete)
    #[pyo3(signature = (timeout = None))]
    fn get<'py>(&self, py: Python<'py>, timeout: Option<f64>) -> PyResult<Bound<'py, PyAny>> {
        let task_ids = self.inner.task_ids.clone();
        let timeout_dur = timeout.map(Duration::from_secs_f64);

        future_into_py(py, async move {
            let backend = get_backend().await?;
            let mut results = Vec::new();

            for task_id in task_ids {
                let result = backend
                    .wait_for_result(&task_id, timeout_dur, Duration::from_millis(100))
                    .await
                    .map_err(task_error_to_pyerr)?;

                match result.state {
                    TaskState::Success => {
                        results.push(result.result.unwrap_or(serde_json::Value::Null));
                    }
                    TaskState::Failure => {
                        return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                            result
                                .error
                                .unwrap_or_else(|| "Unknown error".to_string()),
                        ));
                    }
                    _ => {
                        return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                            format!("Unexpected state: {:?}", result.state),
                        ));
                    }
                }
            }

            // Convert to Python list
            let py_results: Result<Vec<PyObject>, PyErr> = Python::with_gil(|py| {
                results.into_iter()
                    .map(|val| json_to_python(py, val))
                    .collect()
            });

            py_results
        })
    }
}

/// Python wrapper for Chord workflow
///
/// Executes tasks in parallel, then executes a callback with the results.
#[pyclass(name = "Chord")]
pub struct PyChord {
    header: Vec<PyTaskSignature>,
    callback: PyTaskSignature,
}

#[pymethods]
impl PyChord {
    #[new]
    fn new(header: Vec<PyRef<PyTaskSignature>>, callback: PyRef<PyTaskSignature>) -> Self {
        Self {
            header: header.iter().map(|t| (*t).clone()).collect(),
            callback: (*callback).clone(),
        }
    }

    /// Execute the chord
    ///
    /// # Returns
    ///
    /// AsyncResult for the callback task
    fn apply_async<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        let header = self.header.clone();
        let callback = self.callback.clone();

        future_into_py(py, async move {
            let broker = get_broker().await?;
            let backend = get_backend().await?;

            // Convert to Rust signatures
            let header_sigs: Vec<TaskSignature> = header
                .into_iter()
                .map(|t| t.to_rust_signature())
                .collect();
            let callback_sig = callback.to_rust_signature();

            // Create group for header
            let header_group = Group::new(header_sigs);

            // Create and execute chord
            let chord = Chord::new(header_group, callback_sig);
            let result = match broker.as_ref() {
                #[cfg(feature = "tasks-nats")]
                BrokerInstance::Nats(b) => {
                    chord.apply_async(b.as_ref(), backend.as_ref())
                        .await
                        .map_err(task_error_to_pyerr)?
                }
                #[cfg(feature = "tasks-pubsub")]
                BrokerInstance::PubSub(b) => {
                    chord.apply_async(b.as_ref(), backend.as_ref())
                        .await
                        .map_err(task_error_to_pyerr)?
                }
                #[allow(unreachable_patterns)]
                _ => {
                    return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                        "Broker type not supported in this build"
                    ));
                }
            };

            Python::with_gil(|py| {
                Ok(PyAsyncResult {
                    task_id: result.callback_task_id,
                }
                .into_py(py))
            })
        })
    }
}

/// Create a task (helper for Python decorator)
///
/// # Arguments
///
/// * `name` - Task name
/// * `queue` - Queue name (default: "default")
/// * `max_retries` - Maximum retry attempts (default: 3)
/// * `retry_delay_secs` - Delay between retries in seconds (default: 1.0)
///
/// # Example
///
/// ```python
/// # Used via Python wrapper
/// task = create_task(name="add", queue="math", max_retries=5)
/// ```
#[pyfunction]
#[pyo3(signature = (name, queue = "default".to_string(), max_retries = 3, retry_delay_secs = 1.0))]
fn create_task(
    name: String,
    queue: String,
    max_retries: u32,
    retry_delay_secs: f64,
) -> PyTask {
    PyTask::new(name, queue, max_retries, retry_delay_secs)
}

// Helper functions

/// Get broker from global state
async fn get_broker() -> PyResult<Arc<BrokerInstance>> {
    BROKER
        .read()
        .await
        .clone()
        .ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Task system not initialized. Call init() first.",
            )
        })
}

/// Get backend from global state
async fn get_backend() -> PyResult<Arc<RedisBackend>> {
    BACKEND
        .read()
        .await
        .clone()
        .ok_or_else(|| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Task system not initialized. Call init() first.",
            )
        })
}

/// Convert Python object to JSON using pythonize
fn python_to_json(obj: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    pythonize::depythonize(obj)
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}

/// Convert JSON to Python object using pythonize
fn json_to_python(py: Python<'_>, value: serde_json::Value) -> PyResult<PyObject> {
    pythonize::pythonize(py, &value)
        .map(|b| b.into())
        .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
}

/// Convert TaskError to PyErr
fn task_error_to_pyerr(e: TaskError) -> PyErr {
    match e {
        TaskError::Timeout(_) => {
            PyErr::new::<pyo3::exceptions::PyTimeoutError, _>(e.to_string())
        }
        TaskError::Serialization(_) | TaskError::Deserialization(_) => {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string())
        }
        TaskError::Broker(_) | TaskError::Backend(_) | TaskError::NotConnected => {
            PyErr::new::<pyo3::exceptions::PyConnectionError, _>(e.to_string())
        }
        _ => PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()),
    }
}

/// Register the tasks module
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyTask>()?;
    m.add_class::<PyAsyncResult>()?;
    m.add_class::<PyTaskSignature>()?;
    m.add_class::<PyChain>()?;
    m.add_class::<PyGroup>()?;
    m.add_class::<PyGroupResult>()?;
    m.add_class::<PyChord>()?;
    m.add_function(wrap_pyfunction!(init, m)?)?;
    m.add_function(wrap_pyfunction!(create_task, m)?)?;
    Ok(())
}
