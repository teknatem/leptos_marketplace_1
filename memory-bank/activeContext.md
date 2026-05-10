# Active Context

_Последнее обновление: 2026-05-05_

## 🎯 Текущий фокус

Система стабильна. Все 26 агрегатов, 6 юзкейсов, 12 проекций реализованы. Актуальные следующие шаги — в разделе ниже.

## 📝 Ключевые реализованные системы

- **YM Orders scheduled polling** (2026-05-05): добавлен тип регламентного задания `task013_ym_orders_polling` — адаптивный поллер заказов Яндекс.Маркета по аналогии с WB `task001`; использует `u503_import_from_yandex` только для `a013_ym_order`, отдельный progress tracker и две seed-задачи для двух YM-кабинетов.
- **Yandex Market auth/info** (2026-05-05): `a006_connection_mp` для Яндекс.Маркета использует выбранный `ТипАвторизации` (`API Key` → `Api-Key`, `OAuth 2.0` → `Authorization: Bearer`); кнопка "Информация" показывает сводку по кабинетам/магазинам через Partner API и данные API-Key токена, если доступно.
- **WB Orders → WB Supply link** (2026-05-04): в карточке заказа WB (`a015_wb_orders`) блок связанных объектов показывает ссылку на поставку WB (`a029_wb_supply`), найденную по списку заказов поставки; при отсутствии связи выводится "Поставка не найдена".
- **WB Advert Campaigns a030** (2026-04-26): отдельный справочник рекламных кампаний WB (`a030_wb_advert_campaign`) с сырым `info_json` из Advert API; `task012_wb_advert_campaigns` обновляет его ежедневно, `task011_wb_advert` берёт `advert_id` из a030 для `fullstats`. Внутри задач WB Advert используются локальные паузы между запросами (`task012`: 250 мс после списка и 3 сек между info-чанками; `task011`: 21 сек между fullstats-чанками), при `429` пишется диагностика с X-Ratelimit и дальнейшие info-чанки в запуске не отправляются. Добавлен UI: список и детали в сайдбаре `Справочники`.
- **DataView** (2026-03-12): `dv001_revenue`, самодостаточный, LLM tool `list_data_views`
- **Формальные миграции БД** (2026-02-18): `migrations/`, `sqlx::migrate::Migrator`, `_sqlx_migrations`
- **Field Metadata System** (POC): `a001_connection_1c`, JSON → `build.rs` → `metadata_gen.rs`
- **General Ledger** (2026-03): независимая система в `crates/*/src/general_ledger/`
- **Scheduled Tasks**: tokio background worker, file-based logging
- **LLM Chat**: a017-a019, tool_executor, knowledge base, default_agent.md

## 🔄 Следующие шаги

- Расширение Field Metadata на a002-a016
- Новые DataView (dv002+)
- Экспорт данных (Excel, CSV)
- Инструмент `create_bi_indicator` для LLM

## ⚠️ Критические правила (не нарушать)

- **General Ledger** — отдельная система: `crates/*/src/general_ledger/`, НЕ в `domain/`
- **Shell**: PowerShell, никогда не использовать `&&`, только `;`

## 📚 Полезные документы

- `architecture/data-view-system.md` — DataView архитектура
- `architecture/metadata-system.md` — Field Metadata система
- `architecture/list-standard.md` — стандарт списков
- `runbooks/` — пошаговые инструкции
- `known-issues/` — известные ограничения Thaw
