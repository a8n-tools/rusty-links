use axum_extra::extract::CookieJar;
use jsonwebtoken::{decode, DecodingKey, Validation};

/// SaaS user claims extracted from access_token cookie
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SaasUserClaims {
    pub user_id: String,
    pub email: Option<String>,
    pub membership_status: Option<String>,
    pub is_admin: bool,
}

/// Extract and verify user claims from access_token cookie (SaaS mode).
///
/// Validates the JWT signature using the shared secret, then extracts user claims.
pub fn get_user_from_cookie(jar: &CookieJar, secret: &str) -> Option<SaasUserClaims> {
    let cookie = jar.get("access_token")?;
    let token = cookie.value();

    let mut validation = Validation::default();
    validation.algorithms = vec![
        jsonwebtoken::Algorithm::HS256,
        jsonwebtoken::Algorithm::HS384,
        jsonwebtoken::Algorithm::HS512,
    ];
    // Only require exp claim
    validation.required_spec_claims.clear();
    validation.required_spec_claims.insert("exp".to_string());

    let token_data = decode::<serde_json::Value>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .ok()?;

    let payload = token_data.claims;

    // Extract user_id from JWT payload
    // The parent app's JWT may have user_id as "sub" (UUID or integer), "user_id", or "id"
    let user_id = payload
        .get("user_id")
        .and_then(|v| v.as_str().map(String::from))
        .or_else(|| {
            payload
                .get("sub")
                .and_then(|v| v.as_str().map(String::from))
        })
        .or_else(|| payload.get("id").and_then(|v| v.as_str().map(String::from)))?;

    let email = payload
        .get("email")
        .and_then(|v| v.as_str())
        .map(String::from);
    let membership_status = payload
        .get("membership_status")
        .and_then(|v| v.as_str())
        .map(String::from);

    // Note: membership access is enforced in the middleware, not here.
    // All valid JWT holders are returned so the middleware can decide
    // whether to redirect non-members to the membership page.

    // Extract admin status from JWT payload
    // Supports "is_admin" (bool) or "role"/"roles" containing "admin"
    let is_admin = payload
        .get("is_admin")
        .and_then(|v| v.as_bool())
        .unwrap_or_else(|| {
            // Check "role" == "admin"
            if let Some(role) = payload.get("role").and_then(|v| v.as_str()) {
                return role.eq_ignore_ascii_case("admin");
            }
            // Check "roles" array contains "admin"
            if let Some(roles) = payload.get("roles").and_then(|v| v.as_array()) {
                return roles
                    .iter()
                    .any(|r| r.as_str().is_some_and(|s| s.eq_ignore_ascii_case("admin")));
            }
            false
        });

    Some(SaasUserClaims {
        user_id,
        email,
        membership_status,
        is_admin,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum_extra::extract::cookie::Cookie;

    const TEST_SECRET: &str = "test-saas-secret";

    fn make_jwt(claims: &serde_json::Value, secret: &str) -> String {
        jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            claims,
            &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    fn jar_with_token(token: &str) -> CookieJar {
        CookieJar::new().add(Cookie::new("access_token", token.to_string()))
    }

    #[test]
    fn test_extract_user_id_from_user_id_field() {
        let claims = serde_json::json!({
            "user_id": "abc-123",
            "exp": 9999999999u64
        });
        let token = make_jwt(&claims, TEST_SECRET);
        let jar = jar_with_token(&token);
        let result = get_user_from_cookie(&jar, TEST_SECRET).unwrap();
        assert_eq!(result.user_id, "abc-123");
    }

    #[test]
    fn test_extract_user_id_from_sub_field() {
        let claims = serde_json::json!({
            "sub": "user-456",
            "exp": 9999999999u64
        });
        let token = make_jwt(&claims, TEST_SECRET);
        let jar = jar_with_token(&token);
        let result = get_user_from_cookie(&jar, TEST_SECRET).unwrap();
        assert_eq!(result.user_id, "user-456");
    }

    #[test]
    fn test_extract_user_id_from_id_field() {
        let claims = serde_json::json!({
            "id": "user-789",
            "exp": 9999999999u64
        });
        let token = make_jwt(&claims, TEST_SECRET);
        let jar = jar_with_token(&token);
        let result = get_user_from_cookie(&jar, TEST_SECRET).unwrap();
        assert_eq!(result.user_id, "user-789");
    }

    #[test]
    fn test_extract_email() {
        let claims = serde_json::json!({
            "user_id": "u1",
            "email": "test@example.com",
            "exp": 9999999999u64
        });
        let token = make_jwt(&claims, TEST_SECRET);
        let jar = jar_with_token(&token);
        let result = get_user_from_cookie(&jar, TEST_SECRET).unwrap();
        assert_eq!(result.email, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_extract_membership_status() {
        let claims = serde_json::json!({
            "user_id": "u1",
            "membership_status": "active",
            "exp": 9999999999u64
        });
        let token = make_jwt(&claims, TEST_SECRET);
        let jar = jar_with_token(&token);
        let result = get_user_from_cookie(&jar, TEST_SECRET).unwrap();
        assert_eq!(result.membership_status, Some("active".to_string()));
    }

    #[test]
    fn test_is_admin_from_bool() {
        let claims = serde_json::json!({
            "user_id": "u1",
            "is_admin": true,
            "exp": 9999999999u64
        });
        let token = make_jwt(&claims, TEST_SECRET);
        let jar = jar_with_token(&token);
        let result = get_user_from_cookie(&jar, TEST_SECRET).unwrap();
        assert!(result.is_admin);
    }

    #[test]
    fn test_is_admin_from_role_string() {
        let claims = serde_json::json!({
            "user_id": "u1",
            "role": "admin",
            "exp": 9999999999u64
        });
        let token = make_jwt(&claims, TEST_SECRET);
        let jar = jar_with_token(&token);
        let result = get_user_from_cookie(&jar, TEST_SECRET).unwrap();
        assert!(result.is_admin);
    }

    #[test]
    fn test_is_admin_from_roles_array() {
        let claims = serde_json::json!({
            "user_id": "u1",
            "roles": ["user", "admin"],
            "exp": 9999999999u64
        });
        let token = make_jwt(&claims, TEST_SECRET);
        let jar = jar_with_token(&token);
        let result = get_user_from_cookie(&jar, TEST_SECRET).unwrap();
        assert!(result.is_admin);
    }

    #[test]
    fn test_not_admin_by_default() {
        let claims = serde_json::json!({
            "user_id": "u1",
            "exp": 9999999999u64
        });
        let token = make_jwt(&claims, TEST_SECRET);
        let jar = jar_with_token(&token);
        let result = get_user_from_cookie(&jar, TEST_SECRET).unwrap();
        assert!(!result.is_admin);
    }

    #[test]
    fn test_not_admin_from_non_admin_role() {
        let claims = serde_json::json!({
            "user_id": "u1",
            "role": "user",
            "exp": 9999999999u64
        });
        let token = make_jwt(&claims, TEST_SECRET);
        let jar = jar_with_token(&token);
        let result = get_user_from_cookie(&jar, TEST_SECRET).unwrap();
        assert!(!result.is_admin);
    }

    #[test]
    fn test_invalid_secret_returns_none() {
        let claims = serde_json::json!({
            "user_id": "u1",
            "exp": 9999999999u64
        });
        let token = make_jwt(&claims, "correct-secret");
        let jar = jar_with_token(&token);
        let result = get_user_from_cookie(&jar, "wrong-secret");
        assert!(result.is_none());
    }

    #[test]
    fn test_missing_cookie_returns_none() {
        let jar = CookieJar::new();
        let result = get_user_from_cookie(&jar, TEST_SECRET);
        assert!(result.is_none());
    }

    #[test]
    fn test_missing_user_id_returns_none() {
        let claims = serde_json::json!({
            "email": "test@test.com",
            "exp": 9999999999u64
        });
        let token = make_jwt(&claims, TEST_SECRET);
        let jar = jar_with_token(&token);
        let result = get_user_from_cookie(&jar, TEST_SECRET);
        assert!(result.is_none());
    }

    #[test]
    fn test_user_id_field_takes_priority_over_sub() {
        let claims = serde_json::json!({
            "user_id": "from-user-id",
            "sub": "from-sub",
            "exp": 9999999999u64
        });
        let token = make_jwt(&claims, TEST_SECRET);
        let jar = jar_with_token(&token);
        let result = get_user_from_cookie(&jar, TEST_SECRET).unwrap();
        assert_eq!(result.user_id, "from-user-id");
    }

    #[test]
    fn test_is_admin_case_insensitive() {
        let claims = serde_json::json!({
            "user_id": "u1",
            "role": "ADMIN",
            "exp": 9999999999u64
        });
        let token = make_jwt(&claims, TEST_SECRET);
        let jar = jar_with_token(&token);
        let result = get_user_from_cookie(&jar, TEST_SECRET).unwrap();
        assert!(result.is_admin);
    }
}
