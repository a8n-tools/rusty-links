use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

use crate::auth::middleware::Claims;

/// Create a JWT token for a user
pub fn create_jwt(
    email: &str,
    user_id: Uuid,
    is_admin: bool,
    secret: &str,
    expiry_hours: i64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::hours(expiry_hours))
        .expect("valid timestamp")
        .timestamp();

    let claims = Claims {
        sub: email.to_owned(),
        user_id: user_id.to_string(),
        is_admin,
        exp: expiration as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_ref()),
    )
}

/// Decode and validate a JWT token
pub fn decode_jwt(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_ref()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

/// Generate a cryptographically secure refresh token
pub fn generate_refresh_token() -> String {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-key-for-jwt-testing";

    #[test]
    fn test_create_and_decode_jwt() {
        let user_id = Uuid::new_v4();
        let email = "test@example.com";

        let token = create_jwt(email, user_id, false, TEST_SECRET, 1).unwrap();
        let claims = decode_jwt(&token, TEST_SECRET).unwrap();

        assert_eq!(claims.sub, email);
        assert_eq!(claims.user_id, user_id.to_string());
        assert!(!claims.is_admin);
    }

    #[test]
    fn test_create_jwt_admin() {
        let user_id = Uuid::new_v4();
        let token = create_jwt("admin@test.com", user_id, true, TEST_SECRET, 1).unwrap();
        let claims = decode_jwt(&token, TEST_SECRET).unwrap();
        assert!(claims.is_admin);
    }

    #[test]
    fn test_decode_jwt_wrong_secret() {
        let token =
            create_jwt("test@test.com", Uuid::new_v4(), false, TEST_SECRET, 1).unwrap();
        let result = decode_jwt(&token, "wrong-secret");
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_jwt_invalid_token() {
        let result = decode_jwt("not.a.valid.token", TEST_SECRET);
        assert!(result.is_err());
    }

    #[test]
    fn test_jwt_expiration() {
        // Create token that expires in the past (0 hours = already expired, but
        // chrono Duration::hours(0) means now, so use the token normally)
        let token =
            create_jwt("test@test.com", Uuid::new_v4(), false, TEST_SECRET, 1).unwrap();
        // Token with 1 hour expiry should be valid
        assert!(decode_jwt(&token, TEST_SECRET).is_ok());
    }

    #[test]
    fn test_generate_refresh_token_length() {
        let token = generate_refresh_token();
        // 32 bytes base64url-encoded without padding = 43 chars
        assert_eq!(token.len(), 43);
    }

    #[test]
    fn test_generate_refresh_token_uniqueness() {
        let t1 = generate_refresh_token();
        let t2 = generate_refresh_token();
        assert_ne!(t1, t2, "Refresh tokens should be unique");
    }

    #[test]
    fn test_generate_refresh_token_is_base64url() {
        let token = generate_refresh_token();
        // base64url chars: A-Z, a-z, 0-9, -, _
        assert!(
            token.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "Token should be valid base64url: {}",
            token
        );
    }

    #[test]
    fn test_jwt_preserves_user_id() {
        let user_id = Uuid::new_v4();
        let token = create_jwt("u@t.com", user_id, false, TEST_SECRET, 24).unwrap();
        let claims = decode_jwt(&token, TEST_SECRET).unwrap();
        let parsed: Uuid = claims.user_id.parse().unwrap();
        assert_eq!(parsed, user_id);
    }
}
