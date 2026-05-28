-- Итог p913 (advert_clicks_order_expense) за дату документа — снапшот проекции.
-- Используется для колонки «Документ» в блоке сверки рекламы.
ALTER TABLE a033_wb_day_close ADD COLUMN snap_advert_order_expense REAL NOT NULL DEFAULT 0.0;
