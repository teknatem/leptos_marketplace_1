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

/// Default configuration embedded in the binary
const DEFAULT_CONFIG: &str = r#"
[database]
path = "target/db/app.db"

[scheduled_tasks]
enabled = true

[llm]
knowledge_base_path = "data/knowledge"
"#;

fn default_true() -> bool {
    true
}

/// Load configuration from config.toml file
///
/// Search order:
/// 1. Next to the executable (for production)
/// 2. Falls back to embedded default config
pub fn load_config() -> anyhow::Result<Config> {
    println!("\n========================================");
    println!("  CONFIGURATION LOADING DIAGNOSTICS");
    println!("========================================\n");

    // Try to find config.toml next to the executable
    if let Ok(exe_path) = std::env::current_exe() {
        println!("✓ Executable path: {}", exe_path.display());

        if let Some(exe_dir) = exe_path.parent() {
            println!("✓ Executable directory: {}", exe_dir.display());

            let config_path = exe_dir.join("config.toml");
            println!("✓ Looking for config at: {}\n", config_path.display());

            // Check if file exists
            if config_path.exists() {
                println!("✓ Config file found!");

                // Check file metadata
                match std::fs::metadata(&config_path) {
                    Ok(metadata) => {
                        println!("✓ File size: {} bytes", metadata.len());
                        println!("✓ File is read-only: {}", metadata.permissions().readonly());

                        #[cfg(windows)]
                        {
                            use std::os::windows::fs::MetadataExt;
                            println!("✓ File attributes: 0x{:X}", metadata.file_attributes());
                        }
                    }
                    Err(e) => {
                        println!("✗ ERROR: Cannot read file metadata: {}", e);
                        println!("  This might be a permissions issue.\n");
                        return Err(anyhow::anyhow!("Cannot access file metadata: {}", e));
                    }
                }

                // Try to read file contents
                println!("\nAttempting to read file...");
                let contents = match std::fs::read_to_string(&config_path) {
                    Ok(c) => {
                        println!("✓ File read successfully ({} characters)", c.len());
                        c
                    }
                    Err(e) => {
                        println!("✗ ERROR: Cannot read file: {}", e);
                        println!("  Error kind: {:?}", e.kind());
                        println!("  Possible causes:");
                        println!("  - Insufficient permissions");
                        println!("  - File is locked by another process");
                        println!("  - Antivirus blocking access\n");
                        return Err(anyhow::anyhow!("Cannot read config file: {}", e));
                    }
                };

                // Try to parse TOML
                println!("\nAttempting to parse TOML...");
                let config: Config = match toml::from_str(&contents) {
                    Ok(c) => {
                        println!("✓ TOML parsed successfully");
                        c
                    }
                    Err(e) => {
                        println!("✗ ERROR: Invalid TOML format: {}", e);
                        println!("\nFile contents:");
                        println!("---BEGIN---");
                        println!("{}", contents);
                        println!("---END---\n");

                        // Check for common TOML syntax errors
                        let error_str = e.to_string();
                        if error_str.contains("invalid escape") || error_str.contains("unicode") {
                            println!(
                                "╔══════════════════════════════════════════════════════════╗"
                            );
                            println!("║  DETECTED: Invalid escape sequences in TOML!           ║");
                            println!(
                                "╚══════════════════════════════════════════════════════════╝"
                            );
                            println!("\n⚠  PROBLEM:");
                            println!("   TOML uses backslashes (\\) for escape sequences.");
                            println!("   Single backslashes in strings must be escaped.\n");
                            println!("✓ SOLUTION - Use one of these formats:\n");
                            println!("   Option 1 (RECOMMENDED): Use forward slashes");
                            println!("   path = \"C:/Users/udv/Desktop/MPI/data/app.db\"\n");
                            println!("   Option 2: Use single quotes (literal string)");
                            println!("   path = 'C:\\Users\\udv\\Desktop\\MPI\\data\\app.db'\n");
                            println!("   Option 3: Double all backslashes");
                            println!("   path = \"C:\\\\Users\\\\udv\\\\Desktop\\\\MPI\\\\data\\\\app.db\"\n");
                            println!(
                                "NOTE: After parsing, Windows paths are automatically normalized to forward slashes."
                            );
                            println!("========================================\n");
                        }

                        return Err(anyhow::anyhow!("Invalid TOML: {}", e));
                    }
                };

                println!("✓ Database path from config: {}", config.database.path);
                println!(
                    "✓ Scheduled task worker enabled: {}",
                    config.scheduled_tasks.enabled
                );

                // Информируем о нормализации путей
                if config.database.path.contains('\\') {
                    println!("ℹ Note: Backslashes in paths are automatically converted to forward slashes");
                }

                println!("\n✓ Configuration loaded successfully!\n");
                println!("========================================\n");

                tracing::info!("Config loaded from: {}", config_path.display());
                return Ok(config);
            } else {
                println!("✗ Config file NOT found at expected location");
                println!("  Will use default embedded configuration\n");
                tracing::warn!("config.toml not found at: {}", config_path.display());
            }
        } else {
            println!("✗ Cannot determine executable directory\n");
        }
    } else {
        println!("✗ Cannot determine executable path\n");
    }

    // Fall back to default config
    println!("Using default embedded configuration:");
    println!("---BEGIN DEFAULT CONFIG---");
    println!("{}", DEFAULT_CONFIG);
    println!("---END DEFAULT CONFIG---\n");
    println!("========================================\n");

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
    fn test_default_config_loads() {
        let config: Result<Config, _> = toml::from_str(DEFAULT_CONFIG);
        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.database.path, "target/db/app.db");
        assert!(config.scheduled_tasks.enabled);
    }
}
