-- Отмены по счётчику воронки маркетплейса.
--
-- WB отдаёт cancelCount/cancelSumRub в отчёте воронки (DETAIL_HISTORY_REPORT), но импорт
-- их выбрасывал: в p916 отмены были только order-level (движения из a015 по конкретным
-- заказам), которые страдают от неполноты FBS-пути. Дневной счётчик маркетплейса —
-- независимый источник и основной для «сколько отказов за период».
--
-- Обе метрики nullable: NULL = источник счётчик не отдал (документы, импортированные до
-- этой правки, и пути API без такого поля). N/A ≠ 0 — на чтении это различают флаги
-- *_available, поэтому нельзя ставить DEFAULT 0.
--
-- Именование в p916 — funnel_cancel_*, по образцу funnel_order_*: префикс funnel_ метит
-- маркетинговый счётчик и отделяет его от cancel_count/cancel_sum стадии fulfillment.

ALTER TABLE a036_wb_sales_funnel_daily ADD COLUMN total_cancel_count INTEGER;
ALTER TABLE a036_wb_sales_funnel_daily ADD COLUMN total_cancel_sum REAL;

ALTER TABLE p916_mp_sales_funnel_turnovers ADD COLUMN funnel_cancel_count INTEGER;
ALTER TABLE p916_mp_sales_funnel_turnovers ADD COLUMN funnel_cancel_sum REAL;
