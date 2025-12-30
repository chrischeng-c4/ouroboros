//! Security testing framework for data-bridge
//!
//! Provides security-focused testing utilities:
//! - SQL injection detection and prevention testing
//! - Fuzzing framework for input validation
//! - Payload databases for security tests
//!
//! # Example
//! ```rust,ignore
//! use data_bridge_test::security::{PayloadDatabase, Fuzzer, FuzzConfig};
//!
//! // Test SQL injection prevention
//! let payloads = PayloadDatabase::new();
//! for payload in payloads.sql_injection() {
//!     let result = validate_identifier(payload);
//!     assert!(result.is_err(), "Should block: {}", payload);
//! }
//!
//! // Fuzz test an input validator
//! let config = FuzzConfig::default().with_iterations(1000);
//! let fuzzer = Fuzzer::new(config);
//! let result = fuzzer.fuzz(|input| validate_input(input));
//! assert!(result.crashes.is_empty());
//! ```

mod fuzzer;
mod payloads;
mod sql_injection;

pub use fuzzer::{FuzzConfig, FuzzCrash, FuzzResult, Fuzzer, MutationStrategy};
pub use payloads::{PayloadCategory, PayloadDatabase};
pub use sql_injection::{InjectionResult, InjectionTest, SqlInjectionTester};
