//! PostgreSQL transaction management.
//!
//! This module provides transaction support with ACID guarantees.

use crate::{Connection, Result};
use sqlx::Postgres;

/// Transaction isolation levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    /// Read uncommitted
    ReadUncommitted,
    /// Read committed (PostgreSQL default)
    ReadCommitted,
    /// Repeatable read
    RepeatableRead,
    /// Serializable
    Serializable,
}

impl IsolationLevel {
    /// Returns the SQL isolation level string.
    pub fn to_sql(&self) -> &'static str {
        match self {
            IsolationLevel::ReadUncommitted => "READ UNCOMMITTED",
            IsolationLevel::ReadCommitted => "READ COMMITTED",
            IsolationLevel::RepeatableRead => "REPEATABLE READ",
            IsolationLevel::Serializable => "SERIALIZABLE",
        }
    }
}

/// Represents an active database transaction.
///
/// Transactions ensure ACID properties:
/// - Atomicity: All operations succeed or fail together
/// - Consistency: Database remains in a valid state
/// - Isolation: Concurrent transactions don't interfere
/// - Durability: Committed changes persist
pub struct Transaction {
    tx: sqlx::Transaction<'static, Postgres>,
}

impl Transaction {
    /// Returns a mutable reference to the underlying sqlx transaction.
    pub fn as_mut_transaction(&mut self) -> &mut sqlx::Transaction<'static, Postgres> {
        &mut self.tx
    }

    /// Begins a new transaction.
    ///
    /// # Arguments
    ///
    /// * `conn` - Connection to use for transaction
    /// * `isolation_level` - Transaction isolation level
    ///
    /// # Errors
    ///
    /// Returns error if transaction cannot be started.
    pub async fn begin(conn: &Connection, isolation_level: IsolationLevel) -> Result<Self> {
        // Begin transaction from pool
        let mut tx = conn.pool().begin().await?;

        // Set isolation level
        let sql = format!("SET TRANSACTION ISOLATION LEVEL {}", isolation_level.to_sql());
        sqlx::query(&sql).execute(&mut *tx).await?;

        Ok(Self { tx })
    }

    /// Commits the transaction.
    ///
    /// # Errors
    ///
    /// Returns error if commit fails.
    pub async fn commit(self) -> Result<()> {
        self.tx.commit().await?;
        Ok(())
    }

    /// Rolls back the transaction.
    ///
    /// # Errors
    ///
    /// Returns error if rollback fails.
    pub async fn rollback(self) -> Result<()> {
        self.tx.rollback().await?;
        Ok(())
    }

    /// Creates a savepoint within the transaction.
    ///
    /// # Arguments
    ///
    /// * `name` - Savepoint name
    ///
    /// # Errors
    ///
    /// Returns error if savepoint name is invalid or creation fails.
    pub async fn savepoint(&mut self, name: &str) -> Result<()> {
        use crate::QueryBuilder;

        // Validate savepoint name to prevent SQL injection
        QueryBuilder::validate_identifier(name)?;

        // Execute SAVEPOINT statement
        let sql = format!("SAVEPOINT {}", name);
        sqlx::query(&sql).execute(&mut *self.tx).await?;

        tracing::debug!(savepoint = name, "Created savepoint");
        Ok(())
    }

    /// Rolls back to a savepoint.
    ///
    /// # Arguments
    ///
    /// * `name` - Savepoint name
    ///
    /// # Errors
    ///
    /// Returns error if savepoint name is invalid or rollback fails.
    pub async fn rollback_to(&mut self, name: &str) -> Result<()> {
        use crate::QueryBuilder;

        // Validate savepoint name to prevent SQL injection
        QueryBuilder::validate_identifier(name)?;

        // Execute ROLLBACK TO SAVEPOINT statement
        let sql = format!("ROLLBACK TO SAVEPOINT {}", name);
        sqlx::query(&sql).execute(&mut *self.tx).await?;

        tracing::debug!(savepoint = name, "Rolled back to savepoint");
        Ok(())
    }

    /// Releases a savepoint.
    ///
    /// # Arguments
    ///
    /// * `name` - Savepoint name
    ///
    /// # Errors
    ///
    /// Returns error if savepoint name is invalid or release fails.
    pub async fn release_savepoint(&mut self, name: &str) -> Result<()> {
        use crate::QueryBuilder;

        // Validate savepoint name to prevent SQL injection
        QueryBuilder::validate_identifier(name)?;

        // Execute RELEASE SAVEPOINT statement
        let sql = format!("RELEASE SAVEPOINT {}", name);
        sqlx::query(&sql).execute(&mut *self.tx).await?;

        tracing::debug!(savepoint = name, "Released savepoint");
        Ok(())
    }
}

// Auto-rollback on drop if not committed
// Note: SQLx Transaction already implements Drop with auto-rollback.
// We don't need to manually implement rollback here as SQLx handles it.
// The inner tx will be dropped when this struct is dropped, triggering SQLx's Drop impl.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_isolation_level_to_sql() {
        assert_eq!(IsolationLevel::ReadUncommitted.to_sql(), "READ UNCOMMITTED");
        assert_eq!(IsolationLevel::ReadCommitted.to_sql(), "READ COMMITTED");
        assert_eq!(IsolationLevel::RepeatableRead.to_sql(), "REPEATABLE READ");
        assert_eq!(IsolationLevel::Serializable.to_sql(), "SERIALIZABLE");
    }

    #[test]
    fn test_isolation_level_equality() {
        assert_eq!(IsolationLevel::ReadCommitted, IsolationLevel::ReadCommitted);
        assert_ne!(IsolationLevel::ReadCommitted, IsolationLevel::Serializable);
    }

    #[test]
    fn test_isolation_level_clone() {
        let level = IsolationLevel::Serializable;
        let cloned = level;
        assert_eq!(level, cloned);
    }

    // Integration tests require a live PostgreSQL database
    // Run these with: cargo test --package data-bridge-postgres --features test-postgres
    // TODO: Add integration tests for:
    // - test_transaction_commit()
    // - test_transaction_rollback()
    // - test_transaction_auto_rollback_on_drop()
    // - test_transaction_isolation_levels()
}
