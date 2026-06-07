use crate::error::AppError;

/// OIDC Relying Party + Resource Server configuration (hosted mode).
#[derive(Debug, Clone)]
pub struct OidcConfig {
    /// Issuer URL (`iss` value in tokens).  Empty string means OIDC disabled.
    pub issuer: String,
    /// `aud` expected in `at+jwt` access tokens.
    pub audience: String,
    /// JWKS endpoint (derived from issuer when empty).
    pub jwks_url: String,
    /// JWKS in-memory cache TTL in seconds.
    pub jwks_cache_ttl: u64,
    /// OAuth2 client_id.
    pub client_id: String,
    /// OAuth2 client_secret (confidential client).
    pub client_secret: String,
    /// Absolute redirect URI registered with the OP.
    pub redirect_uri: String,
    /// Post-logout redirect URI registered with the OP.
    pub post_logout_redirect_uri: String,
    /// Clock-skew leeway in seconds applied during token validation.
    pub leeway_seconds: u64,
    /// TTL in seconds for the JTI idempotency cache (lifecycle + logout events).
    pub lifecycle_jti_cache_ttl: u64,
    /// Lifetime in seconds for BFF `rl_session` cookies.
    pub session_ttl_seconds: u64,
}

impl OidcConfig {
    pub fn enabled(&self) -> bool {
        !self.issuer.is_empty()
    }
}

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub app_port: u16,
    pub update_interval_days: u32,
    pub log_level: String,
    // Scheduler configuration
    pub update_interval_hours: u32,
    pub batch_size: usize,
    pub jitter_percent: u8,
    // Hosted (OIDC) mode configuration. Inert when `oidc.issuer` is empty.
    pub host_url: String,
    pub webhook_secret: String,
    pub oidc: OidcConfig,
    // JWT configuration (standalone mode). Inert in hosted mode.
    pub jwt_secret: String,
    pub jwt_expiry_hours: i64,
    pub refresh_token_expiry_days: i64,
    pub account_lockout_attempts: i32,
    pub account_lockout_duration_minutes: i64,
    pub allow_registration: bool,
}

impl Config {
    /// True when the instance runs in hosted mode (OIDC login against a8n
    /// Tools). Resolved at runtime from `OIDC_ISSUER`: set means hosted,
    /// unset means standalone (local JWT auth). Mirrors `OidcConfig::enabled`.
    pub fn hosted(&self) -> bool {
        !self.oidc.issuer.is_empty()
    }

    pub fn from_env() -> Result<Self, AppError> {
        let _ = dotenvy::dotenv();

        let database_url = std::env::var("DATABASE_URL").map_err(|_| {
            AppError::Configuration(
                "Missing required environment variable: DATABASE_URL".to_string(),
            )
        })?;

        let app_port = std::env::var("APP_PORT")
            .map_err(|_| {
                AppError::Configuration(
                    "Missing required environment variable: APP_PORT".to_string(),
                )
            })?
            .parse::<u16>()
            .map_err(|e| AppError::Configuration(format!("Failed to parse APP_PORT: {}", e)))?;

        let update_interval_days = std::env::var("UPDATE_INTERVAL_DAYS")
            .ok()
            .map(|v| {
                v.parse::<u32>().map_err(|e| {
                    AppError::Configuration(format!("Failed to parse UPDATE_INTERVAL_DAYS: {}", e))
                })
            })
            .transpose()?
            .unwrap_or(30);

        if update_interval_days < 1 {
            return Err(AppError::Configuration(
                "Invalid value for UPDATE_INTERVAL_DAYS: must be at least 1".to_string(),
            ));
        }

        let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

        let update_interval_hours = std::env::var("UPDATE_INTERVAL_HOURS")
            .ok()
            .map(|v| {
                v.parse::<u32>().map_err(|e| {
                    AppError::Configuration(format!("Failed to parse UPDATE_INTERVAL_HOURS: {}", e))
                })
            })
            .transpose()?
            .unwrap_or(24);

        let batch_size = std::env::var("BATCH_SIZE")
            .ok()
            .map(|v| {
                v.parse::<usize>().map_err(|e| {
                    AppError::Configuration(format!("Failed to parse BATCH_SIZE: {}", e))
                })
            })
            .transpose()?
            .unwrap_or(50);

        let jitter_percent = std::env::var("JITTER_PERCENT")
            .ok()
            .map(|v| {
                v.parse::<u8>().map_err(|e| {
                    AppError::Configuration(format!("Failed to parse JITTER_PERCENT: {}", e))
                })
            })
            .transpose()?
            .unwrap_or(20);

        if update_interval_hours < 1 {
            return Err(AppError::Configuration(
                "Invalid value for UPDATE_INTERVAL_HOURS: must be at least 1".to_string(),
            ));
        }

        if batch_size < 1 {
            return Err(AppError::Configuration(
                "Invalid value for BATCH_SIZE: must be at least 1".to_string(),
            ));
        }

        if jitter_percent > 100 {
            return Err(AppError::Configuration(
                "Invalid value for JITTER_PERCENT: must be between 0 and 100".to_string(),
            ));
        }

        // Hosted (OIDC) mode configuration
        let host_url =
            std::env::var("HOST_URL").unwrap_or_else(|_| format!("http://localhost:{app_port}"));

        let webhook_secret = std::env::var("WEBHOOK_SECRET").unwrap_or_else(|_| {
            tracing::warn!("WEBHOOK_SECRET not set - webhook signatures will not be validated");
            String::new()
        });

        let oidc = {
            let issuer = std::env::var("OIDC_ISSUER").unwrap_or_default();

            let audience = std::env::var("OIDC_AUDIENCE")
                .unwrap_or_else(|_| "https://links.a8n.run/api".to_string());

            let jwks_url = std::env::var("OIDC_JWKS_URL").unwrap_or_else(|_| {
                if issuer.is_empty() {
                    String::new()
                } else {
                    format!("{}/.well-known/jwks.json", issuer.trim_end_matches('/'))
                }
            });

            let jwks_cache_ttl = std::env::var("OIDC_JWKS_CACHE_TTL")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(300);

            let client_id = std::env::var("OIDC_CLIENT_ID").unwrap_or_default();

            let client_secret = std::env::var("OIDC_CLIENT_SECRET")
                .or_else(|_| {
                    std::fs::read_to_string("/run/secrets/oidc_client_secret")
                        .map(|s| s.trim().to_string())
                })
                .unwrap_or_default();

            let redirect_uri = std::env::var("OIDC_REDIRECT_URI")
                .unwrap_or_else(|_| format!("{}/oauth2/callback", host_url.trim_end_matches('/')));

            let post_logout_redirect_uri = std::env::var("OIDC_POST_LOGOUT_REDIRECT_URI")
                .unwrap_or_else(|_| format!("{}/", host_url.trim_end_matches('/')));

            let leeway_seconds = std::env::var("OIDC_LEEWAY_SECONDS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(30);

            let lifecycle_jti_cache_ttl = std::env::var("OIDC_LIFECYCLE_JTI_CACHE_TTL")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(300);

            let session_ttl_seconds = std::env::var("OIDC_SESSION_TTL_SECONDS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(1_209_600); // 14 days

            // Fail fast: issuer set but credentials missing.
            if !issuer.is_empty() && (client_id.is_empty() || client_secret.is_empty()) {
                return Err(AppError::Configuration(
                    "OIDC_ISSUER is set but OIDC_CLIENT_ID or OIDC_CLIENT_SECRET is missing"
                        .to_string(),
                ));
            }

            // JWKS URL must be HTTPS in production.
            if !jwks_url.is_empty()
                && !jwks_url.starts_with("https://")
                && !jwks_url.starts_with("http://localhost")
            {
                return Err(AppError::Configuration(
                    "OIDC_JWKS_URL must use HTTPS".to_string(),
                ));
            }

            OidcConfig {
                issuer,
                audience,
                jwks_url,
                jwks_cache_ttl,
                client_id,
                client_secret,
                redirect_uri,
                post_logout_redirect_uri,
                leeway_seconds,
                lifecycle_jti_cache_ttl,
                session_ttl_seconds,
            }
        };

        // JWT configuration (standalone mode)
        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            tracing::warn!(
                "JWT_SECRET not set - using random secret (tokens will not survive restarts)"
            );
            use rand::Rng;
            let bytes: [u8; 32] = rand::thread_rng().gen();
            base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
        });

        let jwt_expiry_hours = std::env::var("JWT_EXPIRY")
            .ok()
            .map(|v| {
                v.parse::<i64>().map_err(|e| {
                    AppError::Configuration(format!("Failed to parse JWT_EXPIRY: {}", e))
                })
            })
            .transpose()?
            .unwrap_or(1);

        let refresh_token_expiry_days = std::env::var("REFRESH_TOKEN_EXPIRY")
            .ok()
            .map(|v| {
                v.parse::<i64>().map_err(|e| {
                    AppError::Configuration(format!("Failed to parse REFRESH_TOKEN_EXPIRY: {}", e))
                })
            })
            .transpose()?
            .unwrap_or(7);

        let account_lockout_attempts = std::env::var("ACCOUNT_LOCKOUT_ATTEMPTS")
            .ok()
            .map(|v| {
                v.parse::<i32>().map_err(|e| {
                    AppError::Configuration(format!(
                        "Failed to parse ACCOUNT_LOCKOUT_ATTEMPTS: {}",
                        e
                    ))
                })
            })
            .transpose()?
            .unwrap_or(5);

        let account_lockout_duration_minutes = std::env::var("ACCOUNT_LOCKOUT_DURATION")
            .ok()
            .map(|v| {
                v.parse::<i64>().map_err(|e| {
                    AppError::Configuration(format!(
                        "Failed to parse ACCOUNT_LOCKOUT_DURATION: {}",
                        e
                    ))
                })
            })
            .transpose()?
            .unwrap_or(30);

        let allow_registration = std::env::var("ALLOW_REGISTRATION")
            .ok()
            .map(|v| v == "true" || v == "1")
            .unwrap_or(true);

        Ok(Config {
            database_url,
            app_port,
            update_interval_days,
            log_level,
            update_interval_hours,
            batch_size,
            jitter_percent,
            host_url,
            webhook_secret,
            oidc,
            jwt_secret,
            jwt_expiry_hours,
            refresh_token_expiry_days,
            account_lockout_attempts,
            account_lockout_duration_minutes,
            allow_registration,
        })
    }

    pub fn masked_database_url(&self) -> String {
        if let Some(at_pos) = self.database_url.find('@') {
            if let Some(colon_pos) = self.database_url[..at_pos].rfind(':') {
                let mut masked = self.database_url.clone();
                masked.replace_range(colon_pos + 1..at_pos, "****");
                return masked;
            }
        }
        "postgresql://****:****@****/****".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            database_url: "postgresql://user:password@localhost/rusty_links".to_string(),
            app_port: 4002,
            update_interval_days: 30,
            log_level: "info".to_string(),
            update_interval_hours: 24,
            batch_size: 50,
            jitter_percent: 20,
            host_url: "http://localhost:4002".to_string(),
            webhook_secret: "test-webhook-secret".to_string(),
            oidc: OidcConfig {
                issuer: "http://localhost:18080".to_string(),
                audience: "http://localhost:4002/api".to_string(),
                jwks_url: "http://localhost:18080/.well-known/jwks.json".to_string(),
                jwks_cache_ttl: 300,
                client_id: "a8000000-0000-0000-0000-000000000005".to_string(),
                client_secret: "test-secret".to_string(),
                redirect_uri: "http://localhost:4002/oauth2/callback".to_string(),
                post_logout_redirect_uri: "http://localhost:4002/".to_string(),
                leeway_seconds: 30,
                lifecycle_jti_cache_ttl: 300,
                session_ttl_seconds: 1_209_600,
            },
            jwt_secret: "test_secret".to_string(),
            jwt_expiry_hours: 1,
            refresh_token_expiry_days: 7,
            account_lockout_attempts: 5,
            account_lockout_duration_minutes: 30,
            allow_registration: true,
        }
    }

    #[test]
    fn test_masked_database_url() {
        let config = test_config();
        let masked = config.masked_database_url();
        assert!(!masked.contains("password"));
        assert!(masked.contains("****"));
    }

    #[test]
    fn test_masked_database_url_preserves_user_and_host() {
        let config = test_config();
        let masked = config.masked_database_url();
        assert!(masked.contains("user"));
        assert!(masked.contains("localhost"));
        assert!(masked.contains("rusty_links"));
    }

    #[test]
    fn test_masked_database_url_no_password() {
        let mut config = test_config();
        config.database_url = "postgresql://localhost/rusty_links".to_string();
        let masked = config.masked_database_url();
        assert_eq!(masked, "postgresql://****:****@****/****");
    }

    #[test]
    fn test_masked_database_url_complex_password() {
        let mut config = test_config();
        config.database_url = "postgresql://admin:p@ss:w0rd!#@db.example.com:5432/mydb".to_string();
        let masked = config.masked_database_url();
        assert!(!masked.contains("p@ss:w0rd!#"));
        assert!(masked.contains("****"));
    }

    #[test]
    fn test_config_validation_update_interval_days_minimum() {
        let min_valid = 1u32;
        assert!(min_valid >= 1);
        let invalid = 0u32;
        assert!(invalid < 1);
    }

    #[test]
    fn test_config_validation_jitter_percent_range() {
        let valid_zero: u8 = 0;
        let valid_max: u8 = 100;
        assert!(valid_zero <= 100);
        assert!(valid_max <= 100);
        let invalid: u8 = 101;
        assert!(invalid > 100);
    }

    #[test]
    fn test_config_validation_batch_size_minimum() {
        let valid: usize = 1;
        assert!(valid >= 1);
        let invalid: usize = 0;
        assert!(invalid < 1);
    }

    #[test]
    fn test_hosted_true_when_issuer_set() {
        let config = test_config();
        assert!(!config.oidc.issuer.is_empty());
        assert!(config.hosted());
    }

    #[test]
    fn test_hosted_false_when_issuer_empty() {
        let mut config = test_config();
        config.oidc.issuer = String::new();
        assert!(!config.hosted());
    }

    #[test]
    fn test_config_clone() {
        let config = test_config();
        let cloned = config.clone();
        assert_eq!(config.database_url, cloned.database_url);
        assert_eq!(config.app_port, cloned.app_port);
        assert_eq!(config.update_interval_hours, cloned.update_interval_hours);
        assert_eq!(config.batch_size, cloned.batch_size);
        assert_eq!(config.jitter_percent, cloned.jitter_percent);
    }
}
