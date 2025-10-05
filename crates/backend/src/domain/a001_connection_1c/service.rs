use super::repository;
use chrono::Utc;
use contracts::domain::a001_connection_1c::aggregate::{
    Connection1CDatabase, Connection1CDatabaseDto, ConnectionTestResult,
};
use uuid::Uuid;

/// Создание нового подключения к 1C
pub async fn create(dto: Connection1CDatabaseDto) -> anyhow::Result<Uuid> {
    let code = dto.code.clone().unwrap_or_else(|| format!("CON-{}", Uuid::new_v4()));
    let mut aggregate = Connection1CDatabase::new_for_insert(
        code,
        dto.description,
        dto.url,
        dto.comment,
        dto.login,
        dto.password,
        dto.is_primary,
    );

    // Валидация
    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;

    // Before write
    aggregate.before_write();

    // Бизнес-логика: обеспечение единственности primary
    if aggregate.is_primary {
        repository::clear_other_primary_flags(None).await?;
    }

    // Сохранение через repository
    repository::insert(&aggregate).await
}

/// Обновление существующего подключения
pub async fn update(dto: Connection1CDatabaseDto) -> anyhow::Result<()> {
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

    // Бизнес-логика: обеспечение единственности primary
    if aggregate.is_primary {
        repository::clear_other_primary_flags(Some(id)).await?;
    }

    // Сохранение
    repository::update(&aggregate).await
}

/// Мягкое удаление подключения
pub async fn delete(id: Uuid) -> anyhow::Result<bool> {
    repository::soft_delete(id).await
}

/// Получение подключения по ID
pub async fn get_by_id(id: Uuid) -> anyhow::Result<Option<Connection1CDatabase>> {
    repository::get_by_id(id).await
}

/// Получение списка всех подключений
pub async fn list_all() -> anyhow::Result<Vec<Connection1CDatabase>> {
    repository::list_all().await
}

/// Получение основного подключения
pub async fn get_primary() -> anyhow::Result<Option<Connection1CDatabase>> {
    repository::get_primary().await
}

/// Вставка тестовых данных
pub async fn insert_test_data() -> anyhow::Result<()> {
    let data = vec![
        Connection1CDatabaseDto {
            id: None,
            code: Some("CON-PROD-001".into()),
            description: "Test Production 1C Server".into(),
            url: "http://192.168.1.10/test_base/odata/standard.odata".into(),
            comment: Some("Main production server for testing".into()),
            login: "test_user".into(),
            password: "test_password".into(),
            is_primary: true,
        },
        Connection1CDatabaseDto {
            id: None,
            code: Some("CON-DEV-001".into()),
            description: "Development 1C Environment".into(),
            url: "http://dev.company.local/dev_base/odata/standard.odata".into(),
            comment: Some("Development environment for 1C integration".into()),
            login: "dev_user".into(),
            password: "dev_password".into(),
            is_primary: false,
        },
    ];

    for dto in data {
        create(dto).await?;
    }

    Ok(())
}

/// Тестирование подключения к 1C
pub async fn test_connection(dto: Connection1CDatabaseDto) -> anyhow::Result<ConnectionTestResult> {
    let start = std::time::Instant::now();

    // Валидация базовых данных
    if dto.url.trim().is_empty() {
        return Ok(ConnectionTestResult {
            success: false,
            message: "URL не может быть пустым".into(),
            duration_ms: 0,
            tested_at: Utc::now(),
        });
    }

    if dto.login.trim().is_empty() {
        return Ok(ConnectionTestResult {
            success: false,
            message: "Логин не может быть пустым".into(),
            duration_ms: 0,
            tested_at: Utc::now(),
        });
    }

    // Попытка подключения к 1C OData
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client
        .get(&dto.url)
        .basic_auth(&dto.login, Some(&dto.password))
        .send()
        .await;

    let duration = start.elapsed();

    match response {
        Ok(resp) => {
            let status = resp.status();
            let success = status.is_success() || status.as_u16() == 401; // 401 означает что сервер ответил, но нужна аутентификация

            let message = if status.is_success() {
                "Подключение успешно".into()
            } else if status.as_u16() == 401 {
                "Сервер доступен, но требуется проверка учетных данных".into()
            } else {
                format!("Сервер вернул статус: {}", status)
            };

            Ok(ConnectionTestResult {
                success,
                message,
                duration_ms: duration.as_millis() as u64,
                tested_at: Utc::now(),
            })
        }
        Err(e) => {
            let message = if e.is_timeout() {
                "Превышено время ожидания (10 сек)".into()
            } else if e.is_connect() {
                format!("Ошибка подключения: не удается достичь сервер")
            } else {
                format!("Ошибка: {}", e)
            };

            Ok(ConnectionTestResult {
                success: false,
                message,
                duration_ms: duration.as_millis() as u64,
                tested_at: Utc::now(),
            })
        }
    }
}
