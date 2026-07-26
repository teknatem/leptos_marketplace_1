-- a034: снять уникальность «один документ на кабинет×день» — теперь документ
-- реализации YM = кабинет × КАМПАНИЯ × день (отчёт о реализации помесячный на
-- кампанию; на один день приходится по документу на FBS, FBY и т.д.).
--
-- Прежний уникальный индекс (connection_id, document_date) из 0132 запрещал
-- второй документ того же дня → при импорте FBY-документ за день, где уже есть
-- FBS, падал с нарушением уникальности, и данные FBY не сохранялись
-- (импорт завершался с ошибками). Заменяем на уникальность по document_no,
-- который уже несёт кампанию: 'YMREAL-{connection}-{campaign}-{YYYY-MM-DD}'
-- (для legacy без кампании — 'YMREAL-{connection}-{YYYY-MM-DD}'). Идемпотентность
-- по-прежнему держит детерминированный первичный ключ (id).

DROP INDEX IF EXISTS idx_a034_ym_realization_connection_date;

CREATE UNIQUE INDEX IF NOT EXISTS idx_a034_ym_realization_connection_document_no
    ON a034_ym_realization(connection_id, document_no)
    WHERE is_deleted = 0;
