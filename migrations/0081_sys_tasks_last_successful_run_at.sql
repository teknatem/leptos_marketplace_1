-- Добавляет отметку времени последнего *успешного* завершения задачи.
-- Используется как watermark для инкрементальной загрузки: задача читает данные
-- начиная с (last_successful_run_at - overlap_days), а не с last_run_at,
-- чтобы повторные неудачные запуски не расширяли окно до полного lookback_days.
ALTER TABLE sys_tasks ADD COLUMN last_successful_run_at DATETIME;
