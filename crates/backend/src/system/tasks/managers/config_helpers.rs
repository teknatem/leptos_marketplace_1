use anyhow::{bail, Context, Result};
use chrono::{Duration, NaiveDate, Utc};
use contracts::system::tasks::aggregate::ScheduledTask;
use uuid::Uuid;

/// Парсит `connection_id` из конфига задачи с понятным сообщением об ошибке.
///
/// Отдельно проверяет незаполненные заглушки вида `REPLACE_WITH_...`,
/// которые оставляются в config_json при сидировании задач из миграций.
pub fn parse_connection_id(raw: &str, label: &str) -> Result<Uuid> {
    if raw.is_empty() {
        bail!(
            "Поле «connection_id» не заполнено в конфиге задачи. \
             Откройте карточку задачи и выберите подключение к {label}."
        );
    }
    if raw.starts_with("REPLACE_") {
        bail!(
            "Поле «connection_id» содержит заглушку «{raw}». \
             Откройте карточку задачи и выберите реальное подключение к {label}."
        );
    }
    Uuid::parse_str(raw).with_context(|| {
        format!(
            "connection_id «{raw}» — не валидный UUID. \
             Откройте карточку задачи и выберите подключение к {label}."
        )
    })
}

/// Вычисляет окно загрузки `(date_from, date_to)` на основе watermark задачи.
///
/// Логика:
/// - Если watermark (`data_loaded_up_to`) не установлен → `date_from = work_start_date`
/// - Иначе → `date_from = max(watermark.date - overlap_days, work_start_date)`
/// - `date_to = min(date_from + chunk_days, today)` — ограничиваем порцию загрузки
/// - Для существующих записей до миграции используется fallback на `last_successful_run_at.date`
///
/// Администратор сбрасывает watermark в UI, после чего задача начинает с `work_start_date`
/// и загружает по `chunk_days` дней за запуск, пока не догонит сегодня.
pub fn compute_date_window(
    task: &ScheduledTask,
    work_start_date: &str,
    overlap_days: i64,
    chunk_days: i64,
) -> (NaiveDate, NaiveDate) {
    let today = Utc::now().date_naive();
    let work_start = NaiveDate::parse_from_str(work_start_date, "%Y-%m-%d")
        .unwrap_or_else(|_| NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());

    let watermark_date = task
        .data_loaded_up_to
        .or_else(|| task.last_successful_run_at.map(|last| last.date_naive()));

    let date_from = watermark_date
        .map(|date| (date - Duration::days(overlap_days)).max(work_start))
        .unwrap_or(work_start)
        .min(today); // guard: watermark never set to the future, but be safe

    let date_to = (date_from + Duration::days(chunk_days)).min(today);
    (date_from, date_to)
}
