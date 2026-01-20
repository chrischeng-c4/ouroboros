//! Integration tests for Transaction operations.
//!
//! These tests require a PostgreSQL database to be running.
//! Set DATABASE_URL environment variable or skip with SKIP_INTEGRATION=true
//!
//! Run with: cargo test -p ouroboros-postgres test_transaction -- --ignored

use ouroboros_postgres::{Connection, IsolationLevel, PoolConfig, Transaction};
use ouroboros_qc::{expect, AssertionError};

/// Helper to get database URL from environment
fn get_database_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgresql://localhost/test_db".to_string())
}

/// Helper to create a test table for transaction tests
async fn setup_test_table(
    pool: &sqlx::PgPool,
    table_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(&format!("DROP TABLE IF EXISTS {} CASCADE", table_name))
        .execute(pool)
        .await?;

    sqlx::query(&format!(
        "CREATE TABLE {} (
            id BIGSERIAL PRIMARY KEY,
            name TEXT NOT NULL,
            value INTEGER NOT NULL
        )",
        table_name
    ))
    .execute(pool)
    .await?;

    Ok(())
}

/// Helper to cleanup test table
async fn cleanup_test_table(
    pool: &sqlx::PgPool,
    table_name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(&format!("DROP TABLE IF EXISTS {} CASCADE", table_name))
        .execute(pool)
        .await?;
    Ok(())
}

/// Helper to count rows in table
async fn count_rows(pool: &sqlx::PgPool, table_name: &str) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", table_name))
        .fetch_one(pool)
        .await?;
    Ok(row.0)
}

/// Helper to check if a record exists by name
async fn record_exists(
    pool: &sqlx::PgPool,
    table_name: &str,
    name: &str,
) -> Result<bool, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(&format!(
        "SELECT COUNT(*) FROM {} WHERE name = $1",
        table_name
    ))
    .bind(name)
    .fetch_one(pool)
    .await?;
    Ok(row.0 > 0)
}

// =============================================================================
// Commit/Rollback Tests
// =============================================================================

#[tokio::test]
#[ignore]
async fn test_transaction_commit() -> Result<(), Box<dyn std::error::Error>> {
    let uri = get_database_url();
    let conn = Connection::new(&uri, PoolConfig::default()).await?;
    let pool = conn.pool();
    let table = "test_txn_commit";

    setup_test_table(pool, table).await?;

    // Begin transaction, insert record, commit
    let mut txn = Transaction::begin(&conn, IsolationLevel::ReadCommitted).await?;

    sqlx::query(&format!("INSERT INTO {} (name, value) VALUES ($1, $2)", table))
        .bind("Alice")
        .bind(100)
        .execute(txn.as_mut_transaction().as_mut())
        .await?;

    txn.commit().await?;

    // Verify record exists with a new connection
    let exists = record_exists(pool, table, "Alice").await?;
    expect(exists).to_be_true()?;

    cleanup_test_table(pool, table).await?;
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_transaction_rollback() -> Result<(), Box<dyn std::error::Error>> {
    let uri = get_database_url();
    let conn = Connection::new(&uri, PoolConfig::default()).await?;
    let pool = conn.pool();
    let table = "test_txn_rollback";

    setup_test_table(pool, table).await?;

    // Begin transaction, insert record, rollback
    let mut txn = Transaction::begin(&conn, IsolationLevel::ReadCommitted).await?;

    sqlx::query(&format!("INSERT INTO {} (name, value) VALUES ($1, $2)", table))
        .bind("Bob")
        .bind(200)
        .execute(txn.as_mut_transaction().as_mut())
        .await?;

    txn.rollback().await?;

    // Verify record does NOT exist
    let exists = record_exists(pool, table, "Bob").await?;
    expect(exists).to_be_false()?;

    cleanup_test_table(pool, table).await?;
    Ok(())
}

// =============================================================================
// Savepoint Tests
// =============================================================================

#[tokio::test]
#[ignore]
async fn test_savepoint_partial_rollback() -> Result<(), Box<dyn std::error::Error>> {
    let uri = get_database_url();
    let conn = Connection::new(&uri, PoolConfig::default()).await?;
    let pool = conn.pool();
    let table = "test_txn_savepoint";

    setup_test_table(pool, table).await?;

    // Begin transaction
    let mut txn = Transaction::begin(&conn, IsolationLevel::ReadCommitted).await?;

    // Insert A
    sqlx::query(&format!("INSERT INTO {} (name, value) VALUES ($1, $2)", table))
        .bind("RecordA")
        .bind(100)
        .execute(txn.as_mut_transaction().as_mut())
        .await?;

    // Create savepoint
    txn.savepoint("sp1").await?;

    // Insert B after savepoint
    sqlx::query(&format!("INSERT INTO {} (name, value) VALUES ($1, $2)", table))
        .bind("RecordB")
        .bind(200)
        .execute(txn.as_mut_transaction().as_mut())
        .await?;

    // Rollback to savepoint (undoes B, keeps A)
    txn.rollback_to("sp1").await?;

    // Commit transaction
    txn.commit().await?;

    // Verify: A exists, B does not
    let a_exists = record_exists(pool, table, "RecordA").await?;
    let b_exists = record_exists(pool, table, "RecordB").await?;

    expect(a_exists).to_be_true()?;
    expect(b_exists).to_be_false()?;

    cleanup_test_table(pool, table).await?;
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_release_savepoint() -> Result<(), Box<dyn std::error::Error>> {
    let uri = get_database_url();
    let conn = Connection::new(&uri, PoolConfig::default()).await?;
    let pool = conn.pool();
    let table = "test_txn_release_sp";

    setup_test_table(pool, table).await?;

    // Begin transaction
    let mut txn = Transaction::begin(&conn, IsolationLevel::ReadCommitted).await?;

    // Insert A
    sqlx::query(&format!("INSERT INTO {} (name, value) VALUES ($1, $2)", table))
        .bind("RecordA")
        .bind(100)
        .execute(txn.as_mut_transaction().as_mut())
        .await?;

    // Create savepoint
    txn.savepoint("sp1").await?;

    // Insert B after savepoint
    sqlx::query(&format!("INSERT INTO {} (name, value) VALUES ($1, $2)", table))
        .bind("RecordB")
        .bind(200)
        .execute(txn.as_mut_transaction().as_mut())
        .await?;

    // Release savepoint (confirms changes since savepoint)
    txn.release_savepoint("sp1").await?;

    // Commit transaction
    txn.commit().await?;

    // Verify: both A and B exist
    let a_exists = record_exists(pool, table, "RecordA").await?;
    let b_exists = record_exists(pool, table, "RecordB").await?;

    expect(a_exists).to_be_true()?;
    expect(b_exists).to_be_true()?;

    cleanup_test_table(pool, table).await?;
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_invalid_savepoint_name() -> Result<(), AssertionError> {
    let uri = get_database_url();
    let conn = Connection::new(&uri, PoolConfig::default()).await.unwrap();

    // Begin transaction
    let mut txn = Transaction::begin(&conn, IsolationLevel::ReadCommitted)
        .await
        .unwrap();

    // Invalid savepoint name with space should fail validation
    let result = txn.savepoint("invalid name").await;

    expect(result.is_err()).to_be_true()?;

    // Verify error is InvalidIdentifier
    let err = result.unwrap_err();
    let err_str = err.to_string();
    expect(err_str.contains("Invalid") || err_str.contains("identifier")).to_be_true()?;

    // Transaction will be auto-rolled back on drop
    Ok(())
}

// =============================================================================
// Isolation Level Tests
// =============================================================================

#[tokio::test]
#[ignore]
async fn test_isolation_level_repeatable_read() -> Result<(), Box<dyn std::error::Error>> {
    let uri = get_database_url();
    let conn = Connection::new(&uri, PoolConfig::default()).await?;

    // Begin transaction with RepeatableRead isolation
    let mut txn = Transaction::begin(&conn, IsolationLevel::RepeatableRead).await?;

    // Query current isolation level
    let row: (String,) = sqlx::query_as("SHOW transaction_isolation")
        .fetch_one(txn.as_mut_transaction().as_mut())
        .await?;

    expect(row.0.as_str()).to_equal(&"repeatable read")?;

    txn.rollback().await?;
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_isolation_level_serializable() -> Result<(), Box<dyn std::error::Error>> {
    let uri = get_database_url();
    let conn = Connection::new(&uri, PoolConfig::default()).await?;

    // Begin transaction with Serializable isolation
    let mut txn = Transaction::begin(&conn, IsolationLevel::Serializable).await?;

    // Query current isolation level
    let row: (String,) = sqlx::query_as("SHOW transaction_isolation")
        .fetch_one(txn.as_mut_transaction().as_mut())
        .await?;

    expect(row.0.as_str()).to_equal(&"serializable")?;

    txn.rollback().await?;
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_isolation_level_read_committed() -> Result<(), Box<dyn std::error::Error>> {
    let uri = get_database_url();
    let conn = Connection::new(&uri, PoolConfig::default()).await?;

    // Begin transaction with ReadCommitted isolation (default)
    let mut txn = Transaction::begin(&conn, IsolationLevel::ReadCommitted).await?;

    // Query current isolation level
    let row: (String,) = sqlx::query_as("SHOW transaction_isolation")
        .fetch_one(txn.as_mut_transaction().as_mut())
        .await?;

    expect(row.0.as_str()).to_equal(&"read committed")?;

    txn.rollback().await?;
    Ok(())
}

// =============================================================================
// Auto-Rollback on Drop Tests
// =============================================================================

#[tokio::test]
#[ignore]
async fn test_auto_rollback_on_drop() -> Result<(), Box<dyn std::error::Error>> {
    let uri = get_database_url();
    let conn = Connection::new(&uri, PoolConfig::default()).await?;
    let pool = conn.pool();
    let table = "test_txn_auto_rollback";

    setup_test_table(pool, table).await?;

    // Scope to ensure transaction is dropped
    {
        let mut txn = Transaction::begin(&conn, IsolationLevel::ReadCommitted).await?;

        // Insert record
        sqlx::query(&format!("INSERT INTO {} (name, value) VALUES ($1, $2)", table))
            .bind("Dropped")
            .bind(999)
            .execute(txn.as_mut_transaction().as_mut())
            .await?;

        // Transaction is dropped here without commit or explicit rollback
        // SQLx should auto-rollback
    }

    // Verify record does NOT exist (auto-rolled back)
    let exists = record_exists(pool, table, "Dropped").await?;
    expect(exists).to_be_false()?;

    cleanup_test_table(pool, table).await?;
    Ok(())
}

// =============================================================================
// Additional Edge Case Tests
// =============================================================================

#[tokio::test]
#[ignore]
async fn test_multiple_savepoints() -> Result<(), Box<dyn std::error::Error>> {
    let uri = get_database_url();
    let conn = Connection::new(&uri, PoolConfig::default()).await?;
    let pool = conn.pool();
    let table = "test_txn_multi_sp";

    setup_test_table(pool, table).await?;

    let mut txn = Transaction::begin(&conn, IsolationLevel::ReadCommitted).await?;

    // Insert A
    sqlx::query(&format!("INSERT INTO {} (name, value) VALUES ($1, $2)", table))
        .bind("A")
        .bind(1)
        .execute(txn.as_mut_transaction().as_mut())
        .await?;

    txn.savepoint("sp1").await?;

    // Insert B
    sqlx::query(&format!("INSERT INTO {} (name, value) VALUES ($1, $2)", table))
        .bind("B")
        .bind(2)
        .execute(txn.as_mut_transaction().as_mut())
        .await?;

    txn.savepoint("sp2").await?;

    // Insert C
    sqlx::query(&format!("INSERT INTO {} (name, value) VALUES ($1, $2)", table))
        .bind("C")
        .bind(3)
        .execute(txn.as_mut_transaction().as_mut())
        .await?;

    // Rollback to sp2 (undoes C only)
    txn.rollback_to("sp2").await?;

    // Rollback to sp1 (undoes B as well)
    txn.rollback_to("sp1").await?;

    txn.commit().await?;

    // Verify: only A exists
    let a_exists = record_exists(pool, table, "A").await?;
    let b_exists = record_exists(pool, table, "B").await?;
    let c_exists = record_exists(pool, table, "C").await?;

    expect(a_exists).to_be_true()?;
    expect(b_exists).to_be_false()?;
    expect(c_exists).to_be_false()?;

    let count = count_rows(pool, table).await?;
    expect(count).to_equal(&1)?;

    cleanup_test_table(pool, table).await?;
    Ok(())
}
