use super::repository;
use chrono::Utc;
use contracts::domain::a006_connection_mp::aggregate::{
    ConnectionMP, ConnectionMPDto, ConnectionTestResult,
};
use uuid::Uuid;

/// Создание нового подключения к маркетплейсу
pub async fn create(dto: ConnectionMPDto) -> anyhow::Result<Uuid> {
    let code = dto.code.clone().unwrap_or_else(|| format!("MP-{}", Uuid::new_v4()));
    let mut aggregate = ConnectionMP::new_for_insert(
        code,
        dto.description,
        dto.marketplace_id,
        dto.organization,
        dto.api_key,
        dto.comment,
    );

    // Обновляем остальные поля
    aggregate.supplier_id = dto.supplier_id;
    aggregate.application_id = dto.application_id;
    aggregate.is_used = dto.is_used;
    aggregate.business_account_id = dto.business_account_id;
    aggregate.api_key_stats = dto.api_key_stats;
    aggregate.test_mode = dto.test_mode;
    aggregate.authorization_type = dto.authorization_type;

    // Валидация
    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    // Before write
    aggregate.before_write();

    // Сохранение через repository
    repository::insert(&aggregate).await
}

/// Обновление существующего подключения
pub async fn update(dto: ConnectionMPDto) -> anyhow::Result<()> {
    let id = dto
        .id
        .as_ref()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| anyhow::anyhow!("Invalid ID"))?;

    let mut aggregate = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Not found"))?;

    aggregate.update(&dto);

    // Валидация
    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    // Before write
    aggregate.before_write();

    // Сохранение
    repository::update(&aggregate).await
}

/// Мягкое удаление подключения
pub async fn delete(id: Uuid) -> anyhow::Result<bool> {
    repository::soft_delete(id).await
}

/// Получение подключения по ID
pub async fn get_by_id(id: Uuid) -> anyhow::Result<Option<ConnectionMP>> {
    repository::get_by_id(id).await
}

/// Получение списка всех подключений
pub async fn list_all() -> anyhow::Result<Vec<ConnectionMP>> {
    repository::list_all().await
}

/// Тестирование подключения к маркетплейсу
pub async fn test_connection(dto: ConnectionMPDto) -> anyhow::Result<ConnectionTestResult> {
    let start = std::time::Instant::now();

    // Валидация базовых данных
    if dto.api_key.trim().is_empty() {
        return Ok(ConnectionTestResult {
            success: false,
            message: "API Key не может быть пустым".into(),
            duration_ms: 0,
            tested_at: Utc::now(),
        });
    }

    // Заглушка - просто проверяем что данные заполнены
    let duration = start.elapsed();

    Ok(ConnectionTestResult {
        success: true,
        message: "Тестирование пока не реализовано полностью".into(),
        duration_ms: duration.as_millis() as u64,
        tested_at: Utc::now(),
    })
}
