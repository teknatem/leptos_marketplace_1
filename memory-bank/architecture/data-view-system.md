# DataView — Архитектура семантического слоя

_Создан: 2026-03-12_

## Обзор

DataView — именованное бизнес-вычисление, которое инкапсулирует:
- источник данных (одна или несколько SQL-таблиц)
- логику агрегации метрик (формулы, группировки)
- доступные измерения для drill-down
- двухпериодное сравнение (P1 vs P2)
- фильтры (ссылки на глобальный FilterRegistry)

DataView является **самодостаточным**: каждый `dvNNN/metadata.json` полностью описывает
все SQL-специфичные данные (db_column, ref_table, join_on_column) и не зависит
от внешних реестров.

## Структура файлов

```
crates/backend/src/data_view/
├── mod.rs                    # DataViewRegistry — реестр + кеш scalar-результатов
├── filters.rs                # GlobalFilterRegistry (date_range_1/2, connection_mp_refs)
└── dv001/
    ├── mod.rs                # Реализация: compute_scalar, compute_drilldown, compute_drilldown_multi
    └── metadata.json         # Метаданные: метрики, измерения (с SQL-полями), фильтры

crates/contracts/src/shared/data_view/mod.rs
    DataViewMeta              # Десериализуется из metadata.json
    DimensionMeta             # id + label + SQL execution fields
    ResourceMeta              # metric_id + label + unit
    ViewContext               # Универсальный входной контекст
    FilterDef / FilterRef     # Типы фильтров
```

## Принцип самодостаточности DimensionMeta

Каждое измерение в `metadata.json` содержит не только UI-данные (id, label),
но и SQL execution поля:

```json
{
  "id": "connection_mp_ref",
  "label": "По кабинету МП",
  "db_column": "connection_mp_ref",
  "ref_table": "a006_connection_mp",
  "ref_display_column": "description"
}
```

```json
{
  "id": "dim1",
  "label": "Измерение 1 (категория)",
  "db_column": "dim1_category",
  "source_table": "a004_nomenclature",
  "join_on_column": "nomenclature_ref"
}
```

Поля `DimensionMeta`:
- `db_column` — реальная колонка в основной таблице (если `None` — совпадает с `id`)
- `ref_table` — JOIN со справочником для отображения метки
- `ref_display_column` — колонка из ref_table для отображения
- `source_table` — для косвенных JOIN (dim1-dim6 через номенклатуру)
- `join_on_column` — колонка связи в основной таблице

Все поля `#[serde(default)]` → обратная совместимость: старые metadata.json без SQL-полей
парсятся без ошибок.

## Шаблон metadata.json для нового DataView (dvNNN)

```json
{
  "id": "dvNNN_name",
  "name": "Отображаемое название",
  "category": "revenue | orders | costs | ...",
  "version": 1,
  "description": "Краткое описание для UI",
  "ai_description": "Развёрнутое описание для LLM: источник данных, формулы, поведение",
  "data_sources": ["table_name"],
  "available_resources": [
    { "id": "metric_key", "label": "Название метрики", "description": "Формула/семантика", "unit": "currency | count | percent" }
  ],
  "available_dimensions": [
    { "id": "field_id", "label": "Название", "db_column": "col_name" },
    { "id": "ref_field", "label": "По справочнику", "db_column": "ref_id",
      "ref_table": "ref_table_name", "ref_display_column": "description" },
    { "id": "dim_field", "label": "Измерение из JOIN", "db_column": "dim_col",
      "source_table": "join_table", "join_on_column": "fk_col" }
  ],
  "filters": [
    { "filter_id": "date_range_1",       "required": true,  "order": 1 },
    { "filter_id": "date_range_2",       "required": false, "order": 2 },
    { "filter_id": "connection_mp_refs", "required": false, "order": 3 }
  ]
}
```

## Регистрация нового DataView

В `crates/backend/src/data_view/mod.rs`:

```rust
pub mod dvNNN;

// В DataViewRegistry::new():
registry.register(
    dvNNN::meta(),
    |ctx| Box::pin(dvNNN::compute_scalar(ctx)),
    |ctx, g, ids| Box::pin(dvNNN::compute_drilldown_multi(ctx, g, ids)),
);
```

## API эндпоинты

```
GET  /api/data-view                    → список всех DataView (метаданные)
GET  /api/data-view/:id                → метаданные конкретного DataView
GET  /api/data-view/:id/filters        → резолвированные фильтры
POST /api/data-view/:id/compute        → вычислить скаляр (ViewContext в теле)
POST /api/data-view/:id/drilldown      → детализация (ViewContext + group_by + metric_ids)
```

## Связь с BI Индикаторами (a024)

`BiIndicator` ссылается на DataView через `DataSpec`:

```rust
pub struct DataSpec {
    pub view_id:   Option<String>,   // "dv001_revenue" — главный путь
    pub metric_id: Option<String>,   // "revenue" | "cost" | ...
    // fallback поля (legacy):
    pub data_source_config: Option<...>,
    pub schema_query: Option<...>,
    pub schema_id: String,
}
```

Приоритет вычисления в `a024_bi_indicator/service.rs`:
1. `view_id` → `DataViewRegistry::compute_scalar()` (основной путь)
2. `data_source_config` → `schema_executor::compute_from_data_source()` (универсальный fallback)
3. `schema_query` → `schema_executor::compute_p904()` (legacy p904)
4. `schema_id` → `IndicatorRegistry` (deprecated, 4 хардкоженных функции)

## Что сохранено из старой схемы (не удалять)

| Компонент | Файл | Зачем нужен |
|---|---|---|
| `DataSourceSchema` + `FieldDef` | `contracts/shared/universal_dashboard/schema.rs` | Используется в `universal_dashboard` (pivot UI) |
| `ds03_p904_sales` | `backend/data_schemes/ds03_p904_sales/schema.rs` | Используется `universal_dashboard` (OLAP-пивот, SQL preview) |
| `SchemaRegistry` | `backend/shared/universal_dashboard/entity_registry.rs` | Используется `universal_dashboard` и `schema_executor` fallback |
| `schema_executor.rs` | `backend/shared/indicators/schema_executor.rs` | Fallback compute (приоритет 2 и 3) |
| `IndicatorRegistry` | `backend/shared/indicators/registry.rs` | Финальный fallback (приоритет 4, deprecated) |

**Важно:** `dv001` больше не зависит от `SchemaRegistry` и `ds03_p904_sales`.
Все новые DataView также не должны зависеть от них.

## Кеш scalar-результатов

`DataViewRegistry::compute_scalar()` кеширует результаты на 30 секунд
по ключу `(view_id, date_from, date_to, period2, connection_mp_refs, metric)`.

Несколько индикаторов дашборда с одинаковым DataView + контекстом
не делают повторных запросов к БД.
