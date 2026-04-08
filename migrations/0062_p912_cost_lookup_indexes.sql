CREATE INDEX IF NOT EXISTS idx_p912_nomenclature_period_updated
    ON p912_nomenclature_costs(nomenclature_ref, period DESC, updated_at DESC, line_no DESC);
