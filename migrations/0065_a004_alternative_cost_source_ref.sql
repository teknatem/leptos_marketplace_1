ALTER TABLE a004_nomenclature
    ADD COLUMN alternative_cost_source_ref TEXT;

CREATE INDEX IF NOT EXISTS idx_a004_alternative_cost_source_ref
    ON a004_nomenclature(alternative_cost_source_ref);
