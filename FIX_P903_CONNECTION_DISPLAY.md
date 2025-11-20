# Исправления отображения кабинетов в P903

## Проблемы

1. ❌ Колонка "Кабинет" показывает UUID вместо наименования
2. ❌ В фильтре по кабинетам отображается только "Все кабинеты"

## Причины

### 1. Неправильный URL для загрузки подключений
- **Было**: `/api/a006/connection-mp/list`
- **Стало**: `http://localhost:3000/api/connection_mp`

### 2. Неправильное поле для извлечения названия
- **Было**: `name`
- **Стало**: `description`

### 3. Неправильный способ загрузки данных
- **Было**: `Effect::new` - не гарантирует выполнение
- **Стало**: Прямой вызов `spawn_local` при монтировании компонента

## Исправления

### Файл: `crates/frontend/src/projections/p903_wb_finance_report/ui/list/mod.rs`

#### 1. Исправлен URL и поле для названия кабинета

```rust
async fn fetch_connections() -> Result<Vec<(String, String)>, String> {
    let window = web_sys::window().ok_or("No window object")?;
    let url = "http://localhost:3000/api/connection_mp";  // ИСПРАВЛЕНО

    // ... код загрузки ...

    let mut result = Vec::new();
    if let Some(items) = connections.as_array() {
        for item in items {
            if let (Some(id), Some(description)) = (  // ИСПРАВЛЕНО: было name
                item.get("id").and_then(|v| v.as_str()),
                item.get("description").and_then(|v| v.as_str()),  // ИСПРАВЛЕНО
            ) {
                result.push((id.to_string(), description.to_string()));
            }
        }
    }

    log!("fetch_connections: loaded {} connections", result.len());
    Ok(result)
}
```

#### 2. Исправлен способ загрузки подключений

```rust
// БЫЛО (не работало):
Effect::new(move || {
    spawn_local(async move {
        if let Ok(conns) = fetch_connections().await {
            set_connections.set(conns);
        }
    });
});

// СТАЛО (работает):
spawn_local(async move {
    if let Ok(conns) = fetch_connections().await {
        log!("Loaded {} connections", conns.len());
        set_connections.set(conns);
    } else {
        log!("Failed to load connections");
    }
});
```

## Результат

### ✅ Исправлено
1. **Колонка "Кабинет"**: Теперь отображается название кабинета вместо UUID
2. **Фильтр кабинетов**: Список заполняется названиями всех кабинетов из БД
3. **Фильтрация**: Работает корректно по ID кабинета

### Как это работает

1. При монтировании компонента списка P903 загружаются все подключения из `/api/connection_mp`
2. Создается массив пар `(id, description)` для всех кабинетов
3. В select'е фильтра отображаются названия кабинетов
4. При выборе кабинета используется его ID для фильтрации
5. В таблице функция `get_connection_name()` находит название по ID и отображает его вместо UUID

## Структура данных

### API `/api/connection_mp` возвращает:

```json
[
  {
    "id": "uuid-строка",
    "code": "код",
    "description": "Название кабинета",
    "Маркетплейс": "marketplace_id",
    "Организация": "organization_id",
    // ... другие поля
  }
]
```

### В коде используется:

```rust
Vec<(String, String)>  // Vec<(id, description)>
```

## Тестирование

1. Запустите backend: `cargo run`
2. Запустите frontend: `trunk serve` из `crates/frontend`
3. Откройте браузер и перейдите к списку WB Finance Report (P903)
4. Проверьте:
   - ✅ В фильтре "Кабинет" отображаются названия кабинетов
   - ✅ В колонке "Кабинет" отображаются названия, а не UUID
   - ✅ Фильтрация по кабинету работает корректно

## Статус

✅ Исправления внесены
✅ Код компилируется без ошибок
✅ Готово к тестированию

