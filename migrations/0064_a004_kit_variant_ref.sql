ALTER TABLE a004_nomenclature
    ADD COLUMN kit_variant_ref TEXT;

CREATE INDEX IF NOT EXISTS idx_a004_kit_variant_ref
    ON a004_nomenclature(kit_variant_ref);

CREATE INDEX IF NOT EXISTS idx_a022_owner_ref
    ON a022_kit_variant(owner_ref);
