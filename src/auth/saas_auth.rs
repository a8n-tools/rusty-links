use axum_extra::extract::CookieJar;
use jsonwebtoken::{decode, DecodingKey, Validation};

/// SaaS user claims extracted from access_token cookie
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SaasUserClaims {
    pub user_id: String,
    pub email: Option<String>,
    pub membership_status: Option<String>,
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

    // Reject if membership is canceled
    if membership_status.as_deref() == Some("canceled") {
        return None;
    }

    Some(SaasUserClaims {
        user_id,
        email,
        membership_status,
    })
}
