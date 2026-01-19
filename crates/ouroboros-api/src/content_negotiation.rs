//! Content negotiation support
//!
//! Provides Accept header parsing and response format selection
//! based on client preferences.

use std::collections::HashMap;
use std::cmp::Ordering;

// ============================================================================
// Media Type
// ============================================================================

/// Parsed media type
#[derive(Debug, Clone, PartialEq)]
pub struct MediaType {
    /// Main type (e.g., "application", "text", "*")
    pub r#type: String,
    /// Subtype (e.g., "json", "html", "*")
    pub subtype: String,
    /// Parameters (e.g., "charset=utf-8")
    pub params: HashMap<String, String>,
    /// Quality value (0.0 - 1.0)
    pub quality: f32,
}

impl MediaType {
    /// Parse a media type string
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let (type_part, params) = if let Some((t, p)) = s.split_once(';') {
            (t.trim(), Some(p))
        } else {
            (s, None)
        };

        let (r#type, subtype) = type_part.split_once('/')?;
        let r#type = r#type.trim().to_lowercase();
        let subtype = subtype.trim().to_lowercase();

        let mut media_type = Self {
            r#type,
            subtype,
            params: HashMap::new(),
            quality: 1.0,
        };

        // Parse parameters
        if let Some(param_str) = params {
            for param in param_str.split(';') {
                let param = param.trim();
                if let Some((key, value)) = param.split_once('=') {
                    let key = key.trim().to_lowercase();
                    let value = value.trim().trim_matches('"').to_string();

                    if key == "q" {
                        media_type.quality = value.parse().unwrap_or(1.0);
                    } else {
                        media_type.params.insert(key, value);
                    }
                }
            }
        }

        Some(media_type)
    }

    /// Create a specific media type
    pub fn new(r#type: impl Into<String>, subtype: impl Into<String>) -> Self {
        Self {
            r#type: r#type.into(),
            subtype: subtype.into(),
            params: HashMap::new(),
            quality: 1.0,
        }
    }

    /// Common media types
    pub fn json() -> Self {
        Self::new("application", "json")
    }

    pub fn xml() -> Self {
        Self::new("application", "xml")
    }

    pub fn html() -> Self {
        Self::new("text", "html")
    }

    pub fn text() -> Self {
        Self::new("text", "plain")
    }

    pub fn form() -> Self {
        Self::new("application", "x-www-form-urlencoded")
    }

    pub fn multipart() -> Self {
        Self::new("multipart", "form-data")
    }

    pub fn octet_stream() -> Self {
        Self::new("application", "octet-stream")
    }

    pub fn any() -> Self {
        Self::new("*", "*")
    }

    /// Set quality value
    pub fn quality(mut self, q: f32) -> Self {
        self.quality = q.clamp(0.0, 1.0);
        self
    }

    /// Add parameter
    pub fn param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    /// Check if this is a wildcard type
    pub fn is_wildcard(&self) -> bool {
        self.r#type == "*" || self.subtype == "*"
    }

    /// Check if this media type matches another
    pub fn matches(&self, other: &MediaType) -> bool {
        let type_matches = self.r#type == "*" || other.r#type == "*" || self.r#type == other.r#type;
        let subtype_matches =
            self.subtype == "*" || other.subtype == "*" || self.subtype == other.subtype;
        type_matches && subtype_matches
    }

    /// Get specificity score (higher = more specific)
    pub fn specificity(&self) -> u8 {
        match (&self.r#type[..], &self.subtype[..]) {
            ("*", "*") => 0,
            ("*", _) => 1,
            (_, "*") => 2,
            _ => 3,
        }
    }

    /// Convert to string
    pub fn to_string(&self) -> String {
        let mut result = format!("{}/{}", self.r#type, self.subtype);

        for (key, value) in &self.params {
            result.push_str(&format!("; {}={}", key, value));
        }

        if self.quality < 1.0 {
            result.push_str(&format!("; q={:.1}", self.quality));
        }

        result
    }
}

impl Eq for MediaType {}

impl PartialOrd for MediaType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MediaType {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher quality first
        other.quality.partial_cmp(&self.quality)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                // Higher specificity first
                other.specificity().cmp(&self.specificity())
            })
    }
}

// ============================================================================
// Accept Header
// ============================================================================

/// Parsed Accept header
#[derive(Debug, Clone)]
pub struct AcceptHeader {
    /// Media types in order of preference
    pub media_types: Vec<MediaType>,
}

impl AcceptHeader {
    /// Parse an Accept header value
    pub fn parse(header: &str) -> Self {
        let mut media_types: Vec<MediaType> = header
            .split(',')
            .filter_map(|s| MediaType::parse(s))
            .collect();

        // Sort by preference (quality and specificity)
        media_types.sort();

        Self { media_types }
    }

    /// Create from a single media type
    pub fn single(media_type: MediaType) -> Self {
        Self {
            media_types: vec![media_type],
        }
    }

    /// Get the preferred media type
    pub fn preferred(&self) -> Option<&MediaType> {
        self.media_types.first()
    }

    /// Check if a media type is acceptable
    pub fn accepts(&self, media_type: &MediaType) -> bool {
        self.media_types.iter().any(|m| m.matches(media_type))
    }

    /// Select the best match from available options
    pub fn select<'a>(&self, available: &'a [MediaType]) -> Option<&'a MediaType> {
        // For each preference in order, find a match
        for preference in &self.media_types {
            for available_type in available {
                if preference.matches(available_type) {
                    return Some(available_type);
                }
            }
        }

        // If no match found but */* is accepted, return first available
        if self.accepts(&MediaType::any()) && !available.is_empty() {
            return Some(&available[0]);
        }

        None
    }

    /// Get quality for a specific media type
    pub fn quality_for(&self, media_type: &MediaType) -> f32 {
        for m in &self.media_types {
            if m.matches(media_type) {
                return m.quality;
            }
        }
        0.0
    }
}

impl Default for AcceptHeader {
    fn default() -> Self {
        Self {
            media_types: vec![MediaType::any()],
        }
    }
}

// ============================================================================
// Content Negotiator
// ============================================================================

/// Format handler function type
pub type FormatHandler = Box<dyn Fn(&[u8]) -> Vec<u8> + Send + Sync>;

/// Content negotiator for response format selection
pub struct ContentNegotiator {
    /// Supported formats
    formats: Vec<SupportedFormat>,
    /// Default format
    default: MediaType,
}

/// A supported format
struct SupportedFormat {
    media_type: MediaType,
}

impl ContentNegotiator {
    /// Create a new content negotiator
    pub fn new() -> Self {
        Self {
            formats: vec![
                SupportedFormat {
                    media_type: MediaType::json(),
                },
            ],
            default: MediaType::json(),
        }
    }

    /// Add a supported format
    pub fn format(mut self, media_type: MediaType) -> Self {
        self.formats.push(SupportedFormat { media_type });
        self
    }

    /// Set default format
    pub fn default_format(mut self, media_type: MediaType) -> Self {
        self.default = media_type;
        self
    }

    /// Add JSON format
    pub fn with_json(self) -> Self {
        self.format(MediaType::json())
    }

    /// Add XML format
    pub fn with_xml(self) -> Self {
        self.format(MediaType::xml())
    }

    /// Add HTML format
    pub fn with_html(self) -> Self {
        self.format(MediaType::html())
    }

    /// Add plain text format
    pub fn with_text(self) -> Self {
        self.format(MediaType::text())
    }

    /// Get available media types
    pub fn available(&self) -> Vec<MediaType> {
        self.formats.iter().map(|f| f.media_type.clone()).collect()
    }

    /// Negotiate best format for request
    pub fn negotiate(&self, accept_header: Option<&str>) -> NegotiationResult {
        let accept = match accept_header {
            Some(h) => AcceptHeader::parse(h),
            None => AcceptHeader::default(),
        };

        let available = self.available();
        match accept.select(&available) {
            Some(media_type) => NegotiationResult::Matched(media_type.clone()),
            None => {
                if !available.is_empty() {
                    NegotiationResult::NotAcceptable(available)
                } else {
                    NegotiationResult::Matched(self.default.clone())
                }
            }
        }
    }
}

impl Default for ContentNegotiator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of content negotiation
#[derive(Debug, Clone)]
pub enum NegotiationResult {
    /// Found a matching format
    Matched(MediaType),
    /// No acceptable format found, includes available formats
    NotAcceptable(Vec<MediaType>),
}

impl NegotiationResult {
    /// Check if negotiation succeeded
    pub fn is_match(&self) -> bool {
        matches!(self, NegotiationResult::Matched(_))
    }

    /// Get the matched media type
    pub fn media_type(&self) -> Option<&MediaType> {
        match self {
            NegotiationResult::Matched(m) => Some(m),
            NegotiationResult::NotAcceptable(_) => None,
        }
    }

    /// Get the content type header value
    pub fn content_type(&self) -> Option<String> {
        self.media_type().map(|m| m.to_string())
    }
}

// ============================================================================
// Accept-Language
// ============================================================================

/// Language tag with quality
#[derive(Debug, Clone, PartialEq)]
pub struct LanguageTag {
    /// Primary language (e.g., "en", "zh")
    pub primary: String,
    /// Region/script (e.g., "US", "Hans")
    pub region: Option<String>,
    /// Quality value
    pub quality: f32,
}

impl LanguageTag {
    /// Parse a language tag
    pub fn parse(s: &str) -> Option<Self> {
        let (tag, quality) = if let Some((t, q)) = s.split_once(";q=") {
            (t.trim(), q.parse().unwrap_or(1.0))
        } else {
            (s.trim(), 1.0)
        };

        let (primary, region) = if let Some((p, r)) = tag.split_once('-') {
            (p.to_lowercase(), Some(r.to_string()))
        } else {
            (tag.to_lowercase(), None)
        };

        if primary.is_empty() {
            return None;
        }

        Some(Self {
            primary,
            region,
            quality,
        })
    }

    /// Check if this matches another language tag
    pub fn matches(&self, other: &LanguageTag) -> bool {
        if self.primary == "*" || other.primary == "*" {
            return true;
        }

        if self.primary != other.primary {
            return false;
        }

        // If regions are specified, they must match
        match (&self.region, &other.region) {
            (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
            (None, _) | (_, None) => true,
        }
    }
}

/// Parsed Accept-Language header
#[derive(Debug, Clone)]
pub struct AcceptLanguage {
    /// Language tags in order of preference
    pub languages: Vec<LanguageTag>,
}

impl AcceptLanguage {
    /// Parse an Accept-Language header
    pub fn parse(header: &str) -> Self {
        let mut languages: Vec<LanguageTag> = header
            .split(',')
            .filter_map(|s| LanguageTag::parse(s))
            .collect();

        languages.sort_by(|a, b| {
            b.quality.partial_cmp(&a.quality).unwrap_or(Ordering::Equal)
        });

        Self { languages }
    }

    /// Get preferred language
    pub fn preferred(&self) -> Option<&LanguageTag> {
        self.languages.first()
    }

    /// Select best match from available languages
    pub fn select<'a>(&self, available: &'a [&str]) -> Option<&'a str> {
        for pref in &self.languages {
            for lang in available {
                if let Some(tag) = LanguageTag::parse(lang) {
                    if pref.matches(&tag) {
                        return Some(lang);
                    }
                }
            }
        }
        None
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_type_parse() {
        let mt = MediaType::parse("application/json").unwrap();
        assert_eq!(mt.r#type, "application");
        assert_eq!(mt.subtype, "json");
        assert_eq!(mt.quality, 1.0);

        let mt = MediaType::parse("text/html; charset=utf-8; q=0.9").unwrap();
        assert_eq!(mt.r#type, "text");
        assert_eq!(mt.subtype, "html");
        assert_eq!(mt.params.get("charset"), Some(&"utf-8".to_string()));
        assert_eq!(mt.quality, 0.9);
    }

    #[test]
    fn test_media_type_matches() {
        let json = MediaType::json();
        let any = MediaType::any();
        let xml = MediaType::xml();

        assert!(any.matches(&json));
        assert!(json.matches(&any));
        assert!(!json.matches(&xml));
    }

    #[test]
    fn test_accept_header_parse() {
        let accept = AcceptHeader::parse("application/json, text/html;q=0.9, */*;q=0.8");
        assert_eq!(accept.media_types.len(), 3);
        assert_eq!(accept.preferred().unwrap().subtype, "json");
    }

    #[test]
    fn test_accept_header_select() {
        let accept = AcceptHeader::parse("text/html, application/json;q=0.9");
        let available = vec![MediaType::json(), MediaType::xml()];

        let selected = accept.select(&available);
        assert_eq!(selected.unwrap().subtype, "json");
    }

    #[test]
    fn test_content_negotiator() {
        let negotiator = ContentNegotiator::new()
            .with_json()
            .with_xml()
            .with_html();

        // JSON preferred
        let result = negotiator.negotiate(Some("application/json, text/html"));
        assert!(result.is_match());
        assert_eq!(result.media_type().unwrap().subtype, "json");

        // HTML preferred
        let result = negotiator.negotiate(Some("text/html, application/json;q=0.5"));
        assert!(result.is_match());
        assert_eq!(result.media_type().unwrap().subtype, "html");

        // No Accept header - use default
        let result = negotiator.negotiate(None);
        assert!(result.is_match());
    }

    #[test]
    fn test_language_tag() {
        let tag = LanguageTag::parse("en-US;q=0.9").unwrap();
        assert_eq!(tag.primary, "en");
        assert_eq!(tag.region, Some("US".to_string()));
        assert_eq!(tag.quality, 0.9);

        let tag2 = LanguageTag::parse("en").unwrap();
        assert!(tag.matches(&tag2));
    }

    #[test]
    fn test_accept_language() {
        let accept = AcceptLanguage::parse("en-US, en;q=0.9, zh-CN;q=0.8");
        assert_eq!(accept.preferred().unwrap().primary, "en");

        let available = &["zh-CN", "en-GB", "fr"];
        let selected = accept.select(available);
        assert_eq!(selected, Some(&"en-GB"));
    }

    #[test]
    fn test_media_type_specificity() {
        assert!(MediaType::json().specificity() > MediaType::any().specificity());
        assert!(MediaType::new("text", "*").specificity() > MediaType::any().specificity());
    }
}
