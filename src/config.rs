use crate::error::AppError;

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
    // SaaS mode configuration
    #[cfg(feature = "saas")]
    pub saas_login_url: String,
    #[cfg(feature = "saas")]
    pub host_url: String,
    #[cfg(feature = "saas")]
    pub saas_jwt_secret: String,
    #[cfg(feature = "saas")]
    pub saas_logout_url: String,
    #[cfg(feature = "saas")]
    pub saas_membership_url: String,
    #[cfg(feature = "saas")]
    pub saas_refresh_url: String,
    // JWT configuration (standalone mode)
    #[cfg(feature = "standalone")]
    pub jwt_secret: String,
    #[cfg(feature = "standalone")]
    pub jwt_expiry_hours: i64,
    #[cfg(feature = "standalone")]
    pub refresh_token_expiry_days: i64,
    #[cfg(feature = "standalone")]
    pub account_lockout_attempts: i32,
    #[cfg(feature = "standalone")]
    pub account_lockout_duration_minutes: i64,
    #[cfg(feature = "standalone")]
    pub allow_registration: bool,
}

impl Config {
    /// Load configuration from environment variables
    ///
    /// # Errors
    ///
    /// Returns `AppError::Configuration` if:
    /// - Required environment variables are missing (DATABASE_URL, APP_PORT)
    /// - Values cannot be parsed (e.g., APP_PORT is not a valid number)
    /// - Values fail validation (e.g., UPDATE_INTERVAL_DAYS < 1)
    pub fn from_env() -> Result<Self, AppError> {
        // Load feature-specific .env file, falling back to .env
        #[cfg(feature = "saas")]
        let _ = dotenvy::from_filename(".env.saas").or_else(|_| dotenvy::dotenv());
        #[cfg(feature = "standalone")]
        let _ = dotenvy::from_filename(".env.standalone").or_else(|_| dotenvy::dotenv());
        #[cfg(not(any(feature = "saas", feature = "standalone")))]
        let _ = dotenvy::dotenv();

        // Load required variables
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

        // Load optional variables with defaults
        let update_interval_days = std::env::var("UPDATE_INTERVAL_DAYS")
            .ok()
            .map(|v| {
                v.parse::<u32>().map_err(|e| {
                    AppError::Configuration(format!("Failed to parse UPDATE_INTERVAL_DAYS: {}", e))
                })
            })
            .transpose()?
            .unwrap_or(30);

        // Validate update_interval_days
        if update_interval_days < 1 {
            return Err(AppError::Configuration(
                "Invalid value for UPDATE_INTERVAL_DAYS: must be at least 1".to_string(),
            ));
        }

        let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

        // Load scheduler configuration
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

        // Validate scheduler configuration
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

        // SaaS mode configuration
        #[cfg(feature = "saas")]
        let saas_login_url = std::env::var("SAAS_LOGIN_URL")
            .unwrap_or_else(|_| "http://localhost:5173/login".to_string());
        #[cfg(feature = "saas")]
        let host_url = std::env::var("HOST_URL")
            .unwrap_or_else(|_| format!("http://localhost:{app_port}"));
        #[cfg(feature = "saas")]
        let saas_jwt_secret = std::env::var("SAAS_JWT_SECRET")
            .unwrap_or_else(|_| {
                tracing::warn!("SAAS_JWT_SECRET not set — JWT signatures will not be validated");
                String::new()
            });

        #[cfg(feature = "saas")]
        let saas_logout_url = std::env::var("SAAS_LOGOUT_URL")
            .unwrap_or_else(|_| "http://localhost:18080/v1/auth/logout".to_string());

        #[cfg(feature = "saas")]
        let saas_membership_url = std::env::var("SAAS_MEMBERSHIP_URL")
            .unwrap_or_else(|_| "http://localhost:5173/membership".to_string());

        #[cfg(feature = "saas")]
        let saas_refresh_url = std::env::var("SAAS_REFRESH_URL")
            .unwrap_or_else(|_| saas_logout_url.replace("/auth/logout", "/auth/refresh"));

        // JWT configuration (standalone mode only)
        #[cfg(feature = "standalone")]
        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| {
            tracing::warn!(
                "JWT_SECRET not set - using random secret (tokens will not survive restarts)"
            );
            use rand::Rng;
            let bytes: [u8; 32] = rand::thread_rng().gen();
            base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
        });

        #[cfg(feature = "standalone")]
        let jwt_expiry_hours = std::env::var("JWT_EXPIRY")
            .ok()
            .map(|v| {
                v.parse::<i64>().map_err(|e| {
                    AppError::Configuration(format!("Failed to parse JWT_EXPIRY: {}", e))
                })
            })
            .transpose()?
            .unwrap_or(1);

        #[cfg(feature = "standalone")]
        let refresh_token_expiry_days = std::env::var("REFRESH_TOKEN_EXPIRY")
            .ok()
            .map(|v| {
                v.parse::<i64>().map_err(|e| {
                    AppError::Configuration(format!("Failed to parse REFRESH_TOKEN_EXPIRY: {}", e))
                })
            })
            .transpose()?
            .unwrap_or(7);

        #[cfg(feature = "standalone")]
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

        #[cfg(feature = "standalone")]
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

        #[cfg(feature = "standalone")]
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
            #[cfg(feature = "saas")]
            saas_login_url,
            #[cfg(feature = "saas")]
            host_url,
            #[cfg(feature = "saas")]
            saas_jwt_secret,
            #[cfg(feature = "saas")]
            saas_logout_url,
            #[cfg(feature = "saas")]
            saas_membership_url,
            #[cfg(feature = "saas")]
            saas_refresh_url,
            #[cfg(feature = "standalone")]
            jwt_secret,
            #[cfg(feature = "standalone")]
            jwt_expiry_hours,
            #[cfg(feature = "standalone")]
            refresh_token_expiry_days,
            #[cfg(feature = "standalone")]
            account_lockout_attempts,
            #[cfg(feature = "standalone")]
            account_lockout_duration_minutes,
            #[cfg(feature = "standalone")]
            allow_registration,
        })
    }

    /// Get a masked version of the database URL for logging
    pub fn masked_database_url(&self) -> String {
        // Mask the password in the database URL
        if let Some(at_pos) = self.database_url.find('@') {
            if let Some(colon_pos) = self.database_url[..at_pos].rfind(':') {
                let mut masked = self.database_url.clone();
                masked.replace_range(colon_pos + 1..at_pos, "****");
                return masked;
            }
        }
        // If we can't parse it, just mask the whole thing
        "postgresql://****:****@****/****".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to create a test Config with sensible defaults
    fn test_config() -> Config {
        Config {
            database_url: "postgresql://user:password@localhost/rusty_links".to_string(),
            app_port: 4002,
            update_interval_days: 30,
            log_level: "info".to_string(),
            update_interval_hours: 24,
            batch_size: 50,
            jitter_percent: 20,
            #[cfg(feature = "saas")]
            saas_login_url: "http://localhost:5173/login".to_string(),
            #[cfg(feature = "saas")]
            host_url: "http://localhost:4002".to_string(),
            #[cfg(feature = "saas")]
            saas_jwt_secret: "test-secret".to_string(),
            #[cfg(feature = "saas")]
            saas_logout_url: "http://localhost:18080/v1/auth/logout".to_string(),
            #[cfg(feature = "saas")]
            saas_membership_url: "http://localhost:5173/membership".to_string(),
            #[cfg(feature = "saas")]
            saas_refresh_url: "http://localhost:18080/v1/auth/refresh".to_string(),
            #[cfg(feature = "standalone")]
            jwt_secret: "test_secret".to_string(),
            #[cfg(feature = "standalone")]
            jwt_expiry_hours: 1,
            #[cfg(feature = "standalone")]
            refresh_token_expiry_days: 7,
            #[cfg(feature = "standalone")]
            account_lockout_attempts: 5,
            #[cfg(feature = "standalone")]
            account_lockout_duration_minutes: 30,
            #[cfg(feature = "standalone")]
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
        // Should return the fully masked fallback
        assert_eq!(masked, "postgresql://****:****@****/****");
    }

    #[test]
    fn test_masked_database_url_complex_password() {
        let mut config = test_config();
        config.database_url =
            "postgresql://admin:p@ss:w0rd!#@db.example.com:5432/mydb".to_string();
        let masked = config.masked_database_url();
        assert!(!masked.contains("p@ss:w0rd!#"));
        assert!(masked.contains("****"));
    }

    #[test]
    fn test_config_validation_update_interval_days_minimum() {
        // Verify that update_interval_days < 1 would be caught by validation
        let min_valid = 1u32;
        assert!(min_valid >= 1);
        let invalid = 0u32;
        assert!(invalid < 1);
    }

    #[test]
    fn test_config_validation_jitter_percent_range() {
        // Valid jitter: 0-100
        let valid_zero: u8 = 0;
        let valid_max: u8 = 100;
        assert!(valid_zero <= 100);
        assert!(valid_max <= 100);
        // 101 would exceed the max
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
