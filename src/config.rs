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
        // Load .env file if it exists (ignore errors if file doesn't exist)
        let _ = dotenvy::dotenv();

        // Load required variables
        let database_url = std::env::var("DATABASE_URL").map_err(|_| {
            AppError::Configuration("Missing required environment variable: DATABASE_URL".to_string())
        })?;

        let app_port = std::env::var("APP_PORT")
            .map_err(|_| {
                AppError::Configuration("Missing required environment variable: APP_PORT".to_string())
            })?
            .parse::<u16>()
            .map_err(|e| {
                AppError::Configuration(format!("Failed to parse APP_PORT: {}", e))
            })?;

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

        let log_level = std::env::var("RUST_LOG")
            .unwrap_or_else(|_| "info".to_string());

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

        Ok(Config {
            database_url,
            app_port,
            update_interval_days,
            log_level,
            update_interval_hours,
            batch_size,
            jitter_percent,
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

    #[test]
    fn test_masked_database_url() {
        let config = Config {
            database_url: "postgresql://user:password@localhost/rusty_links".to_string(),
            app_port: 8080,
            update_interval_days: 30,
            log_level: "info".to_string(),
            update_interval_hours: 24,
            batch_size: 50,
            jitter_percent: 20,
        };

        let masked = config.masked_database_url();
        assert!(!masked.contains("password"));
        assert!(masked.contains("****"));
    }
}
