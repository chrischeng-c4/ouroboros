//! Payload database for security testing
//!
//! Comprehensive collection of malicious payloads for testing input validation,
//! SQL injection prevention, and security boundaries.

/// Category of security payloads
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadCategory {
    /// SQL injection attacks
    SqlInjection,
    /// Identifier-based injection (table/column names)
    IdentifierInjection,
    /// Unicode tricks and homoglyphs
    UnicodeTricks,
    /// Buffer overflow and resource exhaustion
    Overflow,
}

/// Database of security test payloads
#[derive(Debug)]
pub struct PayloadDatabase {
    sql_injection_payloads: Vec<String>,
    identifier_injection_payloads: Vec<String>,
    unicode_trick_payloads: Vec<String>,
    overflow_payloads: Vec<String>,
}

impl PayloadDatabase {
    /// Create a new payload database with all payloads
    pub fn new() -> Self {
        Self {
            sql_injection_payloads: Self::init_sql_injection(),
            identifier_injection_payloads: Self::init_identifier_injection(),
            unicode_trick_payloads: Self::init_unicode_tricks(),
            overflow_payloads: Self::init_overflow(),
        }
    }

    /// Get SQL injection payloads
    pub fn sql_injection(&self) -> &[String] {
        &self.sql_injection_payloads
    }

    /// Get identifier injection payloads
    pub fn identifier_injection(&self) -> &[String] {
        &self.identifier_injection_payloads
    }

    /// Get unicode trick payloads
    pub fn unicode_tricks(&self) -> &[String] {
        &self.unicode_trick_payloads
    }

    /// Get overflow payloads
    pub fn overflow(&self) -> &[String] {
        &self.overflow_payloads
    }

    /// Get payloads by category
    pub fn by_category(&self, category: PayloadCategory) -> &[String] {
        match category {
            PayloadCategory::SqlInjection => self.sql_injection(),
            PayloadCategory::IdentifierInjection => self.identifier_injection(),
            PayloadCategory::UnicodeTricks => self.unicode_tricks(),
            PayloadCategory::Overflow => self.overflow(),
        }
    }

    /// Get all payloads
    pub fn all(&self) -> Vec<&String> {
        let mut all = Vec::new();
        all.extend(self.sql_injection_payloads.iter());
        all.extend(self.identifier_injection_payloads.iter());
        all.extend(self.unicode_trick_payloads.iter());
        all.extend(self.overflow_payloads.iter());
        all
    }

    /// Initialize SQL injection payloads
    fn init_sql_injection() -> Vec<String> {
        vec![
            // Classic SQL injection
            "' OR '1'='1".to_string(),
            "' OR 1=1--".to_string(),
            "' OR 1=1#".to_string(),
            "' OR 1=1/*".to_string(),
            "admin'--".to_string(),
            "admin' #".to_string(),
            "admin'/*".to_string(),
            "' or 1=1--".to_string(),
            "' or 1=1#".to_string(),
            "' or 1=1/*".to_string(),
            "') or '1'='1--".to_string(),
            "') or ('1'='1--".to_string(),

            // UNION-based injection
            "' UNION SELECT NULL--".to_string(),
            "' UNION SELECT NULL, NULL--".to_string(),
            "' UNION ALL SELECT NULL--".to_string(),
            "' UNION SELECT 1,2,3--".to_string(),
            "' UNION SELECT username, password FROM users--".to_string(),

            // Stacked queries
            "'; DROP TABLE users--".to_string(),
            "'; DELETE FROM users--".to_string(),
            "'; UPDATE users SET password='hacked'--".to_string(),
            "'; EXEC sp_MSForEachTable 'DROP TABLE ?'--".to_string(),

            // Boolean-based blind injection
            "' AND 1=1--".to_string(),
            "' AND 1=2--".to_string(),
            "' AND SLEEP(5)--".to_string(),
            "' AND BENCHMARK(10000000,MD5('A'))--".to_string(),

            // Time-based blind injection
            "'; WAITFOR DELAY '00:00:05'--".to_string(),
            "' AND SLEEP(5) AND '1'='1".to_string(),
            "' AND pg_sleep(5)--".to_string(),

            // Error-based injection
            "' AND 1=CONVERT(int,(SELECT @@version))--".to_string(),
            "' AND extractvalue(1,concat(0x7e,version()))--".to_string(),

            // Comment-based
            "--".to_string(),
            "#".to_string(),
            "/**/".to_string(),
            "/*!50000 SELECT * FROM users*/".to_string(),

            // Encoded injection
            "%27%20OR%20%271%27%3D%271".to_string(),
            "\\x27 OR 1=1--".to_string(),

            // NoSQL injection (MongoDB specific)
            "{\"$gt\": \"\"}".to_string(),
            "{\"$ne\": null}".to_string(),
            "{\"$regex\": \".*\"}".to_string(),
            "{\"$where\": \"1==1\"}".to_string(),
            "'; return true; var foo='".to_string(),
            "'; this.password != ''".to_string(),

            // Special characters
            "';--".to_string(),
            "\"--".to_string(),
            "';#".to_string(),
            "\"#".to_string(),

            // Multiple statements
            "'; SELECT * FROM users WHERE '1'='1".to_string(),
            "\"; SELECT * FROM users WHERE \"1\"=\"1".to_string(),

            // NULL byte injection
            "admin\0".to_string(),
            "admin%00".to_string(),
        ]
    }

    /// Initialize identifier injection payloads (for table/column names)
    fn init_identifier_injection() -> Vec<String> {
        vec![
            // Schema attacks
            "information_schema.tables".to_string(),
            "pg_catalog.pg_tables".to_string(),
            "sys.tables".to_string(),
            "mysql.user".to_string(),

            // Quotes and escaping
            "users'--".to_string(),
            "users\"--".to_string(),
            "`users`".to_string(),
            "[users]".to_string(),
            "\"users\"".to_string(),

            // SQL keywords as identifiers
            "SELECT".to_string(),
            "DROP".to_string(),
            "TABLE".to_string(),
            "WHERE".to_string(),
            "UNION".to_string(),
            "INSERT".to_string(),
            "DELETE".to_string(),
            "UPDATE".to_string(),

            // Special characters in identifiers
            "user$name".to_string(),
            "user.name".to_string(),
            "user name".to_string(),
            "user;name".to_string(),
            "user--name".to_string(),
            "user/**/name".to_string(),

            // Path traversal in identifiers
            "../users".to_string(),
            "../../etc/passwd".to_string(),
            "..\\..\\windows\\system32".to_string(),

            // MongoDB special operators
            "$where".to_string(),
            "$gt".to_string(),
            "$ne".to_string(),
            "$regex".to_string(),
        ]
    }

    /// Initialize unicode trick payloads
    fn init_unicode_tricks() -> Vec<String> {
        vec![
            // Homoglyphs (look-alike characters)
            "аdmin".to_string(), // Cyrillic 'а' instead of Latin 'a'
            "аdmіn".to_string(), // Multiple Cyrillic chars
            "ṁongodbḃase".to_string(), // Dotted characters

            // Zero-width characters
            "ad\u{200B}min".to_string(), // Zero-width space
            "ad\u{200C}min".to_string(), // Zero-width non-joiner
            "ad\u{200D}min".to_string(), // Zero-width joiner
            "ad\u{FEFF}min".to_string(), // Zero-width no-break space

            // Right-to-left override
            "admin\u{202E}niam".to_string(), // RTL override
            "\u{202E}SELECT * FROM users".to_string(),

            // Combining characters
            "a\u{0301}dmin".to_string(), // Combining acute accent
            "a\u{0300}dmin".to_string(), // Combining grave accent

            // Normalization attacks
            "ﬁle".to_string(), // Ligature fi
            "ﬀ".to_string(), // Ligature ff

            // Control characters
            "admin\u{0000}".to_string(), // NULL
            "admin\u{0001}".to_string(), // SOH
            "admin\u{001F}".to_string(), // Unit separator

            // Bidirectional text
            "\u{061C}admin".to_string(), // Arabic letter mark
            "admin\u{2066}test\u{2069}".to_string(), // Directional isolates

            // Additional homoglyphs
            "раssword".to_string(), // Cyrillic 'р' and 'а'
            "ехample".to_string(), // Cyrillic 'е' and 'х'
        ]
    }

    /// Initialize overflow payloads
    fn init_overflow() -> Vec<String> {
        vec![
            // Long strings
            "A".repeat(1000),
            "A".repeat(10000),
            "A".repeat(100000),

            // Many parameters
            format!("?{}", (0..1000).map(|i| format!("param{}=value", i)).collect::<Vec<_>>().join("&")),

            // Deep nesting
            "{{{{{{{{{{".to_string(),
            "[[[[[[[[[[".to_string(),

            // Repeated special chars
            "'''''''''''''''''''''''''''''''".to_string(),
            "\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"\"".to_string(),
            "//////////////////////////////".to_string(),
            "\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\\".to_string(),
        ]
    }
}

impl Default for PayloadDatabase {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payload_database_creation() {
        let db = PayloadDatabase::new();
        assert!(!db.sql_injection().is_empty());
        assert!(!db.identifier_injection().is_empty());
        assert!(!db.unicode_tricks().is_empty());
        assert!(!db.overflow().is_empty());
    }

    #[test]
    fn test_sql_injection_payloads_count() {
        let db = PayloadDatabase::new();
        let payloads = db.sql_injection();
        assert!(payloads.len() >= 50, "Should have at least 50 SQL injection payloads");
    }

    #[test]
    fn test_identifier_injection_payloads_count() {
        let db = PayloadDatabase::new();
        let payloads = db.identifier_injection();
        assert!(payloads.len() >= 30, "Should have at least 30 identifier injection payloads");
    }

    #[test]
    fn test_unicode_tricks_payloads_count() {
        let db = PayloadDatabase::new();
        let payloads = db.unicode_tricks();
        assert!(payloads.len() >= 20, "Should have at least 20 unicode trick payloads");
    }

    #[test]
    fn test_overflow_payloads_count() {
        let db = PayloadDatabase::new();
        let payloads = db.overflow();
        assert!(payloads.len() >= 10, "Should have at least 10 overflow payloads");
    }

    #[test]
    fn test_by_category() {
        let db = PayloadDatabase::new();

        let sql = db.by_category(PayloadCategory::SqlInjection);
        assert_eq!(sql.len(), db.sql_injection().len());

        let identifier = db.by_category(PayloadCategory::IdentifierInjection);
        assert_eq!(identifier.len(), db.identifier_injection().len());

        let unicode = db.by_category(PayloadCategory::UnicodeTricks);
        assert_eq!(unicode.len(), db.unicode_tricks().len());

        let overflow = db.by_category(PayloadCategory::Overflow);
        assert_eq!(overflow.len(), db.overflow().len());
    }

    #[test]
    fn test_all_payloads() {
        let db = PayloadDatabase::new();
        let all = db.all();

        let expected_total = db.sql_injection().len()
            + db.identifier_injection().len()
            + db.unicode_tricks().len()
            + db.overflow().len();

        assert_eq!(all.len(), expected_total);
    }

    #[test]
    fn test_sql_injection_contains_classic_attacks() {
        let db = PayloadDatabase::new();
        let payloads = db.sql_injection();

        // Check for classic SQL injection patterns
        assert!(payloads.iter().any(|p| p.contains("OR '1'='1")));
        assert!(payloads.iter().any(|p| p.contains("DROP TABLE")));
        assert!(payloads.iter().any(|p| p.contains("UNION SELECT")));
    }

    #[test]
    fn test_nosql_injection_payloads() {
        let db = PayloadDatabase::new();
        let payloads = db.sql_injection();

        // Check for NoSQL injection patterns
        assert!(payloads.iter().any(|p| p.contains("$gt")));
        assert!(payloads.iter().any(|p| p.contains("$ne")));
        assert!(payloads.iter().any(|p| p.contains("$where")));
    }

    #[test]
    fn test_identifier_contains_schema_attacks() {
        let db = PayloadDatabase::new();
        let payloads = db.identifier_injection();

        assert!(payloads.iter().any(|p| p.contains("information_schema")));
        assert!(payloads.iter().any(|p| p.contains("$where")));
    }

    #[test]
    fn test_unicode_contains_homoglyphs() {
        let db = PayloadDatabase::new();
        let payloads = db.unicode_tricks();

        // Check for zero-width characters
        assert!(payloads.iter().any(|p| p.contains('\u{200B}')));
        // Check for RTL override
        assert!(payloads.iter().any(|p| p.contains('\u{202E}')));
    }

    #[test]
    fn test_overflow_contains_long_strings() {
        let db = PayloadDatabase::new();
        let payloads = db.overflow();

        // Check for very long strings
        assert!(payloads.iter().any(|p| p.len() > 10000));
    }

    #[test]
    fn test_payload_categories() {
        assert_eq!(PayloadCategory::SqlInjection, PayloadCategory::SqlInjection);
        assert_ne!(PayloadCategory::SqlInjection, PayloadCategory::IdentifierInjection);
    }

    #[test]
    fn test_default_implementation() {
        let db1 = PayloadDatabase::default();
        let db2 = PayloadDatabase::new();

        assert_eq!(db1.sql_injection().len(), db2.sql_injection().len());
        assert_eq!(db1.identifier_injection().len(), db2.identifier_injection().len());
    }
}
