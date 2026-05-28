-- Добавляем GL-итоги по трём оборотам рекламы в a033_wb_day_close.
-- Заполняются при recalculate, используются для сверки GL vs снапшот.
ALTER TABLE a033_wb_day_close ADD COLUMN gl_advert_no_order REAL NOT NULL DEFAULT 0.0;
ALTER TABLE a033_wb_day_close ADD COLUMN gl_advert_order_accrual REAL NOT NULL DEFAULT 0.0;
ALTER TABLE a033_wb_day_close ADD COLUMN gl_advert_order_expense REAL NOT NULL DEFAULT 0.0;
