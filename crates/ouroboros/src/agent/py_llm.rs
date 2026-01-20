//! LLM Provider PyO3 bindings

use ouroboros_agent_llm::{
    CompletionRequest, CompletionResponse, LLMProvider, OpenAIProvider, ToolDefinition,
};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use pyo3_async_runtimes::tokio::future_into_py;
use std::sync::Arc;

use super::utils::py_to_json;
use crate::error_handling::sanitize_error_message;

/// Base trait for LLM providers (not exposed to Python directly)
pub trait PyLLMProvider: Send + Sync {
    fn inner(&self) -> Arc<dyn LLMProvider>;
}

/// OpenAI LLM Provider
///
/// Provides access to OpenAI models (GPT-4, GPT-3.5, etc.)
///
/// Example:
///     >>> from ouroboros.agent import OpenAI
///     >>> llm = OpenAI(api_key="sk-...")
///     >>> response = await llm.complete(messages=[{"role": "user", "content": "Hello!"}])
#[pyclass(name = "OpenAI")]
pub struct PyOpenAI {
    inner: Arc<OpenAIProvider>,
}

#[pymethods]
impl PyOpenAI {
    /// Create a new OpenAI provider
    ///
    /// Args:
    ///     api_key: OpenAI API key
    ///     model: Default model to use (default: "gpt-4")
    #[new]
    #[pyo3(signature = (api_key, model = "gpt-4".to_string()))]
    fn new(api_key: String, model: String) -> PyResult<Self> {
        let provider = OpenAIProvider::new(api_key).with_default_model(model);

        Ok(Self {
            inner: Arc::new(provider),
        })
    }

    /// Get provider name
    #[getter]
    fn provider_name(&self) -> String {
        self.inner.provider_name().to_string()
    }

    /// Get supported models
    #[getter]
    fn supported_models(&self) -> Vec<String> {
        self.inner.supported_models()
    }

    /// Generate a completion
    ///
    /// Args:
    ///     messages: List of message dictionaries with 'role' and 'content'
    ///     model: Model to use (optional, uses default if not specified)
    ///     temperature: Sampling temperature (0.0 to 2.0)
    ///     max_tokens: Maximum tokens to generate
    ///     tools: List of available tools (optional)
    ///
    /// Returns:
    ///     Dictionary with response content, tool_calls, usage, etc.
    #[pyo3(signature = (messages, model = None, temperature = None, max_tokens = None, tools = None))]
    fn complete<'py>(
        &self,
        py: Python<'py>,
        messages: Vec<Bound<'_, PyDict>>,
        model: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
        tools: Option<Vec<Bound<'_, PyDict>>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let provider = self.inner.clone();

        // Convert Python messages to Rust Message types
        let rust_messages: Vec<ouroboros_agent_core::Message> = messages
            .iter()
            .map(|msg_dict| {
                let role_str: String = msg_dict.get_item("role")?.unwrap().extract()?;
                let content: String = msg_dict.get_item("content")?.unwrap().extract()?;

                let role = match role_str.as_str() {
                    "system" => ouroboros_agent_core::Role::System,
                    "user" => ouroboros_agent_core::Role::User,
                    "assistant" => ouroboros_agent_core::Role::Assistant,
                    "tool" => ouroboros_agent_core::Role::Tool,
                    _ => {
                        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                            "Invalid role: {}",
                            role_str
                        )))
                    }
                };

                let mut msg = match role {
                    ouroboros_agent_core::Role::System => {
                        ouroboros_agent_core::Message::system(content)
                    }
                    ouroboros_agent_core::Role::User => {
                        ouroboros_agent_core::Message::user(content)
                    }
                    ouroboros_agent_core::Role::Assistant => {
                        ouroboros_agent_core::Message::assistant(content)
                    }
                    ouroboros_agent_core::Role::Tool => {
                        let tool_call_id: String = msg_dict
                            .get_item("tool_call_id")?
                            .ok_or_else(|| {
                                PyErr::new::<pyo3::exceptions::PyValueError, _>(
                                    "Tool message missing tool_call_id",
                                )
                            })?
                            .extract()?;
                        ouroboros_agent_core::Message::tool(tool_call_id, content)
                    }
                };

                // Add optional name
                if let Some(name) = msg_dict.get_item("name")? {
                    let name_str: String = name.extract()?;
                    msg = msg.with_name(name_str);
                }

                Ok(msg)
            })
            .collect::<PyResult<Vec<_>>>()?;

        // Build request
        let model_to_use = model.unwrap_or_else(|| "gpt-4".to_string());
        let mut request = CompletionRequest::new(rust_messages, model_to_use);

        if let Some(temp) = temperature {
            request = request.with_temperature(temp);
        }

        if let Some(max_tok) = max_tokens {
            request = request.with_max_tokens(max_tok);
        }

        // Convert tools if provided
        if let Some(tools_list) = tools {
            let rust_tools: Vec<ToolDefinition> = tools_list
                .iter()
                .map(|tool_dict| {
                    let name: String = tool_dict.get_item("name")?.unwrap().extract()?;
                    let description: String =
                        tool_dict.get_item("description")?.unwrap().extract()?;
                    let parameters = tool_dict.get_item("parameters")?.unwrap();
                    let parameters_json = py_to_json(parameters.as_any())?;

                    Ok(ToolDefinition {
                        name,
                        description,
                        parameters: parameters_json,
                    })
                })
                .collect::<PyResult<Vec<_>>>()?;

            request = request.with_tools(rust_tools);
        }

        // Execute completion (GIL released)
        future_into_py(py, async move {
            let response = provider
                .complete(request)
                .await
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                    sanitize_error_message(&e.to_string())
                ))?;

            Python::with_gil(|py| {
                // Convert response to Python dict
                response_to_py_dict(py, response)
            })
        })
    }

    /// Generate a streaming completion
    ///
    /// Args:
    ///     messages: List of message dictionaries
    ///     model: Model to use (optional)
    ///     temperature: Sampling temperature
    ///     max_tokens: Maximum tokens to generate
    ///
    /// Returns:
    ///     AsyncIterator yielding response chunks
    #[pyo3(signature = (messages, model = None, temperature = None, max_tokens = None))]
    fn complete_stream<'py>(
        &self,
        py: Python<'py>,
        messages: Vec<Bound<'_, PyDict>>,
        model: Option<String>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
    ) -> PyResult<Bound<'py, PyAny>> {
        // TODO: Implement streaming support
        // This requires creating an async iterator in Python
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Streaming support coming in Phase 2",
        ))
    }
}

impl PyLLMProvider for PyOpenAI {
    fn inner(&self) -> Arc<dyn LLMProvider> {
        self.inner.clone()
    }
}

/// Convert CompletionResponse to Python dict
fn response_to_py_dict(py: Python, response: CompletionResponse) -> PyResult<PyObject> {
    let dict = PyDict::new(py);

    dict.set_item("content", response.content)?;
    dict.set_item("finish_reason", response.finish_reason)?;
    dict.set_item("model", response.model)?;

    // Usage stats
    let usage_dict = PyDict::new(py);
    usage_dict.set_item("prompt_tokens", response.usage.prompt_tokens)?;
    usage_dict.set_item("completion_tokens", response.usage.completion_tokens)?;
    usage_dict.set_item("total_tokens", response.usage.total_tokens)?;
    dict.set_item("usage", usage_dict)?;

    // Tool calls (if any)
    if let Some(tool_calls) = response.tool_calls {
        let calls_list = pyo3::types::PyList::empty(py);
        for call in tool_calls {
            let call_dict = PyDict::new(py);
            call_dict.set_item("id", call.id)?;
            call_dict.set_item("name", call.name)?;
            call_dict.set_item("arguments", call.arguments.to_string())?;
            calls_list.append(call_dict)?;
        }
        dict.set_item("tool_calls", calls_list)?;
    }

    Ok(dict.into())
}
