//! Agent PyO3 bindings

use ouroboros_agent_core::{
    Agent as RustAgent, AgentConfig, AgentContext, AgentExecutor, AgentId, Message,
};
use ouroboros_agent_llm::{CompletionRequest, CompletionResponse, LLMProvider};
use ouroboros_agent_tools::ToolExecutor;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_async_runtimes::tokio::future_into_py;
use std::sync::Arc;

use super::py_llm::PyLLMProvider;
use crate::error_handling::sanitize_error_message;

/// LLM-powered Agent
///
/// An agent that uses an LLM to process messages and optionally call tools.
///
/// Example:
///     >>> from ouroboros.agent import Agent, OpenAI
///     >>> llm = OpenAI(api_key="sk-...")
///     >>> agent = Agent(name="assistant", llm=llm, system_prompt="You are a helpful assistant")
///     >>> response = await agent.run("Hello!")
#[pyclass(name = "Agent")]
pub struct PyAgent {
    config: AgentConfig,
    llm: Arc<dyn LLMProvider>,
    executor: AgentExecutor,
    tool_executor: Option<ToolExecutor>,
}

#[pymethods]
impl PyAgent {
    /// Create a new agent
    ///
    /// Args:
    ///     name: Agent name/identifier
    ///     llm: LLM provider to use
    ///     system_prompt: System prompt (optional)
    ///     max_turns: Maximum conversation turns (0 = unlimited)
    ///     tool_timeout: Tool execution timeout in seconds (default: 30)
    ///     tools: List of tool names to use (optional)
    #[new]
    #[pyo3(signature = (name, llm, system_prompt = None, max_turns = 0, tool_timeout = 30, tools = None))]
    fn new(
        name: String,
        llm: PyObject,
        system_prompt: Option<String>,
        max_turns: u32,
        tool_timeout: u64,
        tools: Option<Vec<String>>,
    ) -> PyResult<Self> {
        // Extract LLM provider
        let llm_provider = Python::with_gil(|py| {
            let llm_bound = llm.bind(py);

            // Check if it's OpenAI
            if let Ok(openai) = llm_bound.extract::<PyRef<crate::agent::PyOpenAI>>() {
                Ok(openai.inner())
            } else {
                Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                    "LLM provider must be an instance of OpenAI or another supported provider",
                ))
            }
        })?;

        // Build config
        let mut config = AgentConfig::new(AgentId::new(name));

        if let Some(prompt) = system_prompt {
            config = config.with_system_prompt(prompt);
        }

        config = config.with_max_turns(max_turns).with_tool_timeout(tool_timeout);

        // Create tool executor if tools are provided
        // TODO: Integrate with global registry
        let tool_executor = if tools.is_some() {
            let registry = Arc::new(ouroboros_agent_tools::ToolRegistry::new());
            Some(ToolExecutor::new(registry))
        } else {
            None
        };

        Ok(Self {
            config,
            llm: llm_provider,
            executor: AgentExecutor::new(),
            tool_executor,
        })
    }

    /// Get agent name
    #[getter]
    fn name(&self) -> String {
        self.config.agent_id.as_str().to_string()
    }

    /// Get system prompt
    #[getter]
    fn system_prompt(&self) -> Option<String> {
        self.config.system_prompt.clone()
    }

    /// Run the agent with a text input
    ///
    /// Args:
    ///     input: User input text
    ///     model: LLM model to use (optional, uses provider default)
    ///     temperature: Sampling temperature (optional)
    ///     max_tokens: Maximum tokens to generate (optional)
    ///
    /// Returns:
    ///     Dictionary with response content, tool_calls, usage, etc.
    #[pyo3(signature = (input, model = None, temperature = None, max_tokens = None))]
    fn run<'py>(
        &self,
        py: Python<'py>,
        input: String,
        model: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let llm = self.llm.clone();
        let config = self.config.clone();

        future_into_py(py, async move {
            // Create context
            let mut context = AgentContext::new(config.agent_id.clone());

            // Add system prompt if configured
            if let Some(system_prompt) = &config.system_prompt {
                context.add_message(Message::system(system_prompt.clone()));
            }

            // Add user input
            let user_message = Message::user(input);
            context.add_message(user_message.clone());

            // Build LLM request
            let model_to_use = model.unwrap_or_else(|| "gpt-4o-mini".to_string());
            let mut request = CompletionRequest::new(context.messages.clone(), model_to_use);

            if let Some(temp) = temperature {
                request = request.with_temperature(temp);
            }

            if let Some(max_tok) = max_tokens {
                request = request.with_max_tokens(max_tok);
            }

            // Call LLM
            let response = llm.complete(request).await.map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(sanitize_error_message(
                    &e.to_string(),
                ))
            })?;

            // Convert response to Python
            Python::with_gil(|py| response_to_py_dict(py, &response))
        })
    }

    /// Run the agent with an existing context
    ///
    /// Args:
    ///     context: Python dictionary representing the context
    ///     input: User input text
    ///     model: LLM model to use (optional)
    ///     temperature: Sampling temperature (optional)
    ///     max_tokens: Maximum tokens to generate (optional)
    ///
    /// Returns:
    ///     Dictionary with response content, tool_calls, usage, etc.
    #[pyo3(signature = (_context, input, model = None, temperature = None, max_tokens = None))]
    fn run_with_context<'py>(
        &self,
        py: Python<'py>,
        _context: Bound<'_, PyDict>,
        input: String,
        model: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // TODO: Convert Python context to Rust AgentContext
        // For now, just use run()
        self.run(py, input, model, temperature, max_tokens)
    }

    /// Get agent configuration as dict
    fn get_config(&self, py: Python) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        dict.set_item("agent_id", self.config.agent_id.as_str())?;
        dict.set_item("system_prompt", self.config.system_prompt.clone())?;
        dict.set_item("max_turns", self.config.max_turns)?;
        dict.set_item("tool_timeout_secs", self.config.tool_timeout_secs)?;
        Ok(dict.into())
    }
}

/// Convert CompletionResponse to Python dict
fn response_to_py_dict(py: Python, response: &CompletionResponse) -> PyResult<PyObject> {
    let dict = PyDict::new(py);

    dict.set_item("content", response.content.clone())?;
    dict.set_item("finish_reason", response.finish_reason.clone())?;
    dict.set_item("model", response.model.clone())?;

    // Usage stats
    let usage_dict = PyDict::new(py);
    usage_dict.set_item("prompt_tokens", response.usage.prompt_tokens)?;
    usage_dict.set_item("completion_tokens", response.usage.completion_tokens)?;
    usage_dict.set_item("total_tokens", response.usage.total_tokens)?;
    dict.set_item("usage", usage_dict)?;

    // Tool calls (if any)
    if let Some(tool_calls) = &response.tool_calls {
        let calls_list = pyo3::types::PyList::empty(py);
        for call in tool_calls {
            let call_dict = PyDict::new(py);
            call_dict.set_item("id", call.id.clone())?;
            call_dict.set_item("name", call.name.clone())?;
            call_dict.set_item("arguments", call.arguments.to_string())?;
            calls_list.append(call_dict)?;
        }
        dict.set_item("tool_calls", calls_list)?;
    }

    // Metadata
    if !response.metadata.is_empty() {
        let metadata_dict = PyDict::new(py);
        for (key, value) in &response.metadata {
            metadata_dict.set_item(key, value.to_string())?;
        }
        dict.set_item("metadata", metadata_dict)?;
    }

    Ok(dict.into())
}
