//! Security utilities for OAuth2 and JWT authentication
//!
//! Provides OAuth2 password bearer flow, JWT token handling,
//! and scope-based authorization utilities.

use std::collections::HashSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use crate::error::{ApiError, ApiResult};

// ============================================================================
// JWT Configuration
// ============================================================================

/// JWT algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum JwtAlgorithm {
    /// HMAC-SHA256
    #[default]
    HS256,
    /// HMAC-SHA384
    HS384,
    /// HMAC-SHA512
    HS512,
}

impl JwtAlgorithm {
    fn as_str(&self) -> &'static str {
        match self {
            JwtAlgorithm::HS256 => "HS256",
            JwtAlgorithm::HS384 => "HS384",
            JwtAlgorithm::HS512 => "HS512",
        }
    }
}

/// JWT configuration
#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Secret key for signing
    pub secret: Vec<u8>,
    /// Algorithm
    pub algorithm: JwtAlgorithm,
    /// Token issuer
    pub issuer: Option<String>,
    /// Token audience
    pub audience: Option<String>,
    /// Default expiration duration
    pub expiration: Duration,
}

impl JwtConfig {
    /// Create a new JWT configuration
    pub fn new(secret: impl AsRef<[u8]>) -> Self {
        Self {
            secret: secret.as_ref().to_vec(),
            algorithm: JwtAlgorithm::HS256,
            issuer: None,
            audience: None,
            expiration: Duration::from_secs(3600), // 1 hour default
        }
    }

    /// Set algorithm
    pub fn algorithm(mut self, alg: JwtAlgorithm) -> Self {
        self.algorithm = alg;
        self
    }

    /// Set issuer
    pub fn issuer(mut self, issuer: impl Into<String>) -> Self {
        self.issuer = Some(issuer.into());
        self
    }

    /// Set audience
    pub fn audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = Some(audience.into());
        self
    }

    /// Set expiration duration
    pub fn expiration(mut self, duration: Duration) -> Self {
        self.expiration = duration;
        self
    }
}

// ============================================================================
// JWT Claims
// ============================================================================

/// Standard JWT claims
#[derive(Debug, Clone)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Issued at timestamp
    pub iat: u64,
    /// Expiration timestamp
    pub exp: u64,
    /// Not before timestamp
    pub nbf: Option<u64>,
    /// Issuer
    pub iss: Option<String>,
    /// Audience
    pub aud: Option<String>,
    /// JWT ID (unique identifier)
    pub jti: Option<String>,
    /// Scopes/permissions
    pub scopes: HashSet<String>,
    /// Custom claims
    pub custom: Vec<(String, String)>,
}

impl JwtClaims {
    /// Create new claims for a subject
    pub fn new(subject: impl Into<String>) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            sub: subject.into(),
            iat: now,
            exp: now + 3600, // Default 1 hour
            nbf: None,
            iss: None,
            aud: None,
            jti: None,
            scopes: HashSet::new(),
            custom: Vec::new(),
        }
    }

    /// Set expiration (seconds from now)
    pub fn expires_in(mut self, seconds: u64) -> Self {
        self.exp = self.iat + seconds;
        self
    }

    /// Set expiration duration
    pub fn expires_in_duration(mut self, duration: Duration) -> Self {
        self.exp = self.iat + duration.as_secs();
        self
    }

    /// Set issuer
    pub fn issuer(mut self, issuer: impl Into<String>) -> Self {
        self.iss = Some(issuer.into());
        self
    }

    /// Set audience
    pub fn audience(mut self, audience: impl Into<String>) -> Self {
        self.aud = Some(audience.into());
        self
    }

    /// Add a scope
    pub fn scope(mut self, scope: impl Into<String>) -> Self {
        self.scopes.insert(scope.into());
        self
    }

    /// Add multiple scopes
    pub fn scopes(mut self, scopes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for scope in scopes {
            self.scopes.insert(scope.into());
        }
        self
    }

    /// Add a custom claim
    pub fn claim(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.push((key.into(), value.into()));
        self
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.exp < now
    }

    /// Check if token is not yet valid
    pub fn is_not_yet_valid(&self) -> bool {
        if let Some(nbf) = self.nbf {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            return nbf > now;
        }
        false
    }

    /// Check if token has a specific scope
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.contains(scope)
    }

    /// Check if token has all required scopes
    pub fn has_all_scopes(&self, scopes: &[&str]) -> bool {
        scopes.iter().all(|s| self.scopes.contains(*s))
    }

    /// Check if token has any of the required scopes
    pub fn has_any_scope(&self, scopes: &[&str]) -> bool {
        scopes.iter().any(|s| self.scopes.contains(*s))
    }

    /// Get a custom claim value
    pub fn get_claim(&self, key: &str) -> Option<&str> {
        self.custom
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

// ============================================================================
// JWT Token
// ============================================================================

type HmacSha256 = Hmac<Sha256>;

/// JWT token handler
pub struct JwtHandler {
    config: JwtConfig,
}

impl JwtHandler {
    /// Create a new JWT handler
    pub fn new(config: JwtConfig) -> Self {
        Self { config }
    }

    /// Create with just a secret
    pub fn with_secret(secret: impl AsRef<[u8]>) -> Self {
        Self::new(JwtConfig::new(secret))
    }

    /// Generate a JWT token
    pub fn generate(&self, claims: &JwtClaims) -> String {
        let header = self.encode_header();
        let payload = self.encode_payload(claims);
        let message = format!("{}.{}", header, payload);
        let signature = self.sign(&message);

        format!("{}.{}", message, signature)
    }

    /// Verify and decode a JWT token
    pub fn verify(&self, token: &str) -> ApiResult<JwtClaims> {
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return Err(ApiError::Unauthorized);
        }

        let message = format!("{}.{}", parts[0], parts[1]);
        let signature = parts[2];

        // Verify signature
        let expected_sig = self.sign(&message);
        if !constant_time_compare(&expected_sig, signature) {
            return Err(ApiError::Unauthorized);
        }

        // Decode payload
        let payload = base64_url_decode(parts[1])
            .map_err(|_| ApiError::Unauthorized)?;
        let payload_str = String::from_utf8(payload)
            .map_err(|_| ApiError::Unauthorized)?;

        // Parse claims
        let claims = self.parse_claims(&payload_str)?;

        // Validate claims
        if claims.is_expired() {
            return Err(ApiError::Unauthorized);
        }
        if claims.is_not_yet_valid() {
            return Err(ApiError::Unauthorized);
        }

        // Validate issuer
        if let Some(ref expected_iss) = self.config.issuer {
            if claims.iss.as_ref() != Some(expected_iss) {
                return Err(ApiError::Unauthorized);
            }
        }

        // Validate audience
        if let Some(ref expected_aud) = self.config.audience {
            if claims.aud.as_ref() != Some(expected_aud) {
                return Err(ApiError::Unauthorized);
            }
        }

        Ok(claims)
    }

    fn encode_header(&self) -> String {
        let header = format!(
            r#"{{"alg":"{}","typ":"JWT"}}"#,
            self.config.algorithm.as_str()
        );
        base64_url_encode(header.as_bytes())
    }

    fn encode_payload(&self, claims: &JwtClaims) -> String {
        let mut parts = vec![
            format!(r#""sub":"{}""#, claims.sub),
            format!(r#""iat":{}"#, claims.iat),
            format!(r#""exp":{}"#, claims.exp),
        ];

        if let Some(nbf) = claims.nbf {
            parts.push(format!(r#""nbf":{}"#, nbf));
        }
        if let Some(ref iss) = claims.iss {
            parts.push(format!(r#""iss":"{}""#, iss));
        }
        if let Some(ref aud) = claims.aud {
            parts.push(format!(r#""aud":"{}""#, aud));
        }
        if let Some(ref jti) = claims.jti {
            parts.push(format!(r#""jti":"{}""#, jti));
        }
        if !claims.scopes.is_empty() {
            let scopes: Vec<_> = claims.scopes.iter().map(|s| format!(r#""{}""#, s)).collect();
            parts.push(format!(r#""scopes":[{}]"#, scopes.join(",")));
        }
        for (key, value) in &claims.custom {
            parts.push(format!(r#""{}":"{}""#, key, value));
        }

        let payload = format!("{{{}}}", parts.join(","));
        base64_url_encode(payload.as_bytes())
    }

    fn sign(&self, message: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.config.secret)
            .expect("HMAC can take key of any size");
        mac.update(message.as_bytes());
        let result = mac.finalize().into_bytes();
        base64_url_encode(&result)
    }

    fn parse_claims(&self, json: &str) -> ApiResult<JwtClaims> {
        // Simple JSON parsing for claims
        let sub = extract_string_field(json, "sub")
            .ok_or(ApiError::Unauthorized)?;
        let iat = extract_number_field(json, "iat")
            .ok_or(ApiError::Unauthorized)?;
        let exp = extract_number_field(json, "exp")
            .ok_or(ApiError::Unauthorized)?;

        let claims = JwtClaims {
            sub,
            iat,
            exp,
            nbf: extract_number_field(json, "nbf"),
            iss: extract_string_field(json, "iss"),
            aud: extract_string_field(json, "aud"),
            jti: extract_string_field(json, "jti"),
            scopes: extract_string_array(json, "scopes"),
            custom: Vec::new(),
        };

        // Extract any additional custom claims we recognize
        // (In a real implementation, you'd want a proper JSON parser)

        Ok(claims)
    }
}

// ============================================================================
// OAuth2 Password Bearer
// ============================================================================

/// OAuth2 password bearer configuration
#[derive(Debug, Clone)]
pub struct OAuth2PasswordBearer {
    /// Token URL for authentication
    pub token_url: String,
    /// Required scopes
    pub scopes: HashSet<String>,
    /// Scheme name (default: "Bearer")
    pub scheme: String,
    /// Auto error on missing token
    pub auto_error: bool,
}

impl OAuth2PasswordBearer {
    /// Create a new OAuth2 password bearer
    pub fn new(token_url: impl Into<String>) -> Self {
        Self {
            token_url: token_url.into(),
            scopes: HashSet::new(),
            scheme: "Bearer".to_string(),
            auto_error: true,
        }
    }

    /// Add a required scope
    pub fn scope(mut self, scope: impl Into<String>) -> Self {
        self.scopes.insert(scope.into());
        self
    }

    /// Add multiple required scopes
    pub fn scopes(mut self, scopes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for scope in scopes {
            self.scopes.insert(scope.into());
        }
        self
    }

    /// Set scheme name
    pub fn scheme(mut self, scheme: impl Into<String>) -> Self {
        self.scheme = scheme.into();
        self
    }

    /// Set auto error behavior
    pub fn auto_error(mut self, auto_error: bool) -> Self {
        self.auto_error = auto_error;
        self
    }

    /// Extract token from Authorization header
    pub fn extract_token(&self, auth_header: Option<&str>) -> ApiResult<Option<String>> {
        match auth_header {
            Some(header) => {
                let prefix = format!("{} ", self.scheme);
                if header.starts_with(&prefix) {
                    Ok(Some(header[prefix.len()..].to_string()))
                } else if self.auto_error {
                    Err(ApiError::Unauthorized)
                } else {
                    Ok(None)
                }
            }
            None => {
                if self.auto_error {
                    Err(ApiError::Unauthorized)
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Validate token has required scopes
    pub fn validate_scopes(&self, claims: &JwtClaims) -> ApiResult<()> {
        if self.scopes.is_empty() {
            return Ok(());
        }

        let scopes: Vec<_> = self.scopes.iter().map(|s| s.as_str()).collect();
        if claims.has_all_scopes(&scopes) {
            Ok(())
        } else {
            Err(ApiError::Forbidden)
        }
    }
}

// ============================================================================
// Token Response
// ============================================================================

/// OAuth2 token response
#[derive(Debug, Clone)]
pub struct TokenResponse {
    /// Access token
    pub access_token: String,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Expiration in seconds
    pub expires_in: u64,
    /// Refresh token (optional)
    pub refresh_token: Option<String>,
    /// Granted scopes
    pub scope: Option<String>,
}

impl TokenResponse {
    /// Create a new token response
    pub fn new(access_token: impl Into<String>, expires_in: u64) -> Self {
        Self {
            access_token: access_token.into(),
            token_type: "Bearer".to_string(),
            expires_in,
            refresh_token: None,
            scope: None,
        }
    }

    /// Set refresh token
    pub fn refresh_token(mut self, token: impl Into<String>) -> Self {
        self.refresh_token = Some(token.into());
        self
    }

    /// Set scope
    pub fn scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    /// Convert to JSON string
    pub fn to_json(&self) -> String {
        let mut parts = vec![
            format!(r#""access_token":"{}""#, self.access_token),
            format!(r#""token_type":"{}""#, self.token_type),
            format!(r#""expires_in":{}"#, self.expires_in),
        ];

        if let Some(ref refresh) = self.refresh_token {
            parts.push(format!(r#""refresh_token":"{}""#, refresh));
        }
        if let Some(ref scope) = self.scope {
            parts.push(format!(r#""scope":"{}""#, scope));
        }

        format!("{{{}}}", parts.join(","))
    }
}

// ============================================================================
// API Key Authentication
// ============================================================================

/// API key location
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiKeyLocation {
    Header,
    Query,
    Cookie,
}

/// API key configuration
#[derive(Debug, Clone)]
pub struct ApiKey {
    /// Key name (header name, query param, or cookie name)
    pub name: String,
    /// Key location
    pub location: ApiKeyLocation,
    /// Auto error on missing key
    pub auto_error: bool,
}

impl ApiKey {
    /// Create API key from header
    pub fn header(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            location: ApiKeyLocation::Header,
            auto_error: true,
        }
    }

    /// Create API key from query parameter
    pub fn query(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            location: ApiKeyLocation::Query,
            auto_error: true,
        }
    }

    /// Create API key from cookie
    pub fn cookie(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            location: ApiKeyLocation::Cookie,
            auto_error: true,
        }
    }

    /// Set auto error behavior
    pub fn auto_error(mut self, auto_error: bool) -> Self {
        self.auto_error = auto_error;
        self
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Base64 URL-safe encoding
fn base64_url_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f] as char);
        }
    }

    result
}

/// Base64 URL-safe decoding
fn base64_url_decode(data: &str) -> Result<Vec<u8>, ()> {
    fn decode_char(c: char) -> Result<u8, ()> {
        match c {
            'A'..='Z' => Ok(c as u8 - b'A'),
            'a'..='z' => Ok(c as u8 - b'a' + 26),
            '0'..='9' => Ok(c as u8 - b'0' + 52),
            '-' => Ok(62),
            '_' => Ok(63),
            _ => Err(()),
        }
    }

    let chars: Vec<u8> = data.chars().map(decode_char).collect::<Result<_, _>>()?;
    let mut result = Vec::new();

    for chunk in chars.chunks(4) {
        if chunk.len() >= 2 {
            result.push((chunk[0] << 2) | (chunk[1] >> 4));
        }
        if chunk.len() >= 3 {
            result.push((chunk[1] << 4) | (chunk[2] >> 2));
        }
        if chunk.len() >= 4 {
            result.push((chunk[2] << 6) | chunk[3]);
        }
    }

    Ok(result)
}

/// Constant-time string comparison
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}

/// Extract string field from JSON (simple parser)
fn extract_string_field(json: &str, field: &str) -> Option<String> {
    let pattern = format!(r#""{}":"#, field);
    let start = json.find(&pattern)? + pattern.len();
    let rest = &json[start..];

    if let Some(stripped) = rest.strip_prefix('"') {
        let end = stripped.find('"')?;
        Some(stripped[..end].to_string())
    } else {
        None
    }
}

/// Extract number field from JSON (simple parser)
fn extract_number_field(json: &str, field: &str) -> Option<u64> {
    let pattern = format!(r#""{}":"#, field);
    let start = json.find(&pattern)? + pattern.len();
    let rest = &json[start..];

    let end = rest.find(|c: char| !c.is_ascii_digit()).unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// Extract string array from JSON (simple parser)
fn extract_string_array(json: &str, field: &str) -> HashSet<String> {
    let pattern = format!(r#""{}":["#, field);
    let mut result = HashSet::new();

    if let Some(start) = json.find(&pattern) {
        let start = start + pattern.len();
        if let Some(end) = json[start..].find(']') {
            let array_str = &json[start..start + end];
            for item in array_str.split(',') {
                let item = item.trim().trim_matches('"');
                if !item.is_empty() {
                    result.insert(item.to_string());
                }
            }
        }
    }

    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_claims() {
        let claims = JwtClaims::new("user123")
            .expires_in(3600)
            .issuer("myapp")
            .scope("read")
            .scope("write");

        assert_eq!(claims.sub, "user123");
        assert!(claims.has_scope("read"));
        assert!(claims.has_scope("write"));
        assert!(!claims.has_scope("admin"));
        assert!(claims.has_all_scopes(&["read", "write"]));
        assert!(!claims.has_all_scopes(&["read", "admin"]));
    }

    #[test]
    fn test_jwt_generate_verify() {
        let handler = JwtHandler::with_secret("my-secret-key-123");

        let claims = JwtClaims::new("user456")
            .expires_in(3600)
            .scope("api");

        let token = handler.generate(&claims);
        assert!(token.contains('.'));

        let verified = handler.verify(&token).unwrap();
        assert_eq!(verified.sub, "user456");
        assert!(verified.has_scope("api"));
    }

    #[test]
    fn test_jwt_invalid_signature() {
        let handler = JwtHandler::with_secret("secret1");
        let other_handler = JwtHandler::with_secret("secret2");

        let claims = JwtClaims::new("user");
        let token = handler.generate(&claims);

        assert!(other_handler.verify(&token).is_err());
    }

    #[test]
    fn test_jwt_expired() {
        let handler = JwtHandler::with_secret("secret");

        let mut claims = JwtClaims::new("user");
        claims.exp = claims.iat - 1; // Already expired

        let token = handler.generate(&claims);
        assert!(handler.verify(&token).is_err());
    }

    #[test]
    fn test_oauth2_password_bearer() {
        let oauth = OAuth2PasswordBearer::new("/token")
            .scope("read")
            .scope("write");

        // Valid token extraction
        let token = oauth.extract_token(Some("Bearer abc123")).unwrap();
        assert_eq!(token, Some("abc123".to_string()));

        // Missing header
        let result = oauth.extract_token(None);
        assert!(result.is_err());

        // Wrong scheme
        let result = oauth.extract_token(Some("Basic abc123"));
        assert!(result.is_err());
    }

    #[test]
    fn test_token_response() {
        let response = TokenResponse::new("token123", 3600)
            .refresh_token("refresh456")
            .scope("read write");

        let json = response.to_json();
        assert!(json.contains("token123"));
        assert!(json.contains("3600"));
        assert!(json.contains("refresh456"));
    }

    #[test]
    fn test_api_key() {
        let key = ApiKey::header("X-API-Key");
        assert_eq!(key.name, "X-API-Key");
        assert_eq!(key.location, ApiKeyLocation::Header);

        let key = ApiKey::query("api_key").auto_error(false);
        assert_eq!(key.location, ApiKeyLocation::Query);
        assert!(!key.auto_error);
    }

    #[test]
    fn test_base64_url() {
        let data = b"Hello, JWT!";
        let encoded = base64_url_encode(data);
        let decoded = base64_url_decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("hello", "hello"));
        assert!(!constant_time_compare("hello", "world"));
        assert!(!constant_time_compare("hello", "hell"));
    }
}
