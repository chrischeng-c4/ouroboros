//! Global security configuration for ouroboros
//!
//! This module provides thread-safe configuration for security features,
//! particularly around ObjectId auto-conversion behavior.
//!
//! # Usage
//! ```python
//! from ouroboros import configure_security, ObjectIdConversionMode
//!
//! # Type-hinted mode (recommended for v1.x)
//! configure_security(objectid_mode=ObjectIdConversionMode.TYPE_HINTED)
//!
//! # Strict mode (v2.0 default)
//! configure_security(objectid_mode=ObjectIdConversionMode.STRICT)
//! ```

use pyo3::prelude::*;
use std::sync::RwLock;

/// ObjectId conversion mode determines when string-to-ObjectId conversion happens
///
/// # Security Implications
/// - **TypeHinted**: Only converts when type hints indicate ObjectId (RECOMMENDED)
/// - **Lenient**: Auto-converts 24-char hex strings (BACKWARD COMPATIBLE, less secure)
/// - **Strict**: Requires explicit wrapper, no auto-conversion (MOST SECURE)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[pyclass(module = "ouroboros")]
pub enum ObjectIdConversionMode {
    /// Only convert strings to ObjectId when type hints indicate it
    ///
    /// This is the recommended mode for v1.x releases. It provides a good balance
    /// between security and usability.
    ///
    /// # Security
    /// Prevents NoSQL injection via all-zeros ObjectId attack while maintaining
    /// compatibility with type-hinted code.
    TypeHinted = 0,

    /// Auto-convert any 24-character hex string to ObjectId
    ///
    /// This is the current v1.0 behavior. It's backward compatible but less secure.
    /// Deprecated in v1.1, will be removed in v2.0.
    ///
    /// # Security Risk
    /// Vulnerable to NoSQL injection via crafted hex strings like "000000000000000000000000"
    Lenient = 1,

    /// Require explicit ObjectId wrapper, no auto-conversion
    ///
    /// This is the most secure mode and will be the default in v2.0.
    ///
    /// # Security
    /// Maximum protection against NoSQL injection, but requires code changes.
    Strict = 2,
}

#[pymethods]
impl ObjectIdConversionMode {
    #[classattr]
    const TYPE_HINTED: Self = Self::TypeHinted;

    #[classattr]
    const LENIENT: Self = Self::Lenient;

    #[classattr]
    const STRICT: Self = Self::Strict;

    fn __repr__(&self) -> &'static str {
        match self {
            Self::TypeHinted => "ObjectIdConversionMode.TYPE_HINTED",
            Self::Lenient => "ObjectIdConversionMode.LENIENT",
            Self::Strict => "ObjectIdConversionMode.STRICT",
        }
    }
}

/// Global security configuration
///
/// # Thread Safety
/// All configuration is protected by RwLock for thread-safe access.
/// Multiple readers can access configuration simultaneously, but writes are exclusive.
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// ObjectId conversion mode
    pub objectid_mode: ObjectIdConversionMode,

    /// Whether to enable query validation (blocking dangerous operators)
    pub validate_queries: bool,

    /// Whether to sanitize error messages in production
    pub sanitize_errors: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        SecurityConfig {
            // v1.0 defaults to Lenient for backward compatibility
            // v1.1 will default to TypeHinted
            // v2.0 will default to Strict
            objectid_mode: ObjectIdConversionMode::Lenient,
            validate_queries: true,
            sanitize_errors: true,
        }
    }
}

/// Global security configuration instance
///
/// Protected by RwLock for thread-safe read/write access.
static GLOBAL_CONFIG: RwLock<SecurityConfig> = RwLock::new(SecurityConfig {
    objectid_mode: ObjectIdConversionMode::Lenient,
    validate_queries: true,
    sanitize_errors: true,
});

/// Gets the current security configuration
///
/// # Thread Safety
/// Uses read lock, allowing concurrent reads from multiple threads.
///
/// # Panics
/// Panics if the lock is poisoned (should never happen in normal operation)
pub fn get_config() -> SecurityConfig {
    GLOBAL_CONFIG
        .read()
        .expect("Security config lock poisoned")
        .clone()
}

/// Sets the security configuration
///
/// # Thread Safety
/// Uses write lock, ensuring exclusive access during configuration update.
///
/// # Panics
/// Panics if the lock is poisoned (should never happen in normal operation)
pub fn set_config(config: SecurityConfig) {
    let mut global = GLOBAL_CONFIG
        .write()
        .expect("Security config lock poisoned");
    *global = config;
}

/// Python binding: Configure security settings
///
/// # Arguments
/// * `objectid_mode` - Optional ObjectId conversion mode
/// * `validate_queries` - Optional query validation flag
/// * `sanitize_errors` - Optional error sanitization flag
///
/// # Examples
/// ```python
/// from ouroboros import configure_security, ObjectIdConversionMode
///
/// # Set strict mode for maximum security
/// configure_security(objectid_mode=ObjectIdConversionMode.STRICT)
///
/// # Disable query validation (not recommended)
/// configure_security(validate_queries=False)
///
/// # Configure multiple settings
/// configure_security(
///     objectid_mode=ObjectIdConversionMode.TYPE_HINTED,
///     validate_queries=True,
///     sanitize_errors=True
/// )
/// ```
#[pyfunction]
#[pyo3(signature = (objectid_mode=None, validate_queries=None, sanitize_errors=None))]
pub fn configure_security(
    objectid_mode: Option<ObjectIdConversionMode>,
    validate_queries: Option<bool>,
    sanitize_errors: Option<bool>,
) -> PyResult<()> {
    let mut config = get_config();

    if let Some(mode) = objectid_mode {
        config.objectid_mode = mode;

        // Emit deprecation warning for Lenient mode
        if mode == ObjectIdConversionMode::Lenient {
            eprintln!(
                "WARNING: ObjectIdConversionMode.LENIENT is deprecated and will be removed in v2.0. \
                Please migrate to ObjectIdConversionMode.TYPE_HINTED for better security."
            );
        }
    }

    if let Some(validate) = validate_queries {
        config.validate_queries = validate;
        if !validate {
            eprintln!(
                "WARNING: Query validation is disabled. This may expose your application \
                to NoSQL injection attacks. Only disable for testing purposes."
            );
        }
    }

    if let Some(sanitize) = sanitize_errors {
        config.sanitize_errors = sanitize;
    }

    set_config(config);
    Ok(())
}

/// Python binding: Get current security configuration
///
/// # Returns
/// Dictionary with current configuration values
///
/// # Examples
/// ```python
/// from ouroboros import get_security_config
///
/// config = get_security_config()
/// print(config)  # {'objectid_mode': 'TYPE_HINTED', 'validate_queries': True, ...}
/// ```
#[pyfunction]
pub fn get_security_config() -> PyResult<(ObjectIdConversionMode, bool, bool)> {
    let config = get_config();
    Ok((config.objectid_mode, config.validate_queries, config.sanitize_errors))
}

/// Register config module functions with Python
pub fn register_functions(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ObjectIdConversionMode>()?;
    m.add_function(wrap_pyfunction!(configure_security, m)?)?;
    m.add_function(wrap_pyfunction!(get_security_config, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SecurityConfig::default();
        assert_eq!(config.objectid_mode, ObjectIdConversionMode::Lenient);
        assert!(config.validate_queries);
        assert!(config.sanitize_errors);
    }

    #[test]
    fn test_get_set_config() {
        // Save original config
        let original = get_config();

        // Set new config
        let new_config = SecurityConfig {
            objectid_mode: ObjectIdConversionMode::Strict,
            validate_queries: false,
            sanitize_errors: false,
        };
        set_config(new_config.clone());

        // Verify it was set
        let retrieved = get_config();
        assert_eq!(retrieved.objectid_mode, ObjectIdConversionMode::Strict);
        assert!(!retrieved.validate_queries);
        assert!(!retrieved.sanitize_errors);

        // Restore original config
        set_config(original);
    }

    #[test]
    fn test_objectid_conversion_modes() {
        assert_eq!(ObjectIdConversionMode::TypeHinted as i32, 0);
        assert_eq!(ObjectIdConversionMode::Lenient as i32, 1);
        assert_eq!(ObjectIdConversionMode::Strict as i32, 2);
    }

    #[test]
    fn test_thread_safety() {
        use std::thread;
        

        let original = get_config();

        // Spawn multiple threads that read config
        let handles: Vec<_> = (0..10)
            .map(|_| {
                thread::spawn(|| {
                    let config = get_config();
                    // Just verify we can access the config field
                    let _ = config.validate_queries;
                })
            })
            .collect();

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Restore original
        set_config(original);
    }
}
