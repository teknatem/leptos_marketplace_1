-- Физическое разделение строк a034 на продажи и возвраты: вместо общего
-- lines_json — две отдельные коллекции sales_lines_json / return_lines_json.
-- Бэкфилл из существующего lines_json по флагу is_return, затем удаление lines_json.

ALTER TABLE a034_ym_realization ADD COLUMN sales_lines_json TEXT NOT NULL DEFAULT '[]';
ALTER TABLE a034_ym_realization ADD COLUMN return_lines_json TEXT NOT NULL DEFAULT '[]';

UPDATE a034_ym_realization
SET sales_lines_json = COALESCE((
    SELECT json_group_array(json(li.value))
    FROM json_each(lines_json) li
    WHERE COALESCE(json_extract(li.value, '$.is_return'), 0) = 0
), '[]');

UPDATE a034_ym_realization
SET return_lines_json = COALESCE((
    SELECT json_group_array(json(li.value))
    FROM json_each(lines_json) li
    WHERE json_extract(li.value, '$.is_return') = 1
), '[]');

ALTER TABLE a034_ym_realization DROP COLUMN lines_json;
