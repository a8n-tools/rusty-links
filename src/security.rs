use chrono::{Duration, Utc};
use sqlx::PgPool;
use std::net::{IpAddr, Ipv4Addr, ToSocketAddrs};
use url::Url;

use crate::error::AppError;

/// Validate that a URL is safe to fetch (SSRF protection)
///
/// Parses the URL, verifies the scheme is http/https, resolves the hostname,
/// and checks that none of the resolved IP addresses are private or reserved.
///
/// # Arguments
/// * `url` - The URL string to validate
///
/// # Returns
/// * `Ok(())` if the URL is safe to fetch
/// * `Err(AppError::Validation)` if the URL targets a private/reserved IP or has an invalid scheme
pub fn validate_url_for_ssrf(url: &str) -> Result<(), AppError> {
    let parsed =
        Url::parse(url).map_err(|e| AppError::validation("url", &format!("Invalid URL: {}", e)))?;

    // Check scheme
    match parsed.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(AppError::validation(
                "url",
                &format!("Unsupported URL scheme: {}", scheme),
            ));
        }
    }

    // Extract hostname
    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::validation("url", "URL must have a hostname"))?;

    // Resolve hostname to IP addresses
    let socket_addr = format!("{}:0", host);
    let addrs: Vec<_> = socket_addr
        .to_socket_addrs()
        .map_err(|e| AppError::validation("url", &format!("Could not resolve hostname: {}", e)))?
        .collect();

    if addrs.is_empty() {
        return Err(AppError::validation(
            "url",
            "Hostname did not resolve to any addresses",
        ));
    }

    // Check each resolved address
    for addr in &addrs {
        let ip = addr.ip();
        check_ip_not_private(&ip)?;
    }

    Ok(())
}

/// Check that an IP address is not private or reserved
fn check_ip_not_private(ip: &IpAddr) -> Result<(), AppError> {
    match ip {
        IpAddr::V4(ipv4) => check_ipv4_not_private(ipv4),
        IpAddr::V6(ipv6) => {
            // Check the IPv6 address itself
            if ipv6.is_loopback() || ipv6.is_unspecified() {
                return Err(AppError::validation(
                    "url",
                    "URL must not point to a reserved IP address",
                ));
            }

            // Check IPv4-mapped IPv6 addresses (e.g., ::ffff:127.0.0.1)
            if let Some(ipv4) = ipv6.to_ipv4_mapped() {
                check_ipv4_not_private(&ipv4)?;
            }

            Ok(())
        }
    }
}

/// Check that an IPv4 address is not private or reserved
fn check_ipv4_not_private(ipv4: &Ipv4Addr) -> Result<(), AppError> {
    let reject = |_reason: &str| -> Result<(), AppError> {
        Err(AppError::validation(
            "url",
            "URL must not point to a private or reserved IP address",
        ))
    };

    // Loopback (127.0.0.0/8)
    if ipv4.is_loopback() {
        return reject("loopback");
    }

    // Unspecified (0.0.0.0)
    if ipv4.is_unspecified() {
        return reject("unspecified");
    }

    // Private (10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16)
    if ipv4.is_private() {
        return reject("private");
    }

    // Link-local (169.254.0.0/16) - also covers cloud metadata (169.254.169.254)
    if ipv4.is_link_local() {
        return reject("link-local");
    }

    // Broadcast (255.255.255.255)
    if ipv4.is_broadcast() {
        return reject("broadcast");
    }

    // CGNAT (100.64.0.0/10): first octet 100, second octet 64-127
    let octets = ipv4.octets();
    if octets[0] == 100 && (64..=127).contains(&octets[1]) {
        return reject("CGNAT");
    }

    // Documentation ranges
    // 192.0.2.0/24 (TEST-NET-1)
    if octets[0] == 192 && octets[1] == 0 && octets[2] == 2 {
        return reject("documentation (TEST-NET-1)");
    }
    // 198.51.100.0/24 (TEST-NET-2)
    if octets[0] == 198 && octets[1] == 51 && octets[2] == 100 {
        return reject("documentation (TEST-NET-2)");
    }
    // 203.0.113.0/24 (TEST-NET-3)
    if octets[0] == 203 && octets[1] == 0 && octets[2] == 113 {
        return reject("documentation (TEST-NET-3)");
    }

    Ok(())
}

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
        return Err("Password must contain at least one special character (e.g., !@#$%^&*)".to_string());
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

/// Delete login attempts older than the specified retention period
///
/// Returns the number of rows deleted.
pub async fn cleanup_old_login_attempts(
    pool: &PgPool,
    retention_days: i64,
) -> Result<u64, sqlx::Error> {
    let cutoff = Utc::now() - Duration::days(retention_days);
    let result = sqlx::query("DELETE FROM login_attempts WHERE attempted_at < $1")
        .bind(cutoff)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

/// Delete expired refresh tokens
///
/// Returns the number of rows deleted.
pub async fn cleanup_expired_refresh_tokens(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM refresh_tokens WHERE expires_at < NOW()")
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
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

    // SSRF validation tests

    #[test]
    fn test_ssrf_valid_public_url() {
        // Well-known public websites should pass
        assert!(validate_url_for_ssrf("https://example.com").is_ok());
        assert!(validate_url_for_ssrf("http://example.com").is_ok());
        assert!(validate_url_for_ssrf("https://github.com/rust-lang/rust").is_ok());
    }

    #[test]
    fn test_ssrf_blocks_private_ips() {
        // 10.0.0.0/8
        assert!(validate_url_for_ssrf("http://10.0.0.1").is_err());
        assert!(validate_url_for_ssrf("http://10.255.255.255").is_err());
        // 172.16.0.0/12
        assert!(validate_url_for_ssrf("http://172.16.0.1").is_err());
        assert!(validate_url_for_ssrf("http://172.31.255.255").is_err());
        // 192.168.0.0/16
        assert!(validate_url_for_ssrf("http://192.168.1.1").is_err());
        assert!(validate_url_for_ssrf("http://192.168.0.1").is_err());
    }

    #[test]
    fn test_ssrf_blocks_loopback() {
        assert!(validate_url_for_ssrf("http://127.0.0.1").is_err());
        assert!(validate_url_for_ssrf("http://127.0.0.1:8080/admin").is_err());
    }

    #[test]
    fn test_ssrf_blocks_link_local() {
        // 169.254.0.0/16 - includes cloud metadata endpoint
        assert!(validate_url_for_ssrf("http://169.254.169.254/latest/meta-data/").is_err());
        assert!(validate_url_for_ssrf("http://169.254.0.1").is_err());
    }

    #[test]
    fn test_ssrf_blocks_non_http_schemes() {
        assert!(validate_url_for_ssrf("ftp://example.com").is_err());
        assert!(validate_url_for_ssrf("file:///etc/passwd").is_err());
        assert!(validate_url_for_ssrf("gopher://example.com").is_err());
        assert!(validate_url_for_ssrf("javascript:alert(1)").is_err());
    }

    #[test]
    fn test_ssrf_blocks_invalid_urls() {
        assert!(validate_url_for_ssrf("not a url").is_err());
        assert!(validate_url_for_ssrf("").is_err());
    }

    #[test]
    fn test_ssrf_ipv4_check_cgnat() {
        // CGNAT range: 100.64.0.0/10
        let ip = Ipv4Addr::new(100, 64, 0, 1);
        assert!(check_ipv4_not_private(&ip).is_err());
        let ip = Ipv4Addr::new(100, 127, 255, 255);
        assert!(check_ipv4_not_private(&ip).is_err());
        // Just outside CGNAT range
        let ip = Ipv4Addr::new(100, 128, 0, 1);
        assert!(check_ipv4_not_private(&ip).is_ok());
    }

    #[test]
    fn test_ssrf_ipv4_check_documentation_ranges() {
        // TEST-NET-1: 192.0.2.0/24
        let ip = Ipv4Addr::new(192, 0, 2, 1);
        assert!(check_ipv4_not_private(&ip).is_err());
        // TEST-NET-2: 198.51.100.0/24
        let ip = Ipv4Addr::new(198, 51, 100, 1);
        assert!(check_ipv4_not_private(&ip).is_err());
        // TEST-NET-3: 203.0.113.0/24
        let ip = Ipv4Addr::new(203, 0, 113, 1);
        assert!(check_ipv4_not_private(&ip).is_err());
    }

    #[test]
    fn test_ssrf_ipv4_check_broadcast() {
        let ip = Ipv4Addr::new(255, 255, 255, 255);
        assert!(check_ipv4_not_private(&ip).is_err());
    }

    #[test]
    fn test_ssrf_ipv4_check_unspecified() {
        let ip = Ipv4Addr::new(0, 0, 0, 0);
        assert!(check_ipv4_not_private(&ip).is_err());
    }

    #[test]
    fn test_ssrf_ipv4_check_public_ip() {
        // Known public IPs should pass
        let ip = Ipv4Addr::new(8, 8, 8, 8); // Google DNS
        assert!(check_ipv4_not_private(&ip).is_ok());
        let ip = Ipv4Addr::new(1, 1, 1, 1); // Cloudflare DNS
        assert!(check_ipv4_not_private(&ip).is_ok());
    }

    #[test]
    fn test_ssrf_blocks_localhost_hostname() {
        assert!(validate_url_for_ssrf("http://localhost").is_err());
        assert!(validate_url_for_ssrf("http://localhost:3000").is_err());
    }

    #[test]
    fn test_ssrf_blocks_no_hostname() {
        assert!(validate_url_for_ssrf("http://").is_err());
    }

    #[test]
    fn test_validate_password_exactly_8_chars() {
        // Exactly 8 chars, meets all requirements
        assert!(validate_password("P@ssw0r!").is_ok());
    }

    #[test]
    fn test_validate_password_empty() {
        let err = validate_password("").unwrap_err();
        assert!(err.contains("8 characters"));
    }

    #[test]
    fn test_validate_password_all_requirements_met() {
        assert!(validate_password("MyP@ss1234!").is_ok());
        assert!(validate_password("A1!bcdef").is_ok());
    }

    #[test]
    fn test_validate_password_no_lowercase() {
        // All uppercase + number + special — should still pass
        // (no lowercase requirement exists)
        assert!(validate_password("ABCDEF1!").is_ok());
    }

    #[test]
    fn test_ssrf_ipv6_loopback() {
        let ip = IpAddr::V6(std::net::Ipv6Addr::LOCALHOST);
        assert!(check_ip_not_private(&ip).is_err());
    }

    #[test]
    fn test_ssrf_ipv6_unspecified() {
        let ip = IpAddr::V6(std::net::Ipv6Addr::UNSPECIFIED);
        assert!(check_ip_not_private(&ip).is_err());
    }
}
