//! Wrapper types for PyO3 IntoPyObject.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use ouroboros_postgres::Row;
use super::conversion::extracted_to_py_value;

/// Wrapper for Row to implement IntoPyObject
#[derive(Debug, Clone)]
pub(super) struct RowWrapper {
    pub(super) columns: Vec<(String, ouroboros_postgres::ExtractedValue)>,
}

impl<'py> IntoPyObject<'py> for RowWrapper {
    type Target = PyDict;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let dict = PyDict::new(py);
        for (column, value) in self.columns {
            let py_value = extracted_to_py_value(py, &value)?;
            dict.set_item(column, py_value)?;
        }
        Ok(dict)
    }
}

impl RowWrapper {
    pub(super) fn from_row(row: &Row) -> PyResult<Self> {
        let mut columns = Vec::new();
        for column in row.columns() {
            if let Ok(value) = row.get(column) {
                columns.push((column.to_string(), value.clone()));
            }
        }
        Ok(Self { columns })
    }
}

/// Wrapper for optional Row
#[derive(Debug, Clone)]
pub(super) struct OptionalRowWrapper(pub(super) Option<RowWrapper>);

impl<'py> IntoPyObject<'py> for OptionalRowWrapper {
    type Target = PyAny;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        match self.0 {
            Some(row) => {
                let dict = row.into_pyobject(py)?;
                Ok(dict.into_any())
            }
            None => Ok(py.None().into_bound(py)),
        }
    }
}

/// Wrapper for multiple rows
#[derive(Debug, Clone)]
pub(super) struct RowsWrapper(pub(super) Vec<RowWrapper>);

impl<'py> IntoPyObject<'py> for RowsWrapper {
    type Target = PyList;
    type Output = Bound<'py, Self::Target>;
    type Error = PyErr;

    fn into_pyobject(self, py: Python<'py>) -> Result<Self::Output, Self::Error> {
        let list = PyList::empty(py);
        for row in self.0 {
            let dict = row.into_pyobject(py)?;
            list.append(dict)?;
        }
        Ok(list)
    }
}
