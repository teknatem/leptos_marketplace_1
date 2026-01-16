---
type: lesson
date: 2026-01-16
tags: [architecture, projection, builder-pattern]
---

# Lesson: Projection Builder vs Service

## projection_builder.rs — Маппинг/Трансформация

**Роль**: Преобразование данных из формата агрегатов в формат проекции

```
OzonFbsPosting  ─┐
OzonFboPosting  ─┤
WbSales         ─┼──▶ SalesRegisterEntry
YmOrder         ─┤
OzonReturns     ─┘
```

**Знает**:

- Структуру входных агрегатов (a009-a013)
- Структуру выходной проекции
- Логику маппинга полей

**НЕ знает**: как сохранять, как запрашивать

## service.rs — Оркестрация

**Роль**: Координация всех операций над проекцией

```
service.rs
├── project_*()           ─▶ builder + repository
├── list_with_filters()   ─▶ repository
├── calculate_*_stats()   ─▶ repository + бизнес-логика
└── delete_by_registrator()─▶ repository
```

**Знает**:

- Какой builder вызвать
- Какой repository метод использовать
- Бизнес-логику агрегации

## Поток данных

```
domain/a012_wb_sales/posting.rs
    │
    ▼
service::project_wb_sales(document, id)
    │
    ├──▶ projection_builder::from_wb_sales(document)
    │         └── SalesRegisterEntry
    │
    └──▶ repository::upsert_entry(entry)
              └── БД
```

## Преимущества разделения

1. Добавление маркетплейса = изменение только builder
2. Изменение логики сохранения = изменение только service/repository
3. Тестирование маппинга отдельно от IO
