# Фильтр по организациям в списке Wildberries Sales (A012)

## Описание изменений

Добавлен фильтр по организациям в список Wildberries Sales (A012). Теперь пользователь может выбрать конкретную организацию из выпадающего списка и обновить данные только для этой организации.

## Файлы изменений

### Backend

#### `crates/backend/src/handlers/a012_wb_sales.rs`

1. **Обновлена структура `ListSalesQuery`:**
   ```rust
   #[derive(Debug, Deserialize)]
   pub struct ListSalesQuery {
       pub date_from: Option<String>,
       pub date_to: Option<String>,
       pub limit: Option<usize>,
       pub offset: Option<usize>,
       pub organization_id: Option<String>,  // НОВОЕ ПОЛЕ
   }
   ```

2. **Добавлена фильтрация в handler `list_sales`:**
   ```rust
   // Фильтруем по organization_id, если указан
   if let Some(org_id) = query.organization_id {
       items.retain(|sale| sale.header.organization_id == org_id);
   }
   ```

### Frontend

#### `crates/frontend/src/domain/a012_wb_sales/ui/list/mod.rs`

1. **Добавлена структура Organization:**
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   pub struct Organization {
       pub id: String,
       pub code: String,
       pub description: String,
   }
   ```

2. **Добавлены состояния для организаций:**
   ```rust
   let (organizations, set_organizations) = signal::<Vec<Organization>>(Vec::new());
   let (selected_organization_id, set_selected_organization_id) = signal::<Option<String>>(None);
   ```

3. **Обновлена функция `load_sales`:**
   - Теперь учитывает выбранную организацию при формировании URL запроса
   - Добавляет параметр `organization_id` в query string, если организация выбрана

4. **Добавлена функция `fetch_organizations`:**
   - Загружает список всех организаций из API
   - Вызывается при монтировании компонента

5. **Добавлен UI элемент для выбора организации:**
   - Выпадающий список (select) с опцией "Все организации"
   - Отображает все доступные организации
   - Расположен рядом с фильтрами по дате

## Использование

1. При открытии списка Wildberries Sales загружаются данные за текущий месяц для всех организаций
2. Пользователь может выбрать конкретную организацию из выпадающего списка
3. При нажатии кнопки "Обновить" данные фильтруются по выбранной организации
4. Чтобы снова показать все организации, нужно выбрать "Все организации" и нажать "Обновить"

## API

**Endpoint:** `GET /api/a012/wb-sales`

**Query параметры:**
- `date_from` (optional): начальная дата периода (формат: YYYY-MM-DD)
- `date_to` (optional): конечная дата периода (формат: YYYY-MM-DD)
- `limit` (optional): максимальное количество записей (по умолчанию: 20000)
- `offset` (optional): смещение для пагинации (по умолчанию: 0)
- `organization_id` (optional): ID организации для фильтрации **[НОВОЕ]**

**Пример запроса:**
```
GET /api/a012/wb-sales?date_from=2025-11-01&date_to=2025-11-30&organization_id=123e4567-e89b-12d3-a456-426614174000
```

## Тестирование

1. Запустите backend: `cargo run` из корня проекта
2. Запустите frontend: `trunk serve` из директории `crates/frontend`
3. Откройте браузер и перейдите к списку Wildberries Sales (A012)
4. Проверьте, что:
   - Выпадающий список организаций заполнен
   - При выборе организации и нажатии "Обновить" отображаются только записи этой организации
   - При выборе "Все организации" отображаются записи всех организаций

## Статус

✅ Backend изменения: реализованы и протестированы
✅ Frontend изменения: реализованы и протестированы
✅ Компиляция: успешна (backend и frontend)

