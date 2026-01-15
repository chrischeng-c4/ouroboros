//! Request data extractors
//!
//! Extractors pull typed data from requests (path params, query params, body, etc.).

use crate::request::Request;
use crate::error::{ApiResult, ApiError};
use serde::de::DeserializeOwned;

/// Path parameter extractor
pub struct Path<T> {
    pub inner: T,
}

impl<T> Path<T>
where
    T: DeserializeOwned,
{
    /// Extract path parameters from request
    pub fn from_request(req: &Request) -> ApiResult<Self> {
        // Convert HashMap<String, String> to JSON object
        let mut map = serde_json::Map::new();
        for (key, value) in &req.inner.path_params {
            map.insert(key.clone(), serde_json::Value::String(value.clone()));
        }
        let json_value = serde_json::Value::Object(map);

        // Deserialize to target type T
        let inner = serde_json::from_value(json_value).map_err(|e| {
            ApiError::BadRequest(format!("Invalid path parameters: {}", e))
        })?;

        Ok(Path { inner })
    }
}

/// Query parameter extractor
pub struct Query<T> {
    pub inner: T,
}

impl<T> Query<T>
where
    T: DeserializeOwned,
{
    /// Extract query parameters from request
    pub fn from_request(req: &Request) -> ApiResult<Self> {
        // Convert HashMap<String, SerializableValue> to JSON object
        let mut map = serde_json::Map::new();
        for (key, value) in &req.inner.query_params {
            map.insert(key.clone(), value.to_json());
        }
        let json_value = serde_json::Value::Object(map);

        // Deserialize to target type T
        let inner = serde_json::from_value(json_value).map_err(|e| {
            ApiError::BadRequest(format!("Invalid query parameters: {}", e))
        })?;

        Ok(Query { inner })
    }
}

/// JSON body extractor
pub struct Json<T> {
    pub inner: T,
}

impl<T> Json<T>
where
    T: DeserializeOwned,
{
    /// Extract JSON body from request
    pub fn from_request(req: &Request) -> ApiResult<Self> {
        let json_value = req.body_json().ok_or_else(|| {
            ApiError::BadRequest("Missing request body".to_string())
        })?;

        let inner = serde_json::from_value(json_value).map_err(|e| {
            ApiError::BadRequest(format!("Invalid JSON: {}", e))
        })?;

        Ok(Json { inner })
    }
}

/// Header extractor
pub struct Headers {
    inner: std::collections::HashMap<String, String>,
}

impl Headers {
    /// Extract headers from request
    pub fn from_request(req: &Request) -> ApiResult<Self> {
        Ok(Headers {
            inner: req.inner.headers.clone(),
        })
    }

    /// Get a header value
    pub fn get(&self, key: &str) -> Option<&String> {
        self.inner.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::{HttpMethod, SerializableRequest, SerializableValue};
    use serde::Deserialize;

    #[derive(Deserialize, Debug, PartialEq)]
    struct TestPathParams {
        id: String,
        name: String,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct TestQueryParams {
        limit: i64,
        offset: i64,
        active: bool,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct TestBody {
        title: String,
        count: i32,
    }

    #[test]
    fn test_path_extractor_success() {
        let req = SerializableRequest::new(HttpMethod::Get, "/users/123/alice")
            .with_path_param("id", "123")
            .with_path_param("name", "alice");
        let request = Request::new(req);

        let result = Path::<TestPathParams>::from_request(&request);
        assert!(result.is_ok());

        let path = result.unwrap();
        assert_eq!(path.inner.id, "123");
        assert_eq!(path.inner.name, "alice");
    }

    #[test]
    fn test_path_extractor_missing_params() {
        let req = SerializableRequest::new(HttpMethod::Get, "/users");
        let request = Request::new(req);

        let result = Path::<TestPathParams>::from_request(&request);
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, ApiError::BadRequest(_)));
        }
    }

    #[test]
    fn test_query_extractor_success() {
        let req = SerializableRequest::new(HttpMethod::Get, "/users")
            .with_query_param("limit", SerializableValue::Int(10))
            .with_query_param("offset", SerializableValue::Int(0))
            .with_query_param("active", SerializableValue::Bool(true));
        let request = Request::new(req);

        let result = Query::<TestQueryParams>::from_request(&request);
        assert!(result.is_ok());

        let query = result.unwrap();
        assert_eq!(query.inner.limit, 10);
        assert_eq!(query.inner.offset, 0);
        assert_eq!(query.inner.active, true);
    }

    #[test]
    fn test_query_extractor_type_conversion() {
        // Test automatic type conversion from String to Int
        let req = SerializableRequest::new(HttpMethod::Get, "/users")
            .with_query_param("limit", SerializableValue::String("10".to_string()))
            .with_query_param("offset", SerializableValue::Int(0))
            .with_query_param("active", SerializableValue::Bool(true));
        let request = Request::new(req);

        let result = Query::<TestQueryParams>::from_request(&request);
        // This should fail because serde cannot auto-convert string "10" to i64
        assert!(result.is_err());
    }

    #[test]
    fn test_query_extractor_invalid_type() {
        let req = SerializableRequest::new(HttpMethod::Get, "/users")
            .with_query_param("limit", SerializableValue::String("not_a_number".to_string()))
            .with_query_param("offset", SerializableValue::Int(0))
            .with_query_param("active", SerializableValue::Bool(true));
        let request = Request::new(req);

        let result = Query::<TestQueryParams>::from_request(&request);
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, ApiError::BadRequest(_)));
        }
    }

    #[test]
    fn test_query_extractor_empty_params() {
        let req = SerializableRequest::new(HttpMethod::Get, "/users");
        let request = Request::new(req);

        let result = Query::<TestQueryParams>::from_request(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_json_extractor_success() {
        let req = SerializableRequest::new(HttpMethod::Post, "/items")
            .with_body(SerializableValue::Object(vec![
                ("title".to_string(), SerializableValue::String("Test".to_string())),
                ("count".to_string(), SerializableValue::Int(42)),
            ]));
        let request = Request::new(req);

        let result = Json::<TestBody>::from_request(&request);
        assert!(result.is_ok());

        let json = result.unwrap();
        assert_eq!(json.inner.title, "Test");
        assert_eq!(json.inner.count, 42);
    }

    #[test]
    fn test_json_extractor_missing_body() {
        let req = SerializableRequest::new(HttpMethod::Post, "/items");
        let request = Request::new(req);

        let result = Json::<TestBody>::from_request(&request);
        assert!(result.is_err());

        if let Err(e) = result {
            assert!(matches!(e, ApiError::BadRequest(_)));
        }
    }

    #[test]
    fn test_headers_extractor() {
        let req = SerializableRequest::new(HttpMethod::Get, "/test")
            .with_header("Content-Type", "application/json")
            .with_header("Authorization", "Bearer token123");
        let request = Request::new(req);

        let result = Headers::from_request(&request);
        assert!(result.is_ok());

        let headers = result.unwrap();
        assert_eq!(headers.get("content-type"), Some(&"application/json".to_string()));
        assert_eq!(headers.get("authorization"), Some(&"Bearer token123".to_string()));
    }
}
