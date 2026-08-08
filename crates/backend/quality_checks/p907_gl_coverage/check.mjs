export async function run(_input, host) {
  const counts = await host.db.query(`
    SELECT CAST(COUNT(*) AS INTEGER) population,
           CAST(COALESCE(SUM(CASE WHEN NOT EXISTS (
             SELECT 1 FROM sys_general_ledger g
             WHERE g.registrator_type = 'p907_ym_payment_report'
               AND g.registrator_ref = p.id
           ) THEN 1 ELSE 0 END), 0) AS INTEGER) violations
    FROM p907_ym_payment_report p
    WHERE p.transaction_sum IS NOT NULL AND p.transaction_sum <> 0`);
  const population = Number(counts[0]?.population || 0);
  const missing = Number(counts[0]?.violations || 0);
  const violations = [];
  if (missing > 0) {
    const rows = await host.db.query(`
      SELECT p.id projection_id,
             'transaction_date=' || COALESCE(p.transaction_date, '')
             || ', source=' || COALESCE(p.transaction_source, '')
             || ', sum=' || COALESCE(CAST(p.transaction_sum AS TEXT), '')
             || ', order_id=' || COALESCE(CAST(p.order_id AS TEXT), '') detail
      FROM p907_ym_payment_report p
      WHERE p.transaction_sum IS NOT NULL AND p.transaction_sum <> 0
        AND NOT EXISTS (
          SELECT 1 FROM sys_general_ledger g
          WHERE g.registrator_type = 'p907_ym_payment_report'
            AND g.registrator_ref = p.id
        )
      ORDER BY p.transaction_date DESC, p.id
      LIMIT 20`);
    for (const row of rows) {
      violations.push({
        violation_type: "missing_gl",
        projection_id: String(row.projection_id || ""),
        projection_table: "p907_ym_payment_report",
        detail: String(row.detail || "")
      });
    }
  }
  return {
    metrics: [{
      label: "p907 — ненулевые строки без GL-проводки (missing_gl)",
      population,
      violations: missing,
      unit: "строк"
    }],
    violations,
    breakdowns: [],
    sources: []
  };
}
