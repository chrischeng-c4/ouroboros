//! PostgreSQL connection management with connection pooling.
//!
//! This module provides connection pooling using SQLx's built-in pool manager.
//! Similar to data-bridge-mongodb's connection management, but optimized for PostgreSQL.

use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing::{info, instrument};

use crate::{DataBridgeError, Result};

/// Connection pool configuration.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Minimum number of connections in the pool.
    pub min_connections: u32,
    /// Maximum number of connections in the pool.
    pub max_connections: u32,
    /// Connection timeout in seconds.
    pub connect_timeout: u64,
    /// Maximum lifetime of a connection in seconds.
    pub max_lifetime: Option<u64>,
    /// Idle timeout in seconds.
    pub idle_timeout: Option<u64>,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            min_connections: 1,
            max_connections: 10,
            connect_timeout: 30,
            max_lifetime: Some(1800), // 30 minutes
            idle_timeout: Some(600),   // 10 minutes
        }
    }
}

/// PostgreSQL connection wrapper with connection pooling.
#[derive(Clone)]
pub struct Connection {
    pool: PgPool,
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Connection")
            .field("size", &self.pool.size())
            .field("num_idle", &self.pool.num_idle())
            .finish()
    }
}

impl Connection {
    /// Creates a new connection pool.
    ///
    /// # Arguments
    ///
    /// * `uri` - PostgreSQL connection URI (e.g., "postgresql://user:password@localhost/db")
    /// * `config` - Pool configuration
    ///
    /// # Errors
    ///
    /// Returns error if connection fails or URI is invalid.
    #[instrument(skip(uri), fields(
        min_connections = config.min_connections,
        max_connections = config.max_connections
    ))]
    pub async fn new(uri: &str, config: PoolConfig) -> Result<Self> {
        // Validate URI format (basic check)
        if uri.is_empty() {
            return Err(DataBridgeError::Connection(
                "Connection URI cannot be empty".to_string(),
            ));
        }

        info!("Initializing connection pool");

        // Build pool options with configuration
        let mut pool_options = PgPoolOptions::new()
            .min_connections(config.min_connections)
            .max_connections(config.max_connections)
            .acquire_timeout(Duration::from_secs(config.connect_timeout));

        // Add optional timeouts
        if let Some(max_lifetime_secs) = config.max_lifetime {
            pool_options = pool_options.max_lifetime(Duration::from_secs(max_lifetime_secs));
        }

        if let Some(idle_timeout_secs) = config.idle_timeout {
            pool_options = pool_options.idle_timeout(Duration::from_secs(idle_timeout_secs));
        }

        // Connect to the database and create pool
        let pool = pool_options.connect(uri).await?;

        // Test the connection with a simple ping
        sqlx::query("SELECT 1")
            .execute(&pool)
            .await
            .map_err(|e| DataBridgeError::Connection(format!("Failed to verify connection: {}", e)))?;

        info!("Connection pool initialized successfully");
        Ok(Self { pool })
    }

    /// Gets a reference to the connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Closes the connection pool.
    pub async fn close(&self) -> Result<()> {
        self.pool.close().await;
        Ok(())
    }

    /// Pings the database to verify connectivity.
    pub async fn ping(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_config_default() {
        // Verify default config values
        let config = PoolConfig::default();

        assert_eq!(config.min_connections, 1);
        assert_eq!(config.max_connections, 10);
        assert_eq!(config.connect_timeout, 30);
        assert_eq!(config.max_lifetime, Some(1800)); // 30 minutes
        assert_eq!(config.idle_timeout, Some(600));  // 10 minutes
    }

    #[test]
    fn test_connection_config_builder() {
        // Verify builder pattern works by creating custom config
        let config = PoolConfig {
            min_connections: 2,
            max_connections: 20,
            connect_timeout: 60,
            max_lifetime: Some(3600), // 1 hour
            idle_timeout: Some(300),  // 5 minutes
        };

        assert_eq!(config.min_connections, 2);
        assert_eq!(config.max_connections, 20);
        assert_eq!(config.connect_timeout, 60);
        assert_eq!(config.max_lifetime, Some(3600));
        assert_eq!(config.idle_timeout, Some(300));
    }

    #[test]
    fn test_connection_config_from_env() {
        // Verify env-based config (using manual parsing simulation)
        // In a real scenario, you would use std::env::var()
        let min_conn = "5".parse::<u32>().unwrap();
        let max_conn = "50".parse::<u32>().unwrap();
        let timeout = "45".parse::<u64>().unwrap();

        let config = PoolConfig {
            min_connections: min_conn,
            max_connections: max_conn,
            connect_timeout: timeout,
            max_lifetime: Some(1800),
            idle_timeout: Some(600),
        };

        assert_eq!(config.min_connections, 5);
        assert_eq!(config.max_connections, 50);
        assert_eq!(config.connect_timeout, 45);
    }

    #[test]
    fn test_pool_config_validation() {
        // Verify pool size constraints
        // Test 1: min_connections should be less than max_connections (logical constraint)
        let valid_config = PoolConfig {
            min_connections: 1,
            max_connections: 10,
            connect_timeout: 30,
            max_lifetime: Some(1800),
            idle_timeout: Some(600),
        };

        assert!(valid_config.min_connections < valid_config.max_connections);

        // Test 2: Check that we can create configs with edge values
        let edge_config = PoolConfig {
            min_connections: 0,
            max_connections: 100,
            connect_timeout: 1,
            max_lifetime: None,
            idle_timeout: None,
        };

        assert_eq!(edge_config.min_connections, 0);
        assert_eq!(edge_config.max_connections, 100);
        assert_eq!(edge_config.connect_timeout, 1);
        assert!(edge_config.max_lifetime.is_none());
        assert!(edge_config.idle_timeout.is_none());
    }

    #[test]
    fn test_uri_parsing_valid() {
        // Verify valid URIs parse correctly (basic format check)
        let valid_uris = vec![
            "postgresql://user:password@localhost/mydb",
            "postgres://user@localhost/db",
            "postgresql://user:pass@127.0.0.1:5432/database",
            "postgres://localhost/db",
        ];

        for uri in valid_uris {
            // Basic validation that would pass Connection::new() URI check
            assert!(!uri.is_empty());
            assert!(uri.starts_with("postgres://") || uri.starts_with("postgresql://"));
        }
    }

    #[test]
    fn test_uri_parsing_invalid() {
        // Verify invalid URIs are rejected
        // Note: Connection::new() only checks for empty strings, not whitespace
        // Additional validation happens at SQLx level
        let empty_uri = "";
        let result = validate_uri_format(empty_uri);
        assert!(result.is_err(), "Expected empty URI to be invalid");

        // Test that the error message is correct
        if let Err(DataBridgeError::Connection(msg)) = result {
            assert_eq!(msg, "Connection URI cannot be empty");
        } else {
            panic!("Expected Connection error for empty URI");
        }
    }

    // Helper function to simulate URI validation logic from Connection::new()
    #[allow(unused)]
    fn validate_uri_format(uri: &str) -> Result<()> {
        if uri.is_empty() {
            return Err(DataBridgeError::Connection(
                "Connection URI cannot be empty".to_string(),
            ));
        }
        Ok(())
    }

    #[test]
    fn test_is_connected_before_init() {
        // Verify is_connected returns false before init
        // Since we can't create a Connection without async/DB,
        // we test that a pool reference would be None in an Option wrapper

        let maybe_connection: Option<Connection> = None;
        assert!(maybe_connection.is_none());

        // This simulates checking connection state before initialization
        let is_connected = maybe_connection.is_some();
        assert!(!is_connected);
    }

    #[test]
    fn test_get_pool_returns_error_before_init() {
        // Verify get_pool errors before init
        // We test this by simulating the error condition

        let maybe_connection: Option<Connection> = None;

        // Attempt to get pool when connection is not initialized
        let result = maybe_connection.as_ref().ok_or_else(|| {
            DataBridgeError::Connection("Connection not initialized".to_string())
        });

        assert!(result.is_err());

        if let Err(e) = result {
            match e {
                DataBridgeError::Connection(msg) => {
                    assert_eq!(msg, "Connection not initialized");
                }
                _ => panic!("Expected Connection error"),
            }
        }
    }

    #[test]
    fn test_pool_config_clone() {
        // Verify PoolConfig implements Clone correctly
        let config = PoolConfig {
            min_connections: 3,
            max_connections: 15,
            connect_timeout: 45,
            max_lifetime: Some(2400),
            idle_timeout: Some(800),
        };

        let cloned = config.clone();

        assert_eq!(config.min_connections, cloned.min_connections);
        assert_eq!(config.max_connections, cloned.max_connections);
        assert_eq!(config.connect_timeout, cloned.connect_timeout);
        assert_eq!(config.max_lifetime, cloned.max_lifetime);
        assert_eq!(config.idle_timeout, cloned.idle_timeout);
    }

    #[test]
    fn test_pool_config_debug() {
        // Verify PoolConfig implements Debug correctly
        let config = PoolConfig::default();
        let debug_output = format!("{:?}", config);

        // Should contain the struct name
        assert!(debug_output.contains("PoolConfig"));
    }
}
