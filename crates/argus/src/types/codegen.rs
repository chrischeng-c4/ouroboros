//! Code generation (Sprint 3 - Track 2)
//!
//! Provides type-aware code generation:
//! - Docstring generation from types
//! - Test stub generation
//! - Type stub (pyi) generation
//! - Implementation from interface

use std::path::PathBuf;

use super::deep_inference::TypeContext;

// ============================================================================
// Code Generation Request
// ============================================================================

/// A request for code generation.
#[derive(Debug, Clone)]
pub struct CodeGenRequest {
    /// Type of generation
    pub kind: CodeGenKind,
    /// Source file
    pub file: PathBuf,
    /// Target symbol (function, class, etc.)
    pub symbol: String,
    /// Generation options
    pub options: CodeGenOptions,
}

/// Type of code generation.
#[derive(Debug, Clone)]
pub enum CodeGenKind {
    /// Generate docstring
    Docstring { style: DocstringStyle },
    /// Generate test stubs
    TestStub { framework: TestFramework },
    /// Generate type stub (.pyi)
    TypeStub,
    /// Generate implementation from protocol/ABC
    Implementation { protocol: String },
    /// Generate constructor (__init__)
    Constructor,
    /// Generate property accessors
    Properties { fields: Vec<String> },
}

/// Docstring style.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocstringStyle {
    /// Google style
    Google,
    /// NumPy style
    NumPy,
    /// Sphinx style
    Sphinx,
    /// reStructuredText
    RST,
}

/// Test framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestFramework {
    /// pytest
    Pytest,
    /// unittest
    Unittest,
    /// doctest
    Doctest,
}

/// Options for code generation.
#[derive(Debug, Clone, Default)]
pub struct CodeGenOptions {
    /// Include type annotations
    pub include_types: bool,
    /// Include examples in docstrings
    pub include_examples: bool,
    /// Generate async versions
    pub async_support: bool,
    /// Indentation (spaces)
    pub indent: usize,
}

// ============================================================================
// Code Generation Result
// ============================================================================

/// Result of code generation.
#[derive(Debug, Clone)]
pub struct CodeGenResult {
    /// Generated code
    pub code: String,
    /// File to insert into
    pub target_file: PathBuf,
    /// Position to insert (line number)
    pub insert_line: usize,
    /// Imports needed
    pub imports: Vec<String>,
}

impl CodeGenResult {
    /// Create a new result.
    pub fn new(code: impl Into<String>, target_file: PathBuf) -> Self {
        Self {
            code: code.into(),
            target_file,
            insert_line: 0,
            imports: Vec::new(),
        }
    }

    /// Set insertion line.
    pub fn at_line(mut self, line: usize) -> Self {
        self.insert_line = line;
        self
    }

    /// Add required import.
    pub fn with_import(mut self, import: impl Into<String>) -> Self {
        self.imports.push(import.into());
        self
    }
}

// ============================================================================
// Code Generator
// ============================================================================

/// Code generator with type awareness.
pub struct CodeGenerator {
    /// Type context for type information
    type_context: TypeContext,
    /// Default options
    default_options: CodeGenOptions,
}

impl CodeGenerator {
    /// Create a new code generator.
    pub fn new() -> Self {
        Self {
            type_context: TypeContext::new(),
            default_options: CodeGenOptions {
                include_types: true,
                include_examples: true,
                async_support: false,
                indent: 4,
            },
        }
    }

    /// Create with type context.
    pub fn with_context(type_context: TypeContext) -> Self {
        Self {
            type_context,
            default_options: CodeGenOptions::default(),
        }
    }

    /// Generate code based on request.
    pub fn generate(&self, request: &CodeGenRequest) -> CodeGenResult {
        match &request.kind {
            CodeGenKind::Docstring { style } => {
                self.generate_docstring(request, *style)
            }
            CodeGenKind::TestStub { framework } => {
                self.generate_test_stub(request, *framework)
            }
            CodeGenKind::TypeStub => {
                self.generate_type_stub(request)
            }
            CodeGenKind::Implementation { protocol } => {
                self.generate_implementation(request, protocol)
            }
            CodeGenKind::Constructor => {
                self.generate_constructor(request)
            }
            CodeGenKind::Properties { fields } => {
                self.generate_properties(request, fields)
            }
        }
    }

    /// Generate docstring for a function/class.
    fn generate_docstring(&self, request: &CodeGenRequest, style: DocstringStyle) -> CodeGenResult {
        let indent = " ".repeat(request.options.indent.max(self.default_options.indent));

        let docstring = match style {
            DocstringStyle::Google => self.google_docstring(&request.symbol, &indent),
            DocstringStyle::NumPy => self.numpy_docstring(&request.symbol, &indent),
            DocstringStyle::Sphinx => self.sphinx_docstring(&request.symbol, &indent),
            DocstringStyle::RST => self.rst_docstring(&request.symbol, &indent),
        };

        CodeGenResult::new(docstring, request.file.clone())
    }

    fn google_docstring(&self, symbol: &str, indent: &str) -> String {
        format!(
            r#"{indent}"""Short description of {symbol}.

{indent}Longer description if needed.

{indent}Args:
{indent}    param1: Description of param1.
{indent}    param2: Description of param2.

{indent}Returns:
{indent}    Description of return value.

{indent}Raises:
{indent}    ValueError: If invalid input.
{indent}""""#,
            indent = indent,
            symbol = symbol
        )
    }

    fn numpy_docstring(&self, symbol: &str, indent: &str) -> String {
        format!(
            r#"{indent}"""
{indent}Short description of {symbol}.

{indent}Parameters
{indent}----------
{indent}param1 : type
{indent}    Description of param1.
{indent}param2 : type
{indent}    Description of param2.

{indent}Returns
{indent}-------
{indent}type
{indent}    Description of return value.
{indent}""""#,
            indent = indent,
            symbol = symbol
        )
    }

    fn sphinx_docstring(&self, symbol: &str, indent: &str) -> String {
        format!(
            r#"{indent}"""Short description of {symbol}.

{indent}:param param1: Description of param1.
{indent}:type param1: type
{indent}:param param2: Description of param2.
{indent}:type param2: type
{indent}:returns: Description of return value.
{indent}:rtype: type
{indent}:raises ValueError: If invalid input.
{indent}""""#,
            indent = indent,
            symbol = symbol
        )
    }

    fn rst_docstring(&self, symbol: &str, indent: &str) -> String {
        self.sphinx_docstring(symbol, indent)
    }

    /// Generate test stubs.
    fn generate_test_stub(&self, request: &CodeGenRequest, framework: TestFramework) -> CodeGenResult {
        let code = match framework {
            TestFramework::Pytest => self.pytest_stub(&request.symbol),
            TestFramework::Unittest => self.unittest_stub(&request.symbol),
            TestFramework::Doctest => self.doctest_stub(&request.symbol),
        };

        let mut result = CodeGenResult::new(code, request.file.clone());

        match framework {
            TestFramework::Pytest => {
                result = result.with_import("import pytest");
            }
            TestFramework::Unittest => {
                result = result.with_import("import unittest");
            }
            TestFramework::Doctest => {}
        }

        result
    }

    fn pytest_stub(&self, symbol: &str) -> String {
        format!(
            r#"import pytest


class Test{symbol}:
    """Tests for {symbol}."""

    def test_{symbol_lower}_basic(self):
        """Test basic functionality."""
        # Arrange

        # Act

        # Assert
        assert True

    def test_{symbol_lower}_edge_case(self):
        """Test edge cases."""
        # Arrange

        # Act

        # Assert
        assert True

    @pytest.mark.parametrize("input,expected", [
        ("input1", "expected1"),
        ("input2", "expected2"),
    ])
    def test_{symbol_lower}_parametrized(self, input, expected):
        """Test with various inputs."""
        assert True
"#,
            symbol = symbol,
            symbol_lower = symbol.to_lowercase()
        )
    }

    fn unittest_stub(&self, symbol: &str) -> String {
        format!(
            r#"import unittest


class Test{symbol}(unittest.TestCase):
    """Tests for {symbol}."""

    def setUp(self):
        """Set up test fixtures."""
        pass

    def tearDown(self):
        """Tear down test fixtures."""
        pass

    def test_{symbol_lower}_basic(self):
        """Test basic functionality."""
        self.assertTrue(True)

    def test_{symbol_lower}_edge_case(self):
        """Test edge cases."""
        self.assertTrue(True)


if __name__ == "__main__":
    unittest.main()
"#,
            symbol = symbol,
            symbol_lower = symbol.to_lowercase()
        )
    }

    fn doctest_stub(&self, symbol: &str) -> String {
        format!(
            r#"def {symbol_lower}():
    """
    Description of {symbol}.

    Examples
    --------
    >>> {symbol_lower}()
    expected_output

    >>> {symbol_lower}(arg)
    expected_output
    """
    pass
"#,
            symbol = symbol,
            symbol_lower = symbol.to_lowercase()
        )
    }

    /// Generate type stub (.pyi file).
    fn generate_type_stub(&self, request: &CodeGenRequest) -> CodeGenResult {
        let stub = format!(
            r#"from typing import Any, Optional

def {symbol}(*args: Any, **kwargs: Any) -> Any: ...
"#,
            symbol = request.symbol
        );

        let mut stub_file = request.file.clone();
        stub_file.set_extension("pyi");

        CodeGenResult::new(stub, stub_file)
    }

    /// Generate implementation from protocol.
    fn generate_implementation(&self, request: &CodeGenRequest, protocol: &str) -> CodeGenResult {
        let code = format!(
            r#"class {symbol}({protocol}):
    """Implementation of {protocol}."""

    def __init__(self) -> None:
        """Initialize {symbol}."""
        pass

    # TODO: Implement required methods from {protocol}
"#,
            symbol = request.symbol,
            protocol = protocol
        );

        CodeGenResult::new(code, request.file.clone())
    }

    /// Generate constructor.
    fn generate_constructor(&self, request: &CodeGenRequest) -> CodeGenResult {
        let code = format!(
            r#"    def __init__(self) -> None:
        """Initialize {symbol}."""
        pass
"#,
            symbol = request.symbol
        );

        CodeGenResult::new(code, request.file.clone())
    }

    /// Generate property accessors.
    fn generate_properties(&self, request: &CodeGenRequest, fields: &[String]) -> CodeGenResult {
        let mut code = String::new();

        for field in fields {
            code.push_str(&format!(
                r#"    @property
    def {field}(self) -> Any:
        """Get {field}."""
        return self._{field}

    @{field}.setter
    def {field}(self, value: Any) -> None:
        """Set {field}."""
        self._{field} = value

"#,
                field = field
            ));
        }

        CodeGenResult::new(code, request.file.clone())
            .with_import("from typing import Any")
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docstring_generation() {
        let gen = CodeGenerator::new();
        let request = CodeGenRequest {
            kind: CodeGenKind::Docstring {
                style: DocstringStyle::Google,
            },
            file: PathBuf::from("test.py"),
            symbol: "my_function".to_string(),
            options: CodeGenOptions::default(),
        };

        let result = gen.generate(&request);
        assert!(result.code.contains("my_function"));
        assert!(result.code.contains("Args:"));
    }

    #[test]
    fn test_pytest_generation() {
        let gen = CodeGenerator::new();
        let request = CodeGenRequest {
            kind: CodeGenKind::TestStub {
                framework: TestFramework::Pytest,
            },
            file: PathBuf::from("test.py"),
            symbol: "MyClass".to_string(),
            options: CodeGenOptions::default(),
        };

        let result = gen.generate(&request);
        assert!(result.code.contains("class TestMyClass"));
        assert!(result.imports.contains(&"import pytest".to_string()));
    }

    #[test]
    fn test_type_stub_generation() {
        let gen = CodeGenerator::new();
        let request = CodeGenRequest {
            kind: CodeGenKind::TypeStub,
            file: PathBuf::from("module.py"),
            symbol: "my_func".to_string(),
            options: CodeGenOptions::default(),
        };

        let result = gen.generate(&request);
        assert!(result.target_file.extension().unwrap() == "pyi");
    }
}
