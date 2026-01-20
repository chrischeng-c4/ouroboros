//! JWT Authentication Example
//!
//! This example demonstrates JWT (JSON Web Token) authentication in ouroboros-api,
//! including token creation, validation, and claims handling.
//!
//! Run with:
//! ```bash
//! cargo run --example auth_example -p ouroboros-api
//! ```

use ouroboros_api::security::{
    JwtConfig, JwtAlgorithm, JwtClaims, JwtHandler,
    OAuth2PasswordBearer, TokenResponse,
};
use std::time::Duration;

// ============================================================================
// JWT Configuration
// ============================================================================

fn demonstrate_jwt_config() {
    println!("1. JWT Configuration");
    println!("--------------------");

    // Basic configuration
    let config = JwtConfig::new("my-secret-key-at-least-32-bytes!");
    println!("  Basic config:");
    println!("    Algorithm: {:?}", config.algorithm);
    println!("    Expiration: {:?}", config.expiration);

    // Full configuration with builder pattern
    let config = JwtConfig::new("my-secret-key-at-least-32-bytes!")
        .algorithm(JwtAlgorithm::HS256)
        .issuer("my-app")
        .audience("my-api")
        .expiration(Duration::from_secs(7200)); // 2 hours

    println!("  Full config:");
    println!("    Algorithm: {:?}", config.algorithm);
    println!("    Issuer: {:?}", config.issuer);
    println!("    Audience: {:?}", config.audience);
    println!("    Expiration: {:?}", config.expiration);
    println!();
}

// ============================================================================
// JWT Claims
// ============================================================================

fn demonstrate_jwt_claims() {
    println!("2. JWT Claims");
    println!("-------------");

    // Create claims with scopes
    let claims = JwtClaims::new("user-123")
        .scope("read")
        .scope("write")
        .claim("role", "admin");

    println!("  Subject: {}", claims.sub);
    println!("  Scopes: {:?}", claims.scopes);
    println!("  Has scope 'read': {}", claims.has_scope("read"));
    println!("  Has scope 'delete': {}", claims.has_scope("delete"));
    println!("  Custom claim 'role': {:?}", claims.get_claim("role"));
    println!();
}

// ============================================================================
// JWT Handler
// ============================================================================

fn demonstrate_jwt_handler() {
    println!("3. JWT Token Creation & Validation");
    println!("-----------------------------------");

    let config = JwtConfig::new("my-super-secret-key-for-jwt-signing!")
        .issuer("auth-example")
        .expiration(Duration::from_secs(3600));

    let handler = JwtHandler::new(config);

    // Create claims
    let claims = JwtClaims::new("user-456")
        .scopes(["read", "write", "profile"]);

    // Generate token
    let token = handler.generate(&claims);
    println!("  Token generated successfully!");
    println!("  Token (first 50 chars): {}...", &token[..50.min(token.len())]);

    // Verify token
    match handler.verify(&token) {
        Ok(validated_claims) => {
            println!("  Token validated!");
            println!("    Subject: {}", validated_claims.sub);
            println!("    Scopes: {:?}", validated_claims.scopes);
            println!("    Expired: {}", validated_claims.is_expired());
        }
        Err(e) => println!("  Validation error: {:?}", e),
    }
    println!();
}

// ============================================================================
// OAuth2 Password Bearer
// ============================================================================

fn demonstrate_oauth2_bearer() {
    println!("4. OAuth2 Password Bearer");
    println!("-------------------------");

    // Create OAuth2 password bearer configuration
    let bearer = OAuth2PasswordBearer::new("/oauth/token")
        .scope("read")
        .scope("write")
        .auto_error(true);

    println!("  Token URL: {}", bearer.token_url);
    println!("  Required scopes: {:?}", bearer.scopes);
    println!("  Scheme: {}", bearer.scheme);
    println!("  Auto error: {}", bearer.auto_error);
    println!();

    // Token extraction examples
    println!("  Token extraction:");

    // Valid token
    let valid_header = Some("Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...");
    match bearer.extract_token(valid_header) {
        Ok(Some(token)) => println!("    Valid header -> Token: {}...", &token[..20.min(token.len())]),
        Ok(None) => println!("    Valid header -> No token"),
        Err(e) => println!("    Valid header -> Error: {:?}", e),
    }

    // Missing token
    let no_bearer = OAuth2PasswordBearer::new("/token").auto_error(false);
    match no_bearer.extract_token(None) {
        Ok(Some(token)) => println!("    Missing header (auto_error=false) -> Token: {}", token),
        Ok(None) => println!("    Missing header (auto_error=false) -> None (ok)"),
        Err(e) => println!("    Missing header -> Error: {:?}", e),
    }
    println!();
}

// ============================================================================
// Token Response
// ============================================================================

fn demonstrate_token_response() {
    println!("5. Token Response");
    println!("-----------------");

    // Create token response
    let response = TokenResponse::new("eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...", 3600)
        .refresh_token("refresh_token_here")
        .scope("read write");

    println!("  access_token: {}...", &response.access_token[..30.min(response.access_token.len())]);
    println!("  token_type: {}", response.token_type);
    println!("  expires_in: {}", response.expires_in);
    println!("  refresh_token: {:?}", response.refresh_token.as_ref().map(|_| "[PRESENT]"));
    println!("  scope: {:?}", response.scope);
    println!();

    // JSON output
    println!("  JSON response:");
    println!("  {}", response.to_json());
    println!();
}

// ============================================================================
// Authentication Middleware Pattern
// ============================================================================

fn demonstrate_auth_middleware_pattern() {
    println!("6. Authentication Middleware Pattern");
    println!("------------------------------------");

    println!("  Example middleware flow:");
    println!("  1. Extract 'Authorization: Bearer <token>' header");
    println!("  2. Validate JWT token");
    println!("  3. Extract claims and attach to request");
    println!("  4. Handler can access claims for authorization");
    println!();

    println!("  Code pattern:");
    println!(r#"
    async fn auth_middleware(req: &mut Request) -> ApiResult<()> {{
        let bearer = OAuth2PasswordBearer::new("/token");

        // Get Authorization header
        let auth = req.header("authorization");

        // Extract Bearer token
        let token = bearer.extract_token(auth)?;

        // Verify token
        let claims = jwt_handler.verify(&token.unwrap())?;

        // Validate scopes if needed
        bearer.validate_scopes(&claims)?;

        // Attach claims to request for handlers
        req.set_extension("claims", claims);
        Ok(())
    }}
    "#);
    println!();
}

// ============================================================================
// Scope-based Authorization
// ============================================================================

fn demonstrate_scope_authorization() {
    println!("7. Scope-based Authorization");
    println!("-----------------------------");

    let claims = JwtClaims::new("user-789")
        .scopes(["read", "profile", "settings"]);

    println!("  User scopes: {:?}", claims.scopes);
    println!();

    // Check various scopes
    let checks = [
        ("read", "View data"),
        ("write", "Modify data"),
        ("profile", "View profile"),
        ("admin", "Admin access"),
    ];

    for (scope, description) in checks {
        let allowed = claims.has_scope(scope);
        let status = if allowed { "ALLOWED" } else { "DENIED" };
        println!("  {} [{}]: {}", description, scope, status);
    }
    println!();

    // Check multiple scopes
    println!("  Multiple scope checks:");
    println!("    has_all_scopes(['read', 'profile']): {}",
        claims.has_all_scopes(&["read", "profile"]));
    println!("    has_all_scopes(['read', 'admin']): {}",
        claims.has_all_scopes(&["read", "admin"]));
    println!("    has_any_scope(['write', 'admin']): {}",
        claims.has_any_scope(&["write", "admin"]));
    println!();
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    println!("JWT Authentication Example");
    println!("==========================\n");

    demonstrate_jwt_config();
    demonstrate_jwt_claims();
    demonstrate_jwt_handler();
    demonstrate_oauth2_bearer();
    demonstrate_token_response();
    demonstrate_auth_middleware_pattern();
    demonstrate_scope_authorization();

    println!("Security Notes:");
    println!("  - Use strong secrets (>= 32 bytes for HS256)");
    println!("  - Set appropriate token expiration times");
    println!("  - Use HTTPS in production");
    println!("  - Implement token refresh for long sessions");
    println!("  - Use scope-based authorization for fine-grained access control");
}
