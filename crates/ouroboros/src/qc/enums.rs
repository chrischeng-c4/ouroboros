//! Enum types for the QC module.

use ouroboros_qc::{
    TestStatus, TestType, ReportFormat,
    discovery::FileType,
    fixtures::FixtureScope,
    ProfilePhase,
    HookType,
};
use pyo3::prelude::*;

// =====================
// TestType
// =====================

/// Python TestType enum
#[pyclass(name = "TestType", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyTestType {
    Unit,
    Profile,
    Stress,
    Security,
}

impl From<PyTestType> for TestType {
    fn from(py_type: PyTestType) -> Self {
        match py_type {
            PyTestType::Unit => TestType::Unit,
            PyTestType::Profile => TestType::Profile,
            PyTestType::Stress => TestType::Stress,
            PyTestType::Security => TestType::Security,
        }
    }
}

impl From<TestType> for PyTestType {
    fn from(rust_type: TestType) -> Self {
        match rust_type {
            TestType::Unit => PyTestType::Unit,
            TestType::Profile => PyTestType::Profile,
            TestType::Stress => PyTestType::Stress,
            TestType::Security => PyTestType::Security,
        }
    }
}

#[pymethods]
impl PyTestType {
    fn __str__(&self) -> &'static str {
        match self {
            PyTestType::Unit => "unit",
            PyTestType::Profile => "profile",
            PyTestType::Stress => "stress",
            PyTestType::Security => "security",
        }
    }

    fn __repr__(&self) -> String {
        format!("TestType.{}", self.__str__().to_uppercase())
    }
}

// =====================
// TestStatus
// =====================

/// Python TestStatus enum
#[pyclass(name = "TestStatus", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyTestStatus {
    Passed,
    Failed,
    Skipped,
    Error,
}

impl From<TestStatus> for PyTestStatus {
    fn from(status: TestStatus) -> Self {
        match status {
            TestStatus::Passed => PyTestStatus::Passed,
            TestStatus::Failed => PyTestStatus::Failed,
            TestStatus::Skipped => PyTestStatus::Skipped,
            TestStatus::Error => PyTestStatus::Error,
        }
    }
}

impl From<PyTestStatus> for TestStatus {
    fn from(status: PyTestStatus) -> Self {
        match status {
            PyTestStatus::Passed => TestStatus::Passed,
            PyTestStatus::Failed => TestStatus::Failed,
            PyTestStatus::Skipped => TestStatus::Skipped,
            PyTestStatus::Error => TestStatus::Error,
        }
    }
}

#[pymethods]
impl PyTestStatus {
    fn __str__(&self) -> &'static str {
        match self {
            PyTestStatus::Passed => "PASSED",
            PyTestStatus::Failed => "FAILED",
            PyTestStatus::Skipped => "SKIPPED",
            PyTestStatus::Error => "ERROR",
        }
    }

    fn __repr__(&self) -> String {
        format!("TestStatus.{}", self.__str__())
    }
}

// =====================
// ReportFormat
// =====================

/// Python ReportFormat enum
#[pyclass(name = "ReportFormat", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyReportFormat {
    Markdown,
    Html,
    Json,
    Yaml,
    JUnit,
    Console,
}

impl From<PyReportFormat> for ReportFormat {
    fn from(fmt: PyReportFormat) -> Self {
        match fmt {
            PyReportFormat::Markdown => ReportFormat::Markdown,
            PyReportFormat::Html => ReportFormat::Html,
            PyReportFormat::Json => ReportFormat::Json,
            PyReportFormat::Yaml => ReportFormat::Yaml,
            PyReportFormat::JUnit => ReportFormat::JUnit,
            PyReportFormat::Console => ReportFormat::Console,
        }
    }
}

#[pymethods]
impl PyReportFormat {
    fn __str__(&self) -> &'static str {
        match self {
            PyReportFormat::Markdown => "markdown",
            PyReportFormat::Html => "html",
            PyReportFormat::Json => "json",
            PyReportFormat::Yaml => "yaml",
            PyReportFormat::JUnit => "junit",
            PyReportFormat::Console => "console",
        }
    }

    fn __repr__(&self) -> String {
        format!("ReportFormat.{}", self.__str__().to_uppercase())
    }
}

// =====================
// FileType
// =====================

/// Python FileType enum
#[pyclass(name = "FileType", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyFileType {
    Test,
    Benchmark,
}

impl From<FileType> for PyFileType {
    fn from(file_type: FileType) -> Self {
        match file_type {
            FileType::Test => PyFileType::Test,
            FileType::Benchmark => PyFileType::Benchmark,
        }
    }
}

impl From<PyFileType> for FileType {
    fn from(py_type: PyFileType) -> Self {
        match py_type {
            PyFileType::Test => FileType::Test,
            PyFileType::Benchmark => FileType::Benchmark,
        }
    }
}

#[pymethods]
impl PyFileType {
    fn __str__(&self) -> &'static str {
        match self {
            PyFileType::Test => "test",
            PyFileType::Benchmark => "benchmark",
        }
    }

    fn __repr__(&self) -> String {
        format!("FileType.{}", self.__str__().to_uppercase())
    }
}

// =====================
// ProfilePhase
// =====================

/// Python ProfilePhase enum
#[pyclass(name = "ProfilePhase", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyProfilePhase {
    PythonExtract,
    RustConvert,
    NetworkIO,
    PyO3Boundary,
    Total,
}

impl From<ProfilePhase> for PyProfilePhase {
    fn from(phase: ProfilePhase) -> Self {
        match phase {
            ProfilePhase::PythonExtract => PyProfilePhase::PythonExtract,
            ProfilePhase::RustConvert => PyProfilePhase::RustConvert,
            ProfilePhase::NetworkIO => PyProfilePhase::NetworkIO,
            ProfilePhase::PyO3Boundary => PyProfilePhase::PyO3Boundary,
            ProfilePhase::Total => PyProfilePhase::Total,
        }
    }
}

impl From<PyProfilePhase> for ProfilePhase {
    fn from(phase: PyProfilePhase) -> Self {
        match phase {
            PyProfilePhase::PythonExtract => ProfilePhase::PythonExtract,
            PyProfilePhase::RustConvert => ProfilePhase::RustConvert,
            PyProfilePhase::NetworkIO => ProfilePhase::NetworkIO,
            PyProfilePhase::PyO3Boundary => ProfilePhase::PyO3Boundary,
            PyProfilePhase::Total => ProfilePhase::Total,
        }
    }
}

#[pymethods]
impl PyProfilePhase {
    fn __str__(&self) -> &'static str {
        match self {
            PyProfilePhase::PythonExtract => "PythonExtract",
            PyProfilePhase::RustConvert => "RustConvert",
            PyProfilePhase::NetworkIO => "NetworkIO",
            PyProfilePhase::PyO3Boundary => "PyO3Boundary",
            PyProfilePhase::Total => "Total",
        }
    }

    fn __repr__(&self) -> String {
        format!("ProfilePhase.{}", self.__str__())
    }
}

// =====================
// FixtureScope
// =====================

/// Python FixtureScope enum
#[pyclass(name = "FixtureScope", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyFixtureScope {
    Function,
    Class,
    Module,
    Session,
}

impl From<PyFixtureScope> for FixtureScope {
    fn from(py_scope: PyFixtureScope) -> Self {
        match py_scope {
            PyFixtureScope::Function => FixtureScope::Function,
            PyFixtureScope::Class => FixtureScope::Class,
            PyFixtureScope::Module => FixtureScope::Module,
            PyFixtureScope::Session => FixtureScope::Session,
        }
    }
}

impl From<FixtureScope> for PyFixtureScope {
    fn from(scope: FixtureScope) -> Self {
        match scope {
            FixtureScope::Function => PyFixtureScope::Function,
            FixtureScope::Class => PyFixtureScope::Class,
            FixtureScope::Module => PyFixtureScope::Module,
            FixtureScope::Session => PyFixtureScope::Session,
        }
    }
}

impl std::fmt::Display for PyFixtureScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PyFixtureScope::Function => write!(f, "function"),
            PyFixtureScope::Class => write!(f, "class"),
            PyFixtureScope::Module => write!(f, "module"),
            PyFixtureScope::Session => write!(f, "session"),
        }
    }
}

#[pymethods]
impl PyFixtureScope {
    fn __str__(&self) -> &'static str {
        match self {
            PyFixtureScope::Function => "function",
            PyFixtureScope::Class => "class",
            PyFixtureScope::Module => "module",
            PyFixtureScope::Session => "session",
        }
    }

    fn __repr__(&self) -> String {
        format!("FixtureScope.{}", self.__str__().to_uppercase())
    }

    #[staticmethod]
    fn from_string(s: &str) -> PyResult<Self> {
        s.parse::<FixtureScope>()
            .map(PyFixtureScope::from)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e))
    }
}

// =====================
// HookType
// =====================

/// Python HookType enum
#[pyclass(name = "HookType", eq)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyHookType {
    SetupClass,
    TeardownClass,
    SetupModule,
    TeardownModule,
    SetupMethod,
    TeardownMethod,
}

impl From<PyHookType> for HookType {
    fn from(py_type: PyHookType) -> Self {
        match py_type {
            PyHookType::SetupClass => HookType::SetupClass,
            PyHookType::TeardownClass => HookType::TeardownClass,
            PyHookType::SetupModule => HookType::SetupModule,
            PyHookType::TeardownModule => HookType::TeardownModule,
            PyHookType::SetupMethod => HookType::SetupMethod,
            PyHookType::TeardownMethod => HookType::TeardownMethod,
        }
    }
}

impl From<HookType> for PyHookType {
    fn from(hook_type: HookType) -> Self {
        match hook_type {
            HookType::SetupClass => PyHookType::SetupClass,
            HookType::TeardownClass => PyHookType::TeardownClass,
            HookType::SetupModule => PyHookType::SetupModule,
            HookType::TeardownModule => PyHookType::TeardownModule,
            HookType::SetupMethod => PyHookType::SetupMethod,
            HookType::TeardownMethod => PyHookType::TeardownMethod,
        }
    }
}

#[pymethods]
impl PyHookType {
    fn __str__(&self) -> &'static str {
        match self {
            PyHookType::SetupClass => "setup_class",
            PyHookType::TeardownClass => "teardown_class",
            PyHookType::SetupModule => "setup_module",
            PyHookType::TeardownModule => "teardown_module",
            PyHookType::SetupMethod => "setup_method",
            PyHookType::TeardownMethod => "teardown_method",
        }
    }

    fn __repr__(&self) -> String {
        format!("HookType.{}", self.__str__().to_uppercase())
    }
}
