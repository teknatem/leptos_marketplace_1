-- Исправление неверной трактовки a040: раньше считалось, что поисковая аналитика WB отдаёт
-- «показы» (счётчик). На деле WB /table/details отдаёт только visibility (% товара в выдаче),
-- а impressions всегда 0. Правим описания задач task024, засеянных в 0176 с формулировкой
-- «показы/позиции», на «видимость/позиции». Идемпотентно: обновляем только исходный текст.

UPDATE sys_tasks
SET description = REPLACE(description, 'показы/позиции', 'видимость/позиции'),
    updated_at = datetime('now')
WHERE task_type = 'task024_wb_search_analytics_daily'
  AND description LIKE '%показы/позиции%';
