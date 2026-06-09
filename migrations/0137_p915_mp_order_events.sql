-- p915_mp_order_events: таймлайн событий заказа МП (YM). Каждая строка —
-- одно событие (order_id + event_type + registrator). marketplace_product
-- заполняется только для построчных событий реализации/возврата (a034);
-- для событий уровня заказа (заказ/доставка/оплата) — NULL.
-- Источник push: каждый регистратор (a013/a034/p907) пишет свои события при
-- проведении (delete-by-registrator + insert), как GL/p914.

CREATE TABLE p915_mp_order_events (
    id TEXT NOT NULL PRIMARY KEY,
    order_id TEXT NOT NULL,
    marketplace_product TEXT,            -- a007 uuid; NULL для order-level событий
    event_date TEXT NOT NULL,            -- MSK дата YYYY-MM-DD
    event_type TEXT NOT NULL,            -- член OrderEventType
    layer TEXT NOT NULL,                 -- oper / ybuh / fina
    amount REAL,                         -- сумма операции (инфо/контроль), nullable
    registrator_type TEXT NOT NULL,
    registrator_ref TEXT NOT NULL,
    connection_mp_ref TEXT NOT NULL,
    created_at_msk TEXT NOT NULL,
    updated_at_msk TEXT NOT NULL
);

CREATE INDEX idx_p915_order_id ON p915_mp_order_events(order_id);
CREATE INDEX idx_p915_registrator ON p915_mp_order_events(registrator_ref);
CREATE INDEX idx_p915_event_date ON p915_mp_order_events(event_date);
CREATE INDEX idx_p915_event_type ON p915_mp_order_events(event_type);
CREATE INDEX idx_p915_connection ON p915_mp_order_events(connection_mp_ref);
CREATE INDEX idx_p915_mp_product ON p915_mp_order_events(marketplace_product);
