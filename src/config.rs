use std::fmt;

/// Application configuration
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub app_port: u16,
    pub update_interval_days: u32,
    pub log_level: String,
}

/// Configuration errors
#[derive(Debug)]
pub enum ConfigError {
    MissingRequired(String),
    InvalidValue(String, String),
    ParseError(String, String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::MissingRequired(var) => {
                write!(f, "Missing required environment variable: {}", var)
            }
            ConfigError::InvalidValue(var, reason) => {
                write!(f, "Invalid value for {}: {}", var, reason)
            }
            ConfigError::ParseError(var, err) => {
                write!(f, "Failed to parse {}: {}", var, err)
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl Config {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self, ConfigError> {
        // Load .env file if it exists (ignore errors if file doesn't exist)
        let _ = dotenvy::dotenv();

        // Load required variables
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| ConfigError::MissingRequired("DATABASE_URL".to_string()))?;

        let app_port = std::env::var("APP_PORT")
            .map_err(|_| ConfigError::MissingRequired("APP_PORT".to_string()))?
            .parse::<u16>()
            .map_err(|e| {
                ConfigError::ParseError("APP_PORT".to_string(), e.to_string())
            })?;

        // Load optional variables with defaults
        let update_interval_days = std::env::var("UPDATE_INTERVAL_DAYS")
            .ok()
            .map(|v| {
                v.parse::<u32>().map_err(|e| {
                    ConfigError::ParseError("UPDATE_INTERVAL_DAYS".to_string(), e.to_string())
                })
            })
            .transpose()?
            .unwrap_or(30);

        // Validate update_interval_days
        if update_interval_days < 1 {
            return Err(ConfigError::InvalidValue(
                "UPDATE_INTERVAL_DAYS".to_string(),
                "must be at least 1".to_string(),
            ));
        }

        let log_level = std::env::var("RUST_LOG")
            .unwrap_or_else(|_| "info".to_string());

        Ok(Config {
            database_url,
            app_port,
            update_interval_days,
            log_level,
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
        };

        let masked = config.masked_database_url();
        assert!(!masked.contains("password"));
        assert!(masked.contains("****"));
    }
}
