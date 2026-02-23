-- Миграция для добавления полей в a006_connection_mp
-- Запустить: sqlite3 "E:/dev/rust/leptos_marketplace_1/data/app.db" < migrate_a006_connection_mp.sql

-- Поле плановой комиссии
ALTER TABLE a006_connection_mp ADD COLUMN planned_commission_percent REAL;

-- UUID-ссылка на организацию.
-- Старое поле organization оставляем временно как deprecated для ручного переноса.
ALTER TABLE a006_connection_mp ADD COLUMN organization_ref TEXT NOT NULL DEFAULT '';

-- Проверка
SELECT COUNT(*) as total_records FROM a006_connection_mp;
