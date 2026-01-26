# D401 WB Finance Dashboard - Implementation Summary

## Overview

Universal dashboard builder (аналог СКД из 1С) with configurable pivot tables, starting with P903 WB Finance Report data source.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│ Frontend (Leptos)                                           │
│ ├── d401_wb_finance (dashboard-specific)                   │
│ │   ├── api.rs - API client                                │
│ │   └── ui/dashboard.rs - Main dashboard UI                │
│ └── shared/dashboards (reusable components)                │
│     ├── config_panel.rs - Field/grouping selection         │
│     ├── pivot_table.rs - Hierarchical table display        │
│     └── saved_configs.rs - Save/load configurations        │
└─────────────────────────────────────────────────────────────┘
                              ↕ HTTP/JSON
┌─────────────────────────────────────────────────────────────┐
│ Backend (Axum)                                              │
│ ├── d401_wb_finance (dashboard-specific)                   │
│ │   ├── schema.rs - P903 field definitions                 │
│ │   └── service.rs - Query execution & config management   │
│ └── shared/dashboards (universal mechanisms)               │
│     ├── query_builder.rs - Dynamic SQL generation          │
│     ├── tree_builder.rs - Pivot transformation             │
│     └── schema_registry.rs - Schema registry               │
└─────────────────────────────────────────────────────────────┘
                              ↕ SQL
┌─────────────────────────────────────────────────────────────┐
│ Database (SQLite)                                           │
│ ├── p903_wb_finance_report (source data)                   │
│ └── sys_dashboard_configs (saved configurations)           │
└─────────────────────────────────────────────────────────────┘
```

## Key Features

### 1. Universal Query Builder
- **Dynamic SQL generation** based on schema and configuration
- **Grouping support** with multiple levels
- **Aggregation functions**: SUM, COUNT, AVG, MIN, MAX
- **Flexible filtering** by date range and dimensions

### 2. Pivot Table Display
- **Hierarchical grouping** with expandable levels
- **Subtotals** at each grouping level
- **Grand totals** for all numeric columns
- **Responsive layout** with indentation for hierarchy

### 3. Configuration Management
- **Save/load configurations** with names and descriptions
- **Persistent storage** in database (sys_dashboard_configs)
- **JSON serialization** for easy sharing/import

### 4. Extensibility
- **Schema-based** approach allows adding new data sources
- **Shared components** reusable across all dashboards
- **Minimal changes** needed for new dashboards (d402, d403, etc.)

## Files Created

### Contracts (shared types)
```
crates/contracts/src/shared/dashboards/
├── mod.rs
├── schema.rs      # DataSourceSchema, FieldDef, AggregateFunction
├── config.rs      # DashboardConfig, filters, requests
└── response.rs    # ExecuteDashboardResponse, PivotRow, column headers
```

### Backend
```
crates/backend/src/
├── shared/dashboards/
│   ├── mod.rs
│   ├── query_builder.rs     # Dynamic SQL builder
│   ├── tree_builder.rs      # Pivot transformation
│   └── schema_registry.rs   # Schema registry
├── dashboards/d401_wb_finance/
│   ├── mod.rs
│   ├── schema.rs            # P903 field definitions
│   └── service.rs           # Service layer
└── api/handlers/
    └── d401_wb_finance.rs   # API endpoints
```

### Frontend
```
crates/frontend/src/
├── shared/dashboards/
│   ├── mod.rs
│   ├── config_panel.rs      # Configuration UI
│   ├── pivot_table.rs       # Table display
│   └── saved_configs.rs     # Save/load UI
└── dashboards/d401_wb_finance/
    ├── mod.rs
    ├── api.rs               # API client
    └── ui/dashboard.rs      # Main dashboard
```

## API Endpoints

```
GET  /api/d401/schemas             - List available data sources
GET  /api/d401/schemas/:id         - Get schema details
POST /api/d401/execute             - Execute dashboard query
GET  /api/d401/configs             - List saved configs
GET  /api/d401/configs/:id         - Get specific config
POST /api/d401/configs             - Save new config
PUT  /api/d401/configs/:id         - Update config
DELETE /api/d401/configs/:id       - Delete config
```

## Database Migration

Run the migration to create the sys_dashboard_configs table:

```bash
# Execute the SQL file
sqlite3 marketplace.db < migrate_d401_wb_finance.sql
```

## P903 Schema Fields

### Grouping Fields (can_group: true)
- `rr_dt` - Дата
- `nm_id` - Артикул WB
- `sa_name` - Артикул продавца
- `subject_name` - Категория товара
- `supplier_oper_name` - Тип операции
- `bonus_type_name` - Тип бонуса
- `connection_mp_ref` - Подключение
- `organization_ref` - Организация

### Aggregated Fields (can_aggregate: true)
- `retail_amount` - Сумма продаж
- `retail_price` - Розничная цена
- `quantity` - Количество
- `ppvz_for_pay` - К перечислению продавцу
- `commission_percent` - Комиссия %
- `delivery_rub` - Стоимость доставки
- `storage_fee` - Стоимость хранения
- `penalty` - Штрафы
- `acquiring_fee` - Эквайринг
- `return_amount` - Сумма возвратов
- `ppvz_sales_commission` - Комиссия продажи
- `additional_payment` - Доплаты
- `cashback_amount` - Кэшбэк
- `rebill_logistic_cost` - Логистика

## Usage Example

### Frontend Access
- Navigate to "Дашборды" → "Финансы WB" in the sidebar
- Route: `d401_wb_finance`

### Typical Workflow
1. **Select groupings** (e.g., date, organization)
2. **Select indicators** (e.g., retail_amount, quantity)
3. **Apply filters** (date range, specific dimensions)
4. **Execute query** to generate pivot table
5. **Save configuration** for future use

## Adding New Data Sources (e.g., d402)

1. **Create schema** in `backend/src/dashboards/d402_xxx/schema.rs`:
   ```rust
   pub static XXX_SCHEMA: DataSourceSchema = DataSourceSchema {
       id: "xxx_table",
       name: "XXX Report",
       fields: &[...],
   };
   ```

2. **Register schema** in handlers (copy d401 pattern)

3. **Create frontend** (copy d401_wb_finance structure)

4. **Add to navigation** in `registry.rs` and `sidebar.rs`

No changes needed to shared mechanisms!

## Technical Notes

- **SQLite limitations**: No ROLLUP support, subtotals computed via multiple queries or in-memory
- **Type safety**: Full type safety across frontend/backend via shared contracts
- **Performance**: Query builder generates optimized SQL with proper indexes
- **Extensibility**: Schema-based approach allows easy addition of new data sources

## Future Enhancements

- Excel export for pivot tables
- Drill-down navigation to source records
- Chart visualizations (bar, line, pie)
- Custom aggregation formulas
- Cross-data-source queries
- Scheduled report generation
