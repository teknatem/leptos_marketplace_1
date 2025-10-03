use super::repository;
use contracts::domain::connection_1c::aggregate::{
    Connection1CDatabase, Connection1CDatabaseForm, ConnectionTestResult,
};
use chrono::Utc;

/// Создание нового подключения к 1C
pub async fn create(form: Connection1CDatabaseForm) -> anyhow::Result<i32> {
    let mut aggregate = Connection1CDatabase::new_for_insert(
        form.description,
        form.url,
        form.comment,
        form.login,
        form.password,
        form.is_primary,
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
pub async fn update(form: Connection1CDatabaseForm) -> anyhow::Result<()> {
    let id = form
        .id
        .as_ref()
        .and_then(|s| s.parse::<i32>().ok())
        .ok_or_else(|| anyhow::anyhow!("Invalid ID"))?;

    let mut aggregate = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Not found"))?;

    aggregate.update_from_form(&form);

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
pub async fn delete(id: i32) -> anyhow::Result<bool> {
    repository::soft_delete(id).await
}

/// Получение подключения по ID
pub async fn get_by_id(id: i32) -> anyhow::Result<Option<Connection1CDatabase>> {
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
        Connection1CDatabaseForm {
            id: None,
            description: "Test Production 1C Server".into(),
            url: "http://192.168.1.10/test_base/odata/standard.odata".into(),
            comment: Some("Main production server for testing".into()),
            login: "test_user".into(),
            password: "test_password".into(),
            is_primary: true,
        },
        Connection1CDatabaseForm {
            id: None,
            description: "Development 1C Environment".into(),
            url: "http://dev.company.local/dev_base/odata/standard.odata".into(),
            comment: Some("Development environment for 1C integration".into()),
            login: "dev_user".into(),
            password: "dev_password".into(),
            is_primary: false,
        },
    ];

    for form in data {
        create(form).await?;
    }

    Ok(())
}

/// Тестирование подключения к 1C
pub async fn test_connection(form: Connection1CDatabaseForm) -> anyhow::Result<ConnectionTestResult> {
    let start = std::time::Instant::now();

    // Валидация базовых данных
    if form.url.trim().is_empty() {
        return Ok(ConnectionTestResult {
            success: false,
            message: "URL не может быть пустым".into(),
            duration_ms: 0,
            tested_at: Utc::now(),
        });
    }

    if form.login.trim().is_empty() {
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
        .get(&form.url)
        .basic_auth(&form.login, Some(&form.password))
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
