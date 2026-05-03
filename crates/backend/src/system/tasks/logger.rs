use anyhow::Result;
use chrono::Utc;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::{Arc, OnceLock};

/// Логгер для записи информации о выполнении задачи в файл.
/// Каждый лог-файл привязан к конкретной сессии выполнения задачи.
pub struct TaskLogger {
    log_dir: String,
}

impl TaskLogger {
    pub fn new(base_log_dir: &str) -> Self {
        Self {
            log_dir: format!("{}/task_logs", base_log_dir),
        }
    }

    /// Создает директорию для логов, если она не существует.
    fn ensure_log_dir_exists(&self) -> Result<()> {
        fs::create_dir_all(&self.log_dir)?;
        Ok(())
    }

    /// Возвращает путь к лог-файлу для данной сессии.
    pub fn get_log_file_path(&self, session_id: &str) -> String {
        format!("{}/{}.log", self.log_dir, session_id)
    }

    /// Записывает сообщение в лог-файл для указанной сессии.
    /// Если файл не существует, он будет создан.
    pub fn write_log(&self, session_id: &str, message: &str) -> Result<()> {
        self.ensure_log_dir_exists()?;
        let file_path = self.get_log_file_path(session_id);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;

        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
        writeln!(file, "[{}] {}", timestamp, message)?;
        Ok(())
    }

    /// Читает все содержимое лог-файла для указанной сессии.
    pub fn read_log(&self, session_id: &str) -> Result<String> {
        let file_path = self.get_log_file_path(session_id);
        if Path::new(&file_path).exists() {
            Ok(fs::read_to_string(&file_path)?)
        } else {
            Ok(format!("Log file for session {} not found.", session_id))
        }
    }

    /// Удаляет лог-файл для указанной сессии.
    pub fn delete_log(&self, session_id: &str) -> Result<()> {
        let file_path = self.get_log_file_path(session_id);
        if Path::new(&file_path).exists() {
            fs::remove_file(&file_path)?;
        }
        Ok(())
    }
}

// ── Глобальный синглтон ──────────────────────────────────────────────────────

static GLOBAL_TASK_LOGGER: OnceLock<Arc<TaskLogger>> = OnceLock::new();

/// Возвращает единственный экземпляр `TaskLogger` для всего процесса.
///
/// При первом вызове создаёт логгер с путём `./task_logs`.  
/// Если вызвать до `set_global_task_logger`, возвращает значение по умолчанию.
pub fn get_global_task_logger() -> Arc<TaskLogger> {
    GLOBAL_TASK_LOGGER
        .get_or_init(|| Arc::new(TaskLogger::new("./task_logs")))
        .clone()
}

/// Устанавливает глобальный логгер (из `initialization`).
/// Если логгер уже был создан вызовом `get_global_task_logger`, вызов игнорируется.
pub fn set_global_task_logger(logger: Arc<TaskLogger>) {
    let _ = GLOBAL_TASK_LOGGER.set(logger);
}
