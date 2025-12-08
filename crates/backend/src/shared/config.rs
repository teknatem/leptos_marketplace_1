use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub database: DatabaseConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub path: String,
}

/// Default configuration embedded in the binary
const DEFAULT_CONFIG: &str = r#"
[database]
path = "target/db/app.db"
"#;

/// Load configuration from config.toml file
/// 
/// Search order:
/// 1. Next to the executable (for production)
/// 2. Falls back to embedded default config
pub fn load_config() -> anyhow::Result<Config> {
    // Try to find config.toml next to the executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let config_path = exe_dir.join("config.toml");
            
            if config_path.exists() {
                tracing::info!("Loading config from: {}", config_path.display());
                let contents = std::fs::read_to_string(&config_path)?;
                let config: Config = toml::from_str(&contents)?;
                return Ok(config);
            } else {
                tracing::warn!("config.toml not found at: {}", config_path.display());
            }
        }
    }
    
    // Fall back to default config
    tracing::info!("Using default embedded configuration");
    let config: Config = toml::from_str(DEFAULT_CONFIG)?;
    Ok(config)
}

/// Get the database file path from configuration
/// Resolves relative paths relative to the executable directory
pub fn get_database_path(config: &Config) -> anyhow::Result<PathBuf> {
    let db_path_str = &config.database.path;
    let db_path = Path::new(db_path_str);
    
    // If absolute path, use as is
    if db_path.is_absolute() {
        return Ok(db_path.to_path_buf());
    }
    
    // If relative path, resolve it relative to the executable directory
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let resolved_path = exe_dir.join(db_path);
            return Ok(resolved_path);
        }
    }
    
    // Fallback: use relative to current directory
    Ok(PathBuf::from(db_path_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_loads() {
        let config: Result<Config, _> = toml::from_str(DEFAULT_CONFIG);
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.database.path, "target/db/app.db");
    }
}

