ALTER TABLE p911_wb_advert_by_items
    ADD COLUMN is_problem INTEGER NOT NULL DEFAULT 0;

UPDATE p911_wb_advert_by_items
SET is_problem = CASE
    WHEN nomenclature_ref IS NULL OR TRIM(nomenclature_ref) = '' THEN 1
    ELSE 0
END;

CREATE INDEX IF NOT EXISTS idx_p911_is_problem
    ON p911_wb_advert_by_items (is_problem);
