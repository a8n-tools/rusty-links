use chrono::{Duration, Utc};
use sqlx::PgPool;

/// Validate password complexity requirements
pub fn validate_password(password: &str) -> Result<(), String> {
    if password.len() < 8 {
        return Err("Password must be at least 8 characters long".to_string());
    }

    if !password.chars().any(|c| c.is_uppercase()) {
        return Err("Password must contain at least one uppercase letter".to_string());
    }

    if !password.chars().any(|c| c.is_numeric()) {
        return Err("Password must contain at least one number".to_string());
    }

    if !password.chars().any(|c| !c.is_alphanumeric()) {
        return Err("Password must contain at least one special character".to_string());
    }

    Ok(())
}

/// Check if account is locked due to too many failed login attempts
pub async fn is_account_locked(
    pool: &PgPool,
    email: &str,
    max_attempts: i32,
    lockout_minutes: i64,
) -> bool {
    let cutoff = Utc::now() - Duration::minutes(lockout_minutes);

    let failed_attempts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM login_attempts WHERE email = $1 AND success = false AND attempted_at > $2",
    )
    .bind(email)
    .bind(cutoff)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    failed_attempts >= max_attempts as i64
}

/// Record a login attempt (success or failure) for tracking
pub async fn record_login_attempt(pool: &PgPool, email: &str, success: bool) {
    let _ = sqlx::query("INSERT INTO login_attempts (email, success) VALUES ($1, $2)")
        .bind(email)
        .bind(success)
        .execute(pool)
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_password_valid() {
        assert!(validate_password("P@ssw0rd!").is_ok());
        assert!(validate_password("Str0ng!Pass").is_ok());
    }

    #[test]
    fn test_validate_password_too_short() {
        let err = validate_password("P@1a").unwrap_err();
        assert!(err.contains("8 characters"));
    }

    #[test]
    fn test_validate_password_no_uppercase() {
        let err = validate_password("p@ssw0rd!").unwrap_err();
        assert!(err.contains("uppercase"));
    }

    #[test]
    fn test_validate_password_no_number() {
        let err = validate_password("P@ssword!").unwrap_err();
        assert!(err.contains("number"));
    }

    #[test]
    fn test_validate_password_no_special() {
        let err = validate_password("Passw0rdd").unwrap_err();
        assert!(err.contains("special"));
    }
}
