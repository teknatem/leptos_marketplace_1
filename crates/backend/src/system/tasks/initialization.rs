use anyhow::Result;
use std::sync::Arc;

use crate::usecases::{
    u501_import_from_ut, u502_import_from_ozon, u503_import_from_yandex,
    u504_import_from_wildberries,
};

use super::{
    logger::{get_global_task_logger, set_global_task_logger, TaskLogger},
    managers::{
        Task001WbOrdersFbsPollingManager, Task002WbOrdersStatsHourlyManager,
        Task003WbProductsManager, Task004WbSalesManager, Task005WbSuppliesManager,
        Task006WbFinanceManager, Task007WbCommissionsManager, Task008WbPricesManager,
        Task009WbPromotionsManager, Task010WbDocumentsManager, Task011WbAdvertManager,
        Task012WbAdvertCampaignsManager, U501ImportUtManager, U502ImportOzonManager,
        U503ImportYandexManager,
    },
    registry::{set_global_registry, TaskManagerRegistry},
    worker::ScheduledTaskWorker,
};

/// Создаёт пару (ProgressTracker, ImportExecutor) для u504 — каждая WB-задача
/// получает собственную пару, чтобы прогресс-трекеры не конфликтовали.
macro_rules! wb_executor {
    () => {{
        let tracker = Arc::new(u504_import_from_wildberries::ProgressTracker::new());
        Arc::new(u504_import_from_wildberries::ImportExecutor::new(tracker))
    }};
}

/// Инициализирует реестр задач и фоновый воркер.
pub async fn initialize_scheduled_tasks() -> Result<ScheduledTaskWorker> {
    let mut registry = TaskManagerRegistry::new();
    // Гарантируем, что глобальный логгер создан с конфигурацией воркера.
    // Если хендлеры уже вызвали get_global_task_logger() раньше — вызов игнорируется.
    set_global_task_logger(Arc::new(TaskLogger::new("./task_logs")));
    let logger = get_global_task_logger();

    // ---- Non-WB usecases ----

    let u501_tracker = Arc::new(u501_import_from_ut::ProgressTracker::new());
    let u501_executor = Arc::new(u501_import_from_ut::ImportExecutor::new(u501_tracker));
    registry.register(U501ImportUtManager::new(u501_executor));

    let u502_tracker = Arc::new(u502_import_from_ozon::ProgressTracker::new());
    let u502_executor = Arc::new(u502_import_from_ozon::ImportExecutor::new(u502_tracker));
    registry.register(U502ImportOzonManager::new(u502_executor));

    let u503_tracker = Arc::new(u503_import_from_yandex::ProgressTracker::new());
    let u503_executor = Arc::new(u503_import_from_yandex::ImportExecutor::new(u503_tracker));
    registry.register(U503ImportYandexManager::new(u503_executor));

    // ---- WB atomic task managers — each owns its own executor + progress tracker ----

    registry.register(Task001WbOrdersFbsPollingManager::new(wb_executor!()));
    registry.register(Task002WbOrdersStatsHourlyManager::new(wb_executor!()));
    registry.register(Task003WbProductsManager::new(wb_executor!()));
    registry.register(Task004WbSalesManager::new(wb_executor!()));
    registry.register(Task005WbSuppliesManager::new(wb_executor!()));
    registry.register(Task006WbFinanceManager::new(wb_executor!()));
    registry.register(Task007WbCommissionsManager::new(wb_executor!()));
    registry.register(Task008WbPricesManager::new(wb_executor!()));
    registry.register(Task009WbPromotionsManager::new(wb_executor!()));
    registry.register(Task010WbDocumentsManager::new(wb_executor!()));
    registry.register(Task011WbAdvertManager::new(wb_executor!()));
    registry.register(Task012WbAdvertCampaignsManager::new(wb_executor!()));

    let registry = Arc::new(registry);
    set_global_registry(Arc::clone(&registry));

    let worker = ScheduledTaskWorker::new(registry, logger, 60);
    Ok(worker)
}
