//! Database migration management.
//!
//! This module provides migration support for schema evolution,
//! similar to Alembic/SQLAlchemy migrations but in Rust.

use crate::{Connection, Result, DataBridgeError};
use chrono::{DateTime, Utc};
use std::path::Path;
use std::fs;
use sha2::{Sha256, Digest};
use sqlx::Row;

/// Represents a single database migration.
#[derive(Debug, Clone)]
pub struct Migration {
    /// Migration version (timestamp or sequential number)
    pub version: String,
    /// Migration name/description
    pub name: String,
    /// SQL statements to apply migration (upgrade)
    pub up: String,
    /// SQL statements to revert migration (downgrade)
    pub down: String,
    /// When this migration was applied (None if not applied)
    pub applied_at: Option<DateTime<Utc>>,
    /// SHA256 checksum of migration content
    pub checksum: String,
}

impl Migration {
    /// Creates a new migration.
    ///
    /// # Arguments
    ///
    /// * `version` - Migration version identifier
    /// * `name` - Migration description
    /// * `up` - SQL for applying migration
    /// * `down` - SQL for reverting migration
    pub fn new(version: String, name: String, up: String, down: String) -> Self {
        let content = format!("{}\n{}\n{}", name, up, down);
        let checksum = Self::calculate_checksum(&content);

        Self {
            version,
            name,
            up,
            down,
            applied_at: None,
            checksum,
        }
    }

    /// Loads migration from a SQL file.
    ///
    /// Expected file format:
    /// ```sql
    /// -- Migration: 20250128_120000_create_users_table
    /// -- Description: Create users table with basic columns
    ///
    /// -- UP
    /// CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT);
    ///
    /// -- DOWN
    /// DROP TABLE users;
    /// ```
    pub fn from_file(path: &Path) -> Result<Self> {
        // Read file contents
        let content = fs::read_to_string(path)
            .map_err(|e| DataBridgeError::Internal(format!("Failed to read migration file: {}", e)))?;

        // Extract version and description from filename
        let filename = path.file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| DataBridgeError::Validation("Invalid migration filename".to_string()))?;

        let (version, description) = Self::parse_filename(filename)?;
        let (desc_from_content, up_sql, down_sql) = Self::parse_content(&content)?;

        // Use description from content if available, otherwise from filename
        let name = if !desc_from_content.is_empty() {
            desc_from_content
        } else {
            description
        };

        // Calculate checksum
        let checksum = Self::calculate_checksum(&content);

        Ok(Self {
            version,
            name,
            up: up_sql,
            down: down_sql,
            applied_at: None,
            checksum,
        })
    }

    /// Parses filename to extract version and description.
    ///
    /// Expected format: `20250128_120000_create_users_table`
    fn parse_filename(filename: &str) -> Result<(String, String)> {
        let parts: Vec<&str> = filename.splitn(3, '_').collect();

        if parts.len() < 2 {
            return Err(DataBridgeError::Validation(
                format!("Invalid migration filename format: {}. Expected format: YYYYMMDD_HHMMSS_description", filename)
            ));
        }

        let version = if parts.len() >= 2 {
            format!("{}_{}", parts[0], parts[1])
        } else {
            parts[0].to_string()
        };

        let description = if parts.len() >= 3 {
            parts[2].replace('_', " ")
        } else {
            String::new()
        };

        Ok((version, description))
    }

    /// Parses migration file content to extract description, up, and down SQL.
    fn parse_content(content: &str) -> Result<(String, String, String)> {
        let mut description = String::new();
        let mut up_sql = String::new();
        let mut down_sql = String::new();
        let mut current_section = Section::None;

        for line in content.lines() {
            let trimmed = line.trim();

            // Check for section markers
            if trimmed.starts_with("-- Description:") {
                description = trimmed.trim_start_matches("-- Description:").trim().to_string();
                continue;
            } else if trimmed.eq_ignore_ascii_case("-- UP") || trimmed.eq_ignore_ascii_case("-- migrate:up") {
                current_section = Section::Up;
                continue;
            } else if trimmed.eq_ignore_ascii_case("-- DOWN") || trimmed.eq_ignore_ascii_case("-- migrate:down") {
                current_section = Section::Down;
                continue;
            }

            // Skip other comment lines and empty lines when not in a section
            if current_section == Section::None {
                continue;
            }

            // Add non-empty lines to current section
            match current_section {
                Section::Up => {
                    up_sql.push_str(line);
                    up_sql.push('\n');
                }
                Section::Down => {
                    down_sql.push_str(line);
                    down_sql.push('\n');
                }
                Section::None => {}
            }
        }

        // Validate that we have both UP and DOWN sections
        if up_sql.trim().is_empty() {
            return Err(DataBridgeError::Validation(
                "Migration file missing UP section".to_string()
            ));
        }

        if down_sql.trim().is_empty() {
            return Err(DataBridgeError::Validation(
                "Migration file missing DOWN section".to_string()
            ));
        }

        Ok((description, up_sql.trim().to_string(), down_sql.trim().to_string()))
    }

    /// Calculates SHA256 checksum of content.
    fn calculate_checksum(content: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }
}

/// Section marker for parsing migration files.
#[derive(Debug, PartialEq)]
enum Section {
    None,
    Up,
    Down,
}

/// Splits SQL into individual statements, handling comments, empty lines, and dollar-quoted strings.
///
/// This function properly handles:
/// - Multiple statements separated by semicolons
/// - PostgreSQL dollar-quoted strings ($$...$$ or $tag$...$tag$)
/// - SQL comments (-- style)
/// - Empty statements
///
/// # Arguments
///
/// * `sql` - SQL string potentially containing multiple statements
///
/// # Returns
///
/// Vector of individual SQL statements
fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current_statement = String::new();
    let mut in_dollar_quote = false;
    let mut dollar_quote_tag: Option<String> = None;
    let mut chars = sql.chars().peekable();

    while let Some(ch) = chars.next() {
        current_statement.push(ch);

        if in_dollar_quote {
            // Check if we're ending a dollar-quoted string
            if ch == '$' {
                // Collect the potential closing tag
                let mut potential_tag = String::from("$");
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphanumeric() || next_ch == '_' {
                        potential_tag.push(next_ch);
                        current_statement.push(next_ch);
                        chars.next();
                    } else if next_ch == '$' {
                        potential_tag.push(next_ch);
                        current_statement.push(next_ch);
                        chars.next();
                        break;
                    } else {
                        break;
                    }
                }

                // Check if this matches our opening tag
                if let Some(ref tag) = dollar_quote_tag {
                    if &potential_tag == tag {
                        in_dollar_quote = false;
                        dollar_quote_tag = None;
                    }
                }
            }
        } else {
            // Check if we're starting a dollar-quoted string
            if ch == '$' {
                // Collect the tag
                let mut tag = String::from("$");
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphanumeric() || next_ch == '_' {
                        tag.push(next_ch);
                        current_statement.push(next_ch);
                        chars.next();
                    } else if next_ch == '$' {
                        tag.push(next_ch);
                        current_statement.push(next_ch);
                        chars.next();
                        in_dollar_quote = true;
                        dollar_quote_tag = Some(tag);
                        break;
                    } else {
                        break;
                    }
                }
            } else if ch == ';' {
                // Found statement terminator (semicolon) outside of dollar-quotes
                let stmt = current_statement.trim().trim_end_matches(';').trim();

                // Check if this statement has any SQL (not just comments)
                let has_sql = stmt.lines()
                    .map(|line| line.trim())
                    .any(|line| !line.is_empty() && !line.starts_with("--"));

                if has_sql {
                    statements.push(stmt.to_string());
                }

                current_statement.clear();
            }
        }
    }

    // Don't forget the last statement if it doesn't end with a semicolon
    let final_stmt = current_statement.trim().trim_end_matches(';').trim();
    let has_sql = final_stmt.lines()
        .map(|line| line.trim())
        .any(|line| !line.is_empty() && !line.starts_with("--"));

    if has_sql {
        statements.push(final_stmt.to_string());
    }

    statements
}

/// Migration runner for applying and reverting migrations.
pub struct MigrationRunner {
    conn: Connection,
    migrations_table: String,
}

impl MigrationRunner {
    /// Creates a new migration runner.
    ///
    /// # Arguments
    ///
    /// * `conn` - Database connection
    /// * `migrations_table` - Name of table to track applied migrations (default: "_migrations")
    pub fn new(conn: Connection, migrations_table: Option<String>) -> Self {
        Self {
            conn,
            migrations_table: migrations_table.unwrap_or_else(|| "_migrations".to_string()),
        }
    }

    /// Initializes the migrations tracking table.
    pub async fn init(&self) -> Result<()> {
        let sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                version VARCHAR(255) PRIMARY KEY,
                description TEXT NOT NULL,
                applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                checksum VARCHAR(64) NOT NULL
            )
            "#,
            self.migrations_table
        );

        sqlx::query(&sql)
            .execute(self.conn.pool())
            .await
            .map_err(|e| DataBridgeError::Database(format!("Failed to create migrations table: {}", e)))?;

        tracing::info!("Migrations table '{}' initialized", self.migrations_table);
        Ok(())
    }

    /// Gets list of applied migrations.
    pub async fn applied_migrations(&self) -> Result<Vec<String>> {
        let sql = format!(
            "SELECT version FROM {} ORDER BY version",
            self.migrations_table
        );

        let rows = sqlx::query_scalar::<_, String>(&sql)
            .fetch_all(self.conn.pool())
            .await
            .map_err(|e| DataBridgeError::Database(format!("Failed to fetch applied migrations: {}", e)))?;

        Ok(rows)
    }

    /// Gets list of applied migrations with full details.
    pub async fn applied_migrations_with_details(&self) -> Result<Vec<Migration>> {
        let sql = format!(
            "SELECT version, description, applied_at, checksum FROM {} ORDER BY version",
            self.migrations_table
        );

        let rows = sqlx::query(&sql)
            .fetch_all(self.conn.pool())
            .await
            .map_err(|e| DataBridgeError::Database(format!("Failed to fetch applied migrations: {}", e)))?;

        let mut migrations = Vec::new();
        for row in rows {
            let version: String = row.try_get("version")
                .map_err(|e| DataBridgeError::Database(format!("Failed to get version: {}", e)))?;
            let description: String = row.try_get("description")
                .map_err(|e| DataBridgeError::Database(format!("Failed to get description: {}", e)))?;
            let applied_at: DateTime<Utc> = row.try_get("applied_at")
                .map_err(|e| DataBridgeError::Database(format!("Failed to get applied_at: {}", e)))?;
            let checksum: String = row.try_get("checksum")
                .map_err(|e| DataBridgeError::Database(format!("Failed to get checksum: {}", e)))?;

            migrations.push(Migration {
                version,
                name: description,
                up: String::new(),
                down: String::new(),
                applied_at: Some(applied_at),
                checksum,
            });
        }

        Ok(migrations)
    }

    /// Gets list of pending migrations.
    pub async fn pending_migrations(&self, all_migrations: &[Migration]) -> Result<Vec<Migration>> {
        let applied = self.applied_migrations().await?;
        let applied_set: std::collections::HashSet<_> = applied.into_iter().collect();

        let pending: Vec<Migration> = all_migrations
            .iter()
            .filter(|m| !applied_set.contains(&m.version))
            .cloned()
            .collect();

        Ok(pending)
    }

    /// Verifies checksum of an applied migration.
    async fn verify_checksum(&self, migration: &Migration) -> Result<bool> {
        let sql = format!(
            "SELECT checksum FROM {} WHERE version = $1",
            self.migrations_table
        );

        let checksum: Option<String> = sqlx::query_scalar(&sql)
            .bind(&migration.version)
            .fetch_optional(self.conn.pool())
            .await
            .map_err(|e| DataBridgeError::Database(format!("Failed to verify checksum: {}", e)))?;

        match checksum {
            Some(stored_checksum) => Ok(stored_checksum == migration.checksum),
            None => Ok(true), // Migration not applied yet
        }
    }

    /// Applies a single migration.
    pub async fn apply(&self, migration: &Migration) -> Result<()> {
        // Verify checksum if migration was already applied
        if !self.verify_checksum(migration).await? {
            return Err(DataBridgeError::Validation(
                format!("Checksum mismatch for migration {}. The migration file has been modified after being applied.", migration.version)
            ));
        }

        // Begin transaction
        let mut tx = self.conn.pool().begin()
            .await
            .map_err(|e| DataBridgeError::Database(format!("Failed to begin transaction: {}", e)))?;

        // Split SQL into individual statements and execute each
        let statements = split_sql_statements(&migration.up);
        for (idx, statement) in statements.iter().enumerate() {
            if !statement.trim().is_empty() {
                sqlx::query(statement)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| {
                        DataBridgeError::Database(format!(
                            "Failed to apply migration {} (statement {}): {}",
                            migration.version, idx + 1, e
                        ))
                    })?;
            }
        }

        // Record migration
        let insert_sql = format!(
            "INSERT INTO {} (version, description, checksum) VALUES ($1, $2, $3)",
            self.migrations_table
        );

        sqlx::query(&insert_sql)
            .bind(&migration.version)
            .bind(&migration.name)
            .bind(&migration.checksum)
            .execute(&mut *tx)
            .await
            .map_err(|e| DataBridgeError::Database(format!("Failed to record migration: {}", e)))?;

        // Commit transaction
        tx.commit()
            .await
            .map_err(|e| DataBridgeError::Database(format!("Failed to commit migration: {}", e)))?;

        tracing::info!("Applied migration: {} - {}", migration.version, migration.name);
        Ok(())
    }

    /// Reverts a single migration.
    pub async fn revert(&self, migration: &Migration) -> Result<()> {
        // Begin transaction
        let mut tx = self.conn.pool().begin()
            .await
            .map_err(|e| DataBridgeError::Database(format!("Failed to begin transaction: {}", e)))?;

        // Split SQL into individual statements and execute each
        let statements = split_sql_statements(&migration.down);
        for (idx, statement) in statements.iter().enumerate() {
            if !statement.trim().is_empty() {
                sqlx::query(statement)
                    .execute(&mut *tx)
                    .await
                    .map_err(|e| {
                        DataBridgeError::Database(format!(
                            "Failed to revert migration {} (statement {}): {}",
                            migration.version, idx + 1, e
                        ))
                    })?;
            }
        }

        // Remove migration record
        let delete_sql = format!(
            "DELETE FROM {} WHERE version = $1",
            self.migrations_table
        );

        sqlx::query(&delete_sql)
            .bind(&migration.version)
            .execute(&mut *tx)
            .await
            .map_err(|e| DataBridgeError::Database(format!("Failed to remove migration record: {}", e)))?;

        // Commit transaction
        tx.commit()
            .await
            .map_err(|e| DataBridgeError::Database(format!("Failed to commit rollback: {}", e)))?;

        tracing::info!("Reverted migration: {} - {}", migration.version, migration.name);
        Ok(())
    }

    /// Applies all pending migrations.
    pub async fn migrate(&self, migrations: &[Migration]) -> Result<Vec<String>> {
        let pending = self.pending_migrations(migrations).await?;

        if pending.is_empty() {
            tracing::info!("No pending migrations to apply");
            return Ok(Vec::new());
        }

        let mut applied = Vec::new();

        for migration in &pending {
            self.apply(migration).await?;
            applied.push(migration.version.clone());
        }

        tracing::info!("Applied {} migrations", applied.len());
        Ok(applied)
    }

    /// Reverts the last N migrations.
    pub async fn rollback(&self, migrations: &[Migration], count: usize) -> Result<Vec<String>> {
        let applied = self.applied_migrations().await?;

        if applied.is_empty() {
            tracing::info!("No migrations to rollback");
            return Ok(Vec::new());
        }

        // Get the last N migrations to revert
        let to_revert_count = count.min(applied.len());
        let to_revert_versions: Vec<String> = applied
            .iter()
            .rev()
            .take(to_revert_count)
            .cloned()
            .collect();

        // Find the corresponding Migration objects
        let mut migrations_to_revert = Vec::new();
        for version in &to_revert_versions {
            if let Some(migration) = migrations.iter().find(|m| &m.version == version) {
                migrations_to_revert.push(migration.clone());
            } else {
                return Err(DataBridgeError::Validation(
                    format!("Migration {} is applied but not found in migration files", version)
                ));
            }
        }

        let mut reverted = Vec::new();

        for migration in &migrations_to_revert {
            self.revert(migration).await?;
            reverted.push(migration.version.clone());
        }

        tracing::info!("Reverted {} migrations", reverted.len());
        Ok(reverted)
    }

    /// Loads migrations from a directory.
    ///
    /// Scans directory for .sql files and loads them as migrations.
    pub fn load_from_directory(path: &Path) -> Result<Vec<Migration>> {
        if !path.exists() {
            return Err(DataBridgeError::Internal(
                format!("Migration directory does not exist: {}", path.display())
            ));
        }

        if !path.is_dir() {
            return Err(DataBridgeError::Internal(
                format!("Path is not a directory: {}", path.display())
            ));
        }

        let mut migrations = Vec::new();

        let entries = fs::read_dir(path)
            .map_err(|e| DataBridgeError::Internal(format!("Failed to read directory: {}", e)))?;

        for entry in entries {
            let entry = entry
                .map_err(|e| DataBridgeError::Internal(format!("Failed to read directory entry: {}", e)))?;
            let file_path = entry.path();

            // Skip non-SQL files
            if file_path.extension().and_then(|s: &std::ffi::OsStr| s.to_str()) != Some("sql") {
                continue;
            }

            // Skip hidden files
            if file_path.file_name()
                .and_then(|s: &std::ffi::OsStr| s.to_str())
                .map(|s: &str| s.starts_with('.'))
                .unwrap_or(false)
            {
                continue;
            }

            match Migration::from_file(&file_path) {
                Ok(migration) => {
                    migrations.push(migration);
                }
                Err(e) => {
                    tracing::warn!("Failed to load migration from {}: {}", file_path.display(), e);
                }
            }
        }

        // Sort migrations by version
        migrations.sort_by(|a, b| a.version.cmp(&b.version));

        tracing::info!("Loaded {} migrations from {}", migrations.len(), path.display());
        Ok(migrations)
    }

    /// Gets migration status information.
    pub async fn status(&self, migrations: &[Migration]) -> Result<MigrationStatus> {
        let applied = self.applied_migrations().await?;
        let pending = self.pending_migrations(migrations).await?;

        Ok(MigrationStatus {
            applied,
            pending: pending.iter().map(|m| m.version.clone()).collect(),
        })
    }
}

/// Migration status information.
#[derive(Debug, Clone)]
pub struct MigrationStatus {
    /// List of applied migration versions
    pub applied: Vec<String>,
    /// List of pending migration versions
    pub pending: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_sql_statements_single_statement() {
        let sql = "CREATE TABLE users (id SERIAL PRIMARY KEY)";
        let statements = split_sql_statements(sql);
        assert_eq!(statements.len(), 1);
        assert_eq!(statements[0], "CREATE TABLE users (id SERIAL PRIMARY KEY)");
    }

    #[test]
    fn test_split_sql_statements_multiple_statements() {
        let sql = r#"
            CREATE TABLE users (id SERIAL PRIMARY KEY);
            CREATE INDEX idx_users ON users(id);
            INSERT INTO users VALUES (1);
        "#;

        let statements = split_sql_statements(sql);
        assert_eq!(statements.len(), 3);
        assert!(statements[0].contains("CREATE TABLE users"));
        assert!(statements[1].contains("CREATE INDEX idx_users"));
        assert!(statements[2].contains("INSERT INTO users"));
    }

    #[test]
    fn test_split_sql_statements_with_inline_comments() {
        // Comments within SQL statements (before semicolons) are preserved
        let sql = r#"
            CREATE TABLE users ( -- inline comment
                id SERIAL
            );
            CREATE INDEX idx_users ON users(id);
        "#;

        let statements = split_sql_statements(sql);
        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("CREATE TABLE users"));
        assert!(statements[0].contains("-- inline comment"));
        assert!(statements[1].contains("CREATE INDEX idx_users"));
    }

    #[test]
    fn test_split_sql_statements_only_comment_statements_filtered() {
        // Pure comment statements (between semicolons) are filtered out
        let sql = r#"
            CREATE TABLE users (id SERIAL);
            -- This is just a comment, no SQL
            CREATE INDEX idx_users ON users(id);
        "#;

        let statements = split_sql_statements(sql);
        // Only the two SQL statements are kept, pure comment is filtered
        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("CREATE TABLE users"));
        assert!(statements[1].contains("CREATE INDEX idx_users"));
    }

    #[test]
    fn test_split_sql_statements_leading_comments_included() {
        // Leading comments become part of the first statement
        let sql = r#"
            -- This comment becomes part of the statement
            CREATE TABLE users (id SERIAL);
        "#;

        let statements = split_sql_statements(sql);
        assert_eq!(statements.len(), 1);
        assert!(statements[0].contains("-- This comment"));
        assert!(statements[0].contains("CREATE TABLE users"));
    }

    #[test]
    fn test_split_sql_statements_with_empty_lines() {
        let sql = r#"
            CREATE TABLE users (id SERIAL);

            CREATE INDEX idx_users ON users(id);

        "#;

        let statements = split_sql_statements(sql);
        assert_eq!(statements.len(), 2);
    }

    #[test]
    fn test_split_sql_statements_empty_input() {
        let sql = "";
        let statements = split_sql_statements(sql);
        assert_eq!(statements.len(), 0);
    }

    #[test]
    fn test_split_sql_statements_only_whitespace() {
        let sql = "   \n\n\t  \n  ";
        let statements = split_sql_statements(sql);
        assert_eq!(statements.len(), 0);
    }

    #[test]
    fn test_split_sql_statements_semicolon_only() {
        let sql = ";;;";
        let statements = split_sql_statements(sql);
        assert_eq!(statements.len(), 0);
    }

    #[test]
    fn test_split_sql_statements_complex_migration() {
        let sql = r#"
            CREATE TABLE products (
                id SERIAL PRIMARY KEY,
                name VARCHAR(255) NOT NULL,
                price DECIMAL(10, 2),
                created_at TIMESTAMPTZ DEFAULT NOW()
            );

            CREATE INDEX idx_products_name ON products(name);

            CREATE TABLE orders (
                id SERIAL PRIMARY KEY,
                product_id INTEGER REFERENCES products(id),
                quantity INTEGER NOT NULL
            );
        "#;

        let statements = split_sql_statements(sql);
        assert_eq!(statements.len(), 3);
        assert!(statements[0].contains("CREATE TABLE products"));
        assert!(statements[1].contains("CREATE INDEX idx_products_name"));
        assert!(statements[2].contains("CREATE TABLE orders"));
    }

    #[test]
    fn test_split_sql_statements_with_dollar_quotes() {
        let sql = r#"
            CREATE FUNCTION test_func()
            RETURNS TRIGGER AS $$
            BEGIN
                NEW.updated_at = NOW();
                RETURN NEW;
            END;
            $$ LANGUAGE plpgsql;

            CREATE TRIGGER test_trigger BEFORE UPDATE ON users
                FOR EACH ROW EXECUTE FUNCTION test_func();
        "#;

        let statements = split_sql_statements(sql);
        assert_eq!(statements.len(), 2);
        assert!(statements[0].contains("CREATE FUNCTION"));
        assert!(statements[0].contains("$$"));
        assert!(statements[0].contains("NEW.updated_at = NOW()"));
        assert!(statements[1].contains("CREATE TRIGGER"));
    }

    #[test]
    fn test_split_sql_statements_with_tagged_dollar_quotes() {
        let sql = r#"
            CREATE FUNCTION complex_func()
            RETURNS TEXT AS $function$
            DECLARE
                result TEXT;
            BEGIN
                result := 'test; with semicolon';
                RETURN result;
            END;
            $function$ LANGUAGE plpgsql;
        "#;

        let statements = split_sql_statements(sql);
        assert_eq!(statements.len(), 1);
        assert!(statements[0].contains("CREATE FUNCTION"));
        assert!(statements[0].contains("$function$"));
        assert!(statements[0].contains("test; with semicolon"));
    }

    #[test]
    fn test_split_sql_statements_real_migration() {
        // This is from the actual migration file
        let sql = r#"
            CREATE TABLE users (
                id SERIAL PRIMARY KEY,
                email VARCHAR(255) UNIQUE NOT NULL,
                name VARCHAR(255),
                created_at TIMESTAMPTZ DEFAULT NOW(),
                updated_at TIMESTAMPTZ DEFAULT NOW()
            );

            CREATE INDEX idx_users_email ON users(email);
            CREATE INDEX idx_users_created_at ON users(created_at);

            CREATE OR REPLACE FUNCTION update_updated_at_column()
            RETURNS TRIGGER AS $$
            BEGIN
                NEW.updated_at = NOW();
                RETURN NEW;
            END;
            $$ language 'plpgsql';

            CREATE TRIGGER update_users_updated_at BEFORE UPDATE ON users
                FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
        "#;

        let statements = split_sql_statements(sql);
        assert_eq!(statements.len(), 5);
        assert!(statements[0].contains("CREATE TABLE users"));
        assert!(statements[1].contains("CREATE INDEX idx_users_email"));
        assert!(statements[2].contains("CREATE INDEX idx_users_created_at"));
        assert!(statements[3].contains("CREATE OR REPLACE FUNCTION"));
        assert!(statements[3].contains("$$"));
        assert!(statements[4].contains("CREATE TRIGGER"));
    }
}
