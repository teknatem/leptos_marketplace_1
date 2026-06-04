-- Новое измерение GL «субъект учёта» (entity): контур, к которому относится
-- проводка — маркетплейс (ym/wb/ozon) или собственная организация (san/sts/upr).
-- Фаза 1: вводим колонку и помечаем весь платёжный отчёт YM (p907) как entity='ym'
-- («весь отчёт = операции маркетплейса», сальдо 7609 при entity='ym' = деньги у Yandex).
-- Прочие источники получат субъект при их переработке позже (пока NULL = «не задан»).

-- 1. Колонка субъекта в журнале проводок + индекс.
ALTER TABLE sys_general_ledger ADD COLUMN entity TEXT;
CREATE INDEX idx_sgl_entity ON sys_general_ledger(entity);

-- 2. Зеркало fina (p914) несёт ту же колонку для консистентности drilldown.
ALTER TABLE p914_mp_finance_turnovers ADD COLUMN entity TEXT;
CREATE INDEX idx_p914_entity ON p914_mp_finance_turnovers(entity);

-- 3. Локальное поле выбора субъекта у организации (НЕ перезаписывается импортом из 1С).
ALTER TABLE a002_organization ADD COLUMN entity_ref TEXT;

-- 4. Детерминированный бэкофилл существующих проводок YM-отчёта.
UPDATE sys_general_ledger
   SET entity = 'ym'
 WHERE registrator_type = 'p907_ym_payment_report';

UPDATE p914_mp_finance_turnovers
   SET entity = 'ym'
 WHERE registrator_type = 'p907_ym_payment_report';
