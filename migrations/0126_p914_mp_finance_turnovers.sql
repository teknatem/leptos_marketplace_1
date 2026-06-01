-- p914_mp_finance_turnovers: проекция финансовых оборотов слоя `fina`.
-- Формируется 1:1 вместе с GL-проводками слоя fina из p903_wb_finance_report
-- и p907_ym_payment_report. Каждая строка зеркалит одну GL-запись fina
-- (general_ledger_ref) и совпадает с ней по сумме, дате транзакции,
-- turnover_code и основным измерениям, добавляя разрезы для отчётности.

CREATE TABLE p914_mp_finance_turnovers (
    id TEXT NOT NULL PRIMARY KEY,
    transaction_date TEXT NOT NULL,
    general_ledger_ref TEXT,
    registrator_type TEXT NOT NULL,
    registrator_ref TEXT NOT NULL,
    connection_mp_ref TEXT NOT NULL,
    nomenclature_ref TEXT,
    marketplace_product_ref TEXT,
    turnover_code TEXT NOT NULL,
    order_key TEXT NOT NULL,
    event_kind TEXT NOT NULL,
    -- UL — юрлицо, FL — физлицо. Источник пока не определён в финотчётах,
    -- заполняется значением-заглушкой до подключения реального источника.
    customer_kind TEXT,
    -- FBO, FBS, FBW.
    fulfillment_type TEXT,
    layer TEXT NOT NULL DEFAULT 'fina',
    amount REAL NOT NULL,
    quantity REAL,
    created_at_msk TEXT NOT NULL,
    updated_at_msk TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_p914_general_ledger_ref ON p914_mp_finance_turnovers(general_ledger_ref);
CREATE INDEX idx_p914_registrator ON p914_mp_finance_turnovers(registrator_ref);
CREATE INDEX idx_p914_transaction_date ON p914_mp_finance_turnovers(transaction_date);
CREATE INDEX idx_p914_connection ON p914_mp_finance_turnovers(connection_mp_ref);
CREATE INDEX idx_p914_turnover_code ON p914_mp_finance_turnovers(turnover_code);
CREATE INDEX idx_p914_order_key ON p914_mp_finance_turnovers(order_key);
