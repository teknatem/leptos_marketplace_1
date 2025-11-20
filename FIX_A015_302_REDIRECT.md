# Исправление: Ошибка 302 Found при загрузке WB Orders

**Дата:** 2025-11-18  
**Проблема:** Wildberries Orders API возвращает 302 Found с ошибкой "Some backend error was happening"

## Ошибка

```
⚠️ WB Orders API unavailable: Wildberries Orders API failed with status 302 Found 
for date 2025-10-02: {"errors":"Some backend error was happening"}. 
```

## Возможные причины

### 1. ⚠️ ДАТА В БУДУЩЕМ
**Важно!** Дата 2025-10-02 - это октябрь **2025** года (будущее).

API не может вернуть заказы из будущего. Возможно:
- Опечатка в дате: имелся в виду **2024-10-02**?
- Или **2023-10-02**?

### 2. Неправильный API endpoint или параметры

302 Found - это HTTP редирект, который обычно означает:
- API endpoint изменился или переехал на другой URL
- Неправильный формат запроса
- Проблема с аутентификацией

### 3. Недействительный или неправильный API ключ

Ответ `{"errors":"Some backend error was happening"}` может указывать на:
- API ключ не имеет прав на Orders API
- API ключ истёк или был отозван
- Неправильный формат API ключа

### 4. Orders API endpoint не существует или недоступен

Как упоминалось в `ISSUE_WB_ORDERS_API.md`, Orders API endpoint может:
- Не существовать в вашем аккаунте WB
- Требовать специальных прав доступа
- Быть временно недоступным

## Что было исправлено

**Файл:** `crates/backend/src/usecases/u504_import_from_wildberries/wildberries_api_client.rs`

### 1. Добавлено детальное логирование

Теперь логируются:
- Final URL после всех редиректов
- Все заголовки HTTP ответа
- Специальное сообщение для 302/301 редиректов

```rust
let status = response.status();
let final_url = response.url().clone();
self.log_to_file(&format!("Response status: {}", status));
self.log_to_file(&format!("Final URL: {}", final_url));

// Логируем все заголовки
self.log_to_file(&format!("Response headers:"));
for (name, value) in response.headers() {
    if let Ok(val_str) = value.to_str() {
        self.log_to_file(&format!("  {}: {}", name, val_str));
    }
}
```

### 2. Специальная обработка редиректов

```rust
if status.as_u16() == 302 || status.as_u16() == 301 {
    anyhow::bail!(
        "Wildberries Orders API returned redirect {} for date {}. \
        This may indicate:\n\
        1. Incorrect API endpoint URL\n\
        2. Missing or invalid authentication\n\
        3. API endpoint has moved\n\
        Response: {}\n\
        Check Wildberries API documentation for the correct endpoint.",
        status,
        current_date,
        body
    );
}
```

### 3. Настройка HTTP клиента для редиректов

```rust
client: reqwest::Client::builder()
    .timeout(std::time::Duration::from_secs(60))
    .danger_accept_invalid_certs(true)
    .redirect(reqwest::redirect::Policy::limited(10)) // Следовать редиректам
    .build()
```

## Как диагностировать проблему

### Шаг 1: Проверьте логи

После следующей попытки импорта проверьте файл `wildberries_api_requests.log`:

Ищите секцию:
```
=== REQUEST ===
GET https://statistics-api.wildberries.ru/api/v1/supplier/orders?dateFrom=...
```

И смотрите на:
```
Response status: 302 Found
Final URL: <куда перенаправил>
Response headers:
  location: <новый URL>
  ...
```

**Важные вопросы:**
1. Куда перенаправляет `Final URL`?
2. Есть ли заголовок `Location` в ответе?
3. Какие еще заголовки присутствуют?

### Шаг 2: Проверьте даты

**Убедитесь что даты корректны:**
- Даты должны быть в прошлом или настоящем
- Формат даты: YYYY-MM-DD (например, 2024-10-02)
- Не используйте даты из будущего

### Шаг 3: Проверьте API ключ

1. Зайдите в личный кабинет Wildberries
2. Перейдите в раздел API ключей
3. Проверьте:
   - ✅ Ключ активен
   - ✅ Ключ имеет права на "Statistics API"
   - ✅ Ключ имеет права на "Orders" (если такая опция есть)
   - ✅ Ключ не истёк

### Шаг 4: Проверьте документацию WB

Ознакомьтесь с актуальной документацией:
- https://openapi.wb.ru/statistics/api/ru/
- Проверьте существует ли endpoint `/api/v1/supplier/orders`
- Проверьте требуемые параметры и формат запроса

## Рекомендации

### Вариант 1: Исправить дату

Если дата 2025-10-02 была ошибкой:
1. Используйте дату из прошлого: **2024-10-02** или **2023-10-02**
2. Или используйте текущий месяц

### Вариант 2: Использовать Sales API вместо Orders

Как указано в `ISSUE_WB_ORDERS_API.md`:

**Orders API может быть недоступен.**  
**Используйте Sales API (a012_wb_sales) как альтернативу.**

Преимущества Sales API:
- ✅ Гарантированно работает
- ✅ Содержит информацию о заказах
- ✅ Включает данные о продажах и возвратах

### Вариант 3: Временно отключить импорт заказов

В u504 UI:
1. Снимите галочку с "a015_wb_orders - Заказы Wildberries"
2. Оставьте только:
   - ✅ a007_marketplace_product (Товары)
   - ✅ a012_wb_sales (Продажи)
   - ✅ p903_wb_finance_report (Финансы)

## Следующие шаги

### 1. Запустите импорт с исправленной датой

```
Дата от: 2024-10-01
Дата до: 2024-10-31
```

### 2. Проверьте логи

После импорта откройте `wildberries_api_requests.log` и найдите:
- Final URL
- Response headers
- Location header (если есть редирект)

### 3. Отправьте логи для анализа

Если проблема сохраняется, сохраните секцию из логов:
```
=== REQUEST ===
...
Response status: ...
Final URL: ...
Response headers:
...
```

## Проверка компиляции

✅ Код компилируется успешно:
```bash
cd crates/backend && cargo check
```

## Связанные файлы

- `crates/backend/src/usecases/u504_import_from_wildberries/wildberries_api_client.rs` - исправления
- `ISSUE_WB_ORDERS_API.md` - документация по Orders API проблемам
- `FIX_A015_EMPTY_RESPONSE.md` - исправление пустых ответов
- `wildberries_api_requests.log` - файл с логами запросов

## Итог

Добавлено детальное логирование для диагностики проблемы с 302 редиректом. 

**Главная рекомендация:** Проверьте дату импорта - 2025-10-02 это будущее!

