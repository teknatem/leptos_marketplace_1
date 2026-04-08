use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub database: DatabaseConfig,
    #[serde(default)]
    pub scheduled_tasks: ScheduledTasksConfig,
    #[serde(default)]
    pub llm: LlmConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LlmConfig {
    /// Путь к директории с MD-файлами базы знаний (Obsidian-формат).
    /// Относительный путь разрешается от директории бинарника.
    pub knowledge_base_path: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            knowledge_base_path: "data/knowledge".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    #[serde(deserialize_with = "normalize_path")]
    pub path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ScheduledTasksConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for ScheduledTasksConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

/// Нормализует пути Windows: конвертирует обратные слеши в прямые
fn normalize_path<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let path = String::deserialize(deserializer)?;

    // Проверяем, является ли это Windows путем (содержит двоеточие и слеши)
    if path.len() >= 3 && path.chars().nth(1) == Some(':') {
        // Это Windows абсолютный путь (C:\... или C:/...)
        Ok(path.replace('\\', "/"))
    } else {
        Ok(path)
    }
}

const CONFIG_FILE_NAME: &str = "config.toml";

fn default_true() -> bool {
    true
}

pub fn get_config_path() -> anyhow::Result<PathBuf> {
    let exe_path = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("Cannot determine executable path: {}", e))?;
    let exe_dir = exe_path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine executable directory"))?;
    let config_path = exe_dir.join(CONFIG_FILE_NAME);

    if !config_path.exists() {
        return Err(anyhow::anyhow!(
            "Required config file not found: {}. Database path must be configured explicitly in config.toml.",
            config_path.display()
        ));
    }

    Ok(config_path)
}

fn validate_config(config: &Config) -> anyhow::Result<()> {
    let raw_db_path = config.database.path.trim();
    if raw_db_path.is_empty() {
        return Err(anyhow::anyhow!(
            "[database].path must be set in config.toml"
        ));
    }

    let db_path = Path::new(raw_db_path);
    if !db_path.is_absolute() {
        return Err(anyhow::anyhow!(
            "[database].path must be an absolute path. Got '{}'.",
            raw_db_path
        ));
    }

    Ok(())
}

/// Load configuration from required config.toml next to the executable.
pub fn load_config() -> anyhow::Result<Config> {
    let config_path = get_config_path()?;
    let contents = std::fs::read_to_string(&config_path)
        .map_err(|e| anyhow::anyhow!("Cannot read config file {}: {}", config_path.display(), e))?;
    let config: Config = toml::from_str(&contents)
        .map_err(|e| anyhow::anyhow!("Invalid TOML in {}: {}", config_path.display(), e))?;

    validate_config(&config)?;

    let database_path = get_database_path(&config)?;
    println!("\n========================================");
    println!("  CONFIGURATION LOADING DIAGNOSTICS");
    println!("========================================");
    println!("✓ Config file: {}", config_path.display());
    println!("✓ Database path: {}", database_path.display());
    println!(
        "✓ Scheduled task worker enabled: {}",
        config.scheduled_tasks.enabled
    );
    println!("========================================\n");

    tracing::info!("Config loaded from: {}", config_path.display());
    tracing::info!("Resolved database path: {}", database_path.display());
    Ok(config)
}

/// Get the database file path from configuration
pub fn get_database_path(config: &Config) -> anyhow::Result<PathBuf> {
    let db_path_str = &config.database.path;
    let db_path = Path::new(db_path_str);

    if !db_path.is_absolute() {
        return Err(anyhow::anyhow!(
            "[database].path must be absolute, got '{}'",
            db_path_str
        ));
    }

    Ok(db_path.to_path_buf())
}

/// Get the knowledge base directory path from configuration.
/// Resolves relative paths relative to the executable directory.
pub fn get_knowledge_base_path(config: &Config) -> PathBuf {
    let raw = &config.llm.knowledge_base_path;
    let p = Path::new(raw);
    if p.is_absolute() {
        return p.to_path_buf();
    }
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            return exe_dir.join(p);
        }
    }
    PathBuf::from(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_database_path_is_accepted() {
        let config: Config = toml::from_str(
            r#"
[database]
path = "E:/dev/rust/leptos_marketplace_1/data/app.db"

[scheduled_tasks]
enabled = true

[llm]
knowledge_base_path = "data/knowledge"
"#,
        )
        .unwrap();

        assert!(validate_config(&config).is_ok());
        assert_eq!(
            get_database_path(&config).unwrap(),
            PathBuf::from("E:/dev/rust/leptos_marketplace_1/data/app.db")
        );
        assert!(config.scheduled_tasks.enabled);
    }

    #[test]
    fn relative_database_path_is_rejected() {
        let config: Config = toml::from_str(
            r#"
[database]
path = "target/db/app.db"

[scheduled_tasks]
enabled = true

[llm]
knowledge_base_path = "data/knowledge"
"#,
        )
        .unwrap();

        assert!(validate_config(&config).is_err());
        assert!(get_database_path(&config).is_err());
    }
}
