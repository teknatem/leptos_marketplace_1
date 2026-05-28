-- Migration: 0121 — a033_wb_day_close advert snapshot columns
-- Adds two JSON snapshot columns for advertising data captured at recalculate time.
-- advert_no_order_json: rows from p911 (advert_clicks_no_order)
-- advert_order_accrual_json: rows from p913 (advert_clicks_order_accrual)

ALTER TABLE a033_wb_day_close
    ADD COLUMN advert_no_order_json TEXT NOT NULL DEFAULT '[]';

ALTER TABLE a033_wb_day_close
    ADD COLUMN advert_order_accrual_json TEXT NOT NULL DEFAULT '[]';
