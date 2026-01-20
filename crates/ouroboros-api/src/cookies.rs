//! Cookie management support
//!
//! Provides cookie parsing, creation, and secure cookie options.
//! Supports HttpOnly, Secure, SameSite attributes, and cookie signing.

use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use hmac::{Hmac, Mac};
use sha2::Sha256;

// ============================================================================
// Cookie Configuration
// ============================================================================

/// Same-site attribute for cookies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SameSite {
    /// Cookie sent with same-site requests and cross-site top-level navigations
    #[default]
    Lax,
    /// Cookie only sent with same-site requests
    Strict,
    /// Cookie sent with all requests (requires Secure)
    None,
}

impl SameSite {
    fn as_str(&self) -> &'static str {
        match self {
            SameSite::Lax => "Lax",
            SameSite::Strict => "Strict",
            SameSite::None => "None",
        }
    }
}

/// Cookie configuration
#[derive(Debug, Clone)]
pub struct Cookie {
    /// Cookie name
    pub name: String,
    /// Cookie value
    pub value: String,
    /// Domain for the cookie
    pub domain: Option<String>,
    /// Path for the cookie
    pub path: Option<String>,
    /// Maximum age in seconds
    pub max_age: Option<i64>,
    /// Expiration timestamp
    pub expires: Option<SystemTime>,
    /// HTTP-only flag (not accessible via JavaScript)
    pub http_only: bool,
    /// Secure flag (only sent over HTTPS)
    pub secure: bool,
    /// Same-site attribute
    pub same_site: SameSite,
    /// Partitioned cookie (CHIPS)
    pub partitioned: bool,
}

impl Cookie {
    /// Create a new cookie with name and value
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            domain: None,
            path: Some("/".to_string()),
            max_age: None,
            expires: None,
            http_only: false,
            secure: false,
            same_site: SameSite::Lax,
            partitioned: false,
        }
    }

    /// Create a session cookie (no expiration, deleted when browser closes)
    pub fn session(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(name, value)
    }

    /// Create a persistent cookie with max age
    pub fn persistent(name: impl Into<String>, value: impl Into<String>, max_age: Duration) -> Self {
        Self::new(name, value).max_age(max_age.as_secs() as i64)
    }

    /// Create a secure cookie (HttpOnly + Secure + SameSite=Strict)
    pub fn secure_cookie(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(name, value)
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Strict)
    }

    /// Set domain
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Set path
    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Set max age in seconds
    pub fn max_age(mut self, seconds: i64) -> Self {
        self.max_age = Some(seconds);
        self
    }

    /// Set max age from duration
    pub fn max_age_duration(mut self, duration: Duration) -> Self {
        self.max_age = Some(duration.as_secs() as i64);
        self
    }

    /// Set expiration time
    pub fn expires(mut self, time: SystemTime) -> Self {
        self.expires = Some(time);
        self
    }

    /// Set expires from duration from now
    pub fn expires_in(mut self, duration: Duration) -> Self {
        self.expires = Some(SystemTime::now() + duration);
        self
    }

    /// Set HTTP-only flag
    pub fn http_only(mut self, http_only: bool) -> Self {
        self.http_only = http_only;
        self
    }

    /// Set secure flag
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    /// Set same-site attribute
    pub fn same_site(mut self, same_site: SameSite) -> Self {
        self.same_site = same_site;
        self
    }

    /// Set partitioned flag (CHIPS)
    pub fn partitioned(mut self, partitioned: bool) -> Self {
        self.partitioned = partitioned;
        self
    }

    /// Check if cookie is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires) = self.expires {
            return expires < SystemTime::now();
        }
        if let Some(max_age) = self.max_age {
            return max_age <= 0;
        }
        false
    }

    /// Convert to Set-Cookie header value
    pub fn to_header_value(&self) -> String {
        let mut parts = vec![format!("{}={}", self.name, encode_cookie_value(&self.value))];

        if let Some(ref domain) = self.domain {
            parts.push(format!("Domain={}", domain));
        }

        if let Some(ref path) = self.path {
            parts.push(format!("Path={}", path));
        }

        if let Some(max_age) = self.max_age {
            parts.push(format!("Max-Age={}", max_age));
        }

        if let Some(expires) = self.expires {
            parts.push(format!("Expires={}", format_http_date(expires)));
        }

        if self.http_only {
            parts.push("HttpOnly".to_string());
        }

        if self.secure {
            parts.push("Secure".to_string());
        }

        parts.push(format!("SameSite={}", self.same_site.as_str()));

        if self.partitioned {
            parts.push("Partitioned".to_string());
        }

        parts.join("; ")
    }

    /// Create a cookie that deletes this cookie
    pub fn deletion_cookie(&self) -> Cookie {
        Cookie::new(&self.name, "")
            .max_age(0)
            .path(self.path.clone().unwrap_or_else(|| "/".to_string()))
    }
}

// ============================================================================
// Cookie Jar
// ============================================================================

/// Collection of cookies
#[derive(Debug, Clone, Default)]
pub struct CookieJar {
    cookies: HashMap<String, Cookie>,
}

impl CookieJar {
    /// Create a new empty cookie jar
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse cookies from a Cookie header
    pub fn from_header(header: &str) -> Self {
        let mut jar = Self::new();
        for pair in header.split(';') {
            let pair = pair.trim();
            if let Some((name, value)) = pair.split_once('=') {
                let cookie = Cookie::new(
                    name.trim(),
                    decode_cookie_value(value.trim()),
                );
                jar.add(cookie);
            }
        }
        jar
    }

    /// Add a cookie to the jar
    pub fn add(&mut self, cookie: Cookie) {
        self.cookies.insert(cookie.name.clone(), cookie);
    }

    /// Get a cookie by name
    pub fn get(&self, name: &str) -> Option<&Cookie> {
        self.cookies.get(name)
    }

    /// Get cookie value by name
    pub fn get_value(&self, name: &str) -> Option<&str> {
        self.cookies.get(name).map(|c| c.value.as_str())
    }

    /// Remove a cookie by name
    pub fn remove(&mut self, name: &str) -> Option<Cookie> {
        self.cookies.remove(name)
    }

    /// Check if cookie exists
    pub fn contains(&self, name: &str) -> bool {
        self.cookies.contains_key(name)
    }

    /// Get all cookies
    pub fn iter(&self) -> impl Iterator<Item = &Cookie> {
        self.cookies.values()
    }

    /// Get cookie names
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.cookies.keys().map(|s| s.as_str())
    }

    /// Number of cookies
    pub fn len(&self) -> usize {
        self.cookies.len()
    }

    /// Check if jar is empty
    pub fn is_empty(&self) -> bool {
        self.cookies.is_empty()
    }

    /// Clear all cookies
    pub fn clear(&mut self) {
        self.cookies.clear();
    }
}

// ============================================================================
// Signed Cookies
// ============================================================================

type HmacSha256 = Hmac<Sha256>;

/// Cookie signer for secure cookie signing
pub struct CookieSigner {
    secret: Vec<u8>,
}

impl CookieSigner {
    /// Create a new cookie signer with the given secret
    pub fn new(secret: impl AsRef<[u8]>) -> Self {
        Self {
            secret: secret.as_ref().to_vec(),
        }
    }

    /// Sign a cookie value
    pub fn sign(&self, value: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .expect("HMAC can take key of any size");
        mac.update(value.as_bytes());
        let signature = mac.finalize().into_bytes();
        let sig_b64 = base64::encode(&signature);
        format!("{}|{}", value, sig_b64)
    }

    /// Verify and extract a signed cookie value
    pub fn verify(&self, signed_value: &str) -> Option<String> {
        let (value, sig_b64) = signed_value.rsplit_once('|')?;
        let signature = base64::decode(sig_b64).ok()?;

        let mut mac = HmacSha256::new_from_slice(&self.secret)
            .expect("HMAC can take key of any size");
        mac.update(value.as_bytes());

        mac.verify_slice(&signature).ok()?;
        Some(value.to_string())
    }

    /// Create a signed cookie
    pub fn sign_cookie(&self, cookie: Cookie) -> Cookie {
        let signed_value = self.sign(&cookie.value);
        Cookie { value: signed_value, ..cookie }
    }

    /// Verify a signed cookie and return the original value
    pub fn verify_cookie(&self, cookie: &Cookie) -> Option<String> {
        self.verify(&cookie.value)
    }
}

// ============================================================================
// Response Extension
// ============================================================================

/// Cookies to be set in response
#[derive(Debug, Clone, Default)]
pub struct ResponseCookies {
    /// Cookies to set
    pub to_set: Vec<Cookie>,
    /// Cookie names to delete
    pub to_delete: Vec<String>,
}

impl ResponseCookies {
    /// Create new response cookies
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a cookie to set
    pub fn set(&mut self, cookie: Cookie) {
        self.to_set.push(cookie);
    }

    /// Delete a cookie by name
    pub fn delete(&mut self, name: impl Into<String>) {
        self.to_delete.push(name.into());
    }

    /// Get all Set-Cookie header values
    pub fn to_headers(&self) -> Vec<String> {
        let mut headers = Vec::new();

        for cookie in &self.to_set {
            headers.push(cookie.to_header_value());
        }

        for name in &self.to_delete {
            let delete_cookie = Cookie::new(name, "").max_age(0).path("/");
            headers.push(delete_cookie.to_header_value());
        }

        headers
    }

    /// Check if there are any cookies to set or delete
    pub fn is_empty(&self) -> bool {
        self.to_set.is_empty() && self.to_delete.is_empty()
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Encode a cookie value (URL-encode special characters)
fn encode_cookie_value(value: &str) -> String {
    // Simple encoding - just handle the most problematic characters
    value
        .replace('%', "%25")
        .replace(';', "%3B")
        .replace(',', "%2C")
        .replace(' ', "%20")
        .replace('"', "%22")
}

/// Decode a cookie value
fn decode_cookie_value(value: &str) -> String {
    // Remove surrounding quotes if present
    let value = value.trim_matches('"');

    // Simple decoding
    value
        .replace("%3B", ";")
        .replace("%2C", ",")
        .replace("%20", " ")
        .replace("%22", "\"")
        .replace("%25", "%")
}

/// Format a SystemTime as HTTP date
fn format_http_date(time: SystemTime) -> String {
    let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = duration.as_secs();

    // Convert to date components (simplified - doesn't handle leap seconds)
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Days since epoch to date (simplified algorithm)
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    // Day of week
    let dow = ((days + 4) % 7) as usize;
    let day_names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let month_names = ["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                       "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

    format!(
        "{}, {:02} {} {} {:02}:{:02}:{:02} GMT",
        day_names[dow],
        d,
        month_names[(m - 1) as usize],
        y,
        hours,
        minutes,
        seconds
    )
}

/// Simple base64 encoding (no external dependencies)
mod base64 {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    pub fn encode(data: &[u8]) -> String {
        let mut result = String::new();
        let chunks = data.chunks(3);

        for chunk in chunks {
            let b0 = chunk[0] as usize;
            let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
            let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

            result.push(ALPHABET[b0 >> 2] as char);
            result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

            if chunk.len() > 1 {
                result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
            } else {
                result.push('=');
            }

            if chunk.len() > 2 {
                result.push(ALPHABET[b2 & 0x3f] as char);
            } else {
                result.push('=');
            }
        }

        result
    }

    pub fn decode(data: &str) -> Result<Vec<u8>, ()> {
        let data = data.trim_end_matches('=');
        let mut result = Vec::new();

        fn decode_char(c: char) -> Result<u8, ()> {
            match c {
                'A'..='Z' => Ok(c as u8 - b'A'),
                'a'..='z' => Ok(c as u8 - b'a' + 26),
                '0'..='9' => Ok(c as u8 - b'0' + 52),
                '+' => Ok(62),
                '/' => Ok(63),
                _ => Err(()),
            }
        }

        let chars: Vec<u8> = data.chars().map(decode_char).collect::<Result<_, _>>()?;

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
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cookie_basic() {
        let cookie = Cookie::new("session", "abc123");
        assert_eq!(cookie.name, "session");
        assert_eq!(cookie.value, "abc123");
        assert_eq!(cookie.path, Some("/".to_string()));
    }

    #[test]
    fn test_cookie_builder() {
        let cookie = Cookie::new("token", "xyz")
            .domain("example.com")
            .path("/api")
            .max_age(3600)
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Strict);

        assert_eq!(cookie.domain, Some("example.com".to_string()));
        assert_eq!(cookie.path, Some("/api".to_string()));
        assert_eq!(cookie.max_age, Some(3600));
        assert!(cookie.http_only);
        assert!(cookie.secure);
        assert_eq!(cookie.same_site, SameSite::Strict);
    }

    #[test]
    fn test_cookie_to_header() {
        let cookie = Cookie::new("session", "abc123")
            .max_age(3600)
            .http_only(true)
            .secure(true);

        let header = cookie.to_header_value();
        assert!(header.contains("session=abc123"));
        assert!(header.contains("Max-Age=3600"));
        assert!(header.contains("HttpOnly"));
        assert!(header.contains("Secure"));
    }

    #[test]
    fn test_cookie_jar_parse() {
        let header = "session=abc123; user=john; theme=dark";
        let jar = CookieJar::from_header(header);

        assert_eq!(jar.len(), 3);
        assert_eq!(jar.get_value("session"), Some("abc123"));
        assert_eq!(jar.get_value("user"), Some("john"));
        assert_eq!(jar.get_value("theme"), Some("dark"));
    }

    #[test]
    fn test_cookie_jar_operations() {
        let mut jar = CookieJar::new();
        jar.add(Cookie::new("a", "1"));
        jar.add(Cookie::new("b", "2"));

        assert!(jar.contains("a"));
        assert!(!jar.contains("c"));
        assert_eq!(jar.len(), 2);

        jar.remove("a");
        assert!(!jar.contains("a"));
        assert_eq!(jar.len(), 1);
    }

    #[test]
    fn test_cookie_signer() {
        let signer = CookieSigner::new("secret-key-123");

        let signed = signer.sign("user:john");
        assert!(signed.contains("|"));

        let verified = signer.verify(&signed);
        assert_eq!(verified, Some("user:john".to_string()));

        // Tampered value should fail
        let tampered = signed.replace("john", "jane");
        assert_eq!(signer.verify(&tampered), None);
    }

    #[test]
    fn test_secure_cookie() {
        let cookie = Cookie::secure_cookie("token", "secret");
        assert!(cookie.http_only);
        assert!(cookie.secure);
        assert_eq!(cookie.same_site, SameSite::Strict);
    }

    #[test]
    fn test_response_cookies() {
        let mut cookies = ResponseCookies::new();
        cookies.set(Cookie::new("session", "abc"));
        cookies.delete("old_session");

        let headers = cookies.to_headers();
        assert_eq!(headers.len(), 2);
        assert!(headers[0].contains("session=abc"));
        assert!(headers[1].contains("old_session="));
        assert!(headers[1].contains("Max-Age=0"));
    }

    #[test]
    fn test_encode_decode() {
        let original = "hello; world, test";
        let encoded = encode_cookie_value(original);
        let decoded = decode_cookie_value(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_base64() {
        let data = b"Hello, World!";
        let encoded = base64::encode(data);
        let decoded = base64::decode(&encoded).unwrap();
        assert_eq!(decoded, data);
    }
}
