use anyhow::Result;
use std::sync::Arc;

use crate::usecases::{
    u501_import_from_ut, u502_import_from_ozon, u503_import_from_yandex,
    u504_import_from_wildberries,
};

use super::{
    logger::TaskLogger,
    managers::{
        U501ImportUtManager, U502ImportOzonManager, U503ImportYandexManager,
        U504ImportWildberriesManager,
    },
    registry::TaskManagerRegistry,
    worker::ScheduledTaskWorker,
};

/// Инициализирует реестр задач и фоновый воркер.
pub async fn initialize_scheduled_tasks() -> Result<ScheduledTaskWorker> {
    let mut registry = TaskManagerRegistry::new();
    let logger = Arc::new(TaskLogger::new("./task_logs"));

    // Register U501 Import from UT manager
    let u501_tracker = Arc::new(u501_import_from_ut::ProgressTracker::new());
    let u501_executor = Arc::new(u501_import_from_ut::ImportExecutor::new(u501_tracker));
    registry.register(U501ImportUtManager::new(u501_executor));

    // Register U502 Import from OZON manager
    let u502_tracker = Arc::new(u502_import_from_ozon::ProgressTracker::new());
    let u502_executor = Arc::new(u502_import_from_ozon::ImportExecutor::new(u502_tracker));
    registry.register(U502ImportOzonManager::new(u502_executor));

    // Register U503 Import from Yandex Market manager
    let u503_tracker = Arc::new(u503_import_from_yandex::ProgressTracker::new());
    let u503_executor = Arc::new(u503_import_from_yandex::ImportExecutor::new(u503_tracker));
    registry.register(U503ImportYandexManager::new(u503_executor));

    // Register U504 Import from Wildberries manager
    let u504_tracker = Arc::new(u504_import_from_wildberries::ProgressTracker::new());
    let u504_executor = Arc::new(u504_import_from_wildberries::ImportExecutor::new(u504_tracker));
    registry.register(U504ImportWildberriesManager::new(u504_executor));

    let worker = ScheduledTaskWorker::new(Arc::new(registry), logger, 60); // Check every 60 seconds
    Ok(worker)
}
