-- a026 валидация: при проведении документа ищем заказы a015 за тот же день
-- по nm_id и connection_id. Найденные заказы и сводный флаг сохраняем в самом
-- документе, чтобы оценить целесообразность механизма привязки расходов к
-- заказам перед внедрением полной проекции p913.

ALTER TABLE a026_wb_advert_daily
    ADD COLUMN has_linked_orders INTEGER NOT NULL DEFAULT 0;

ALTER TABLE a026_wb_advert_daily
    ADD COLUMN linked_orders_count INTEGER NOT NULL DEFAULT 0;

ALTER TABLE a026_wb_advert_daily
    ADD COLUMN linked_orders_json TEXT NOT NULL DEFAULT '[]';
