const TABLES = [
  ["a012_wb_sales", "a012 - WB sales", "id", "'sale_date=' || COALESCE(sale_date, '') || ', document_no=' || COALESCE(document_no, '') || ', sale_id=' || COALESCE(sale_id, '')", "sale_date DESC, id"],
  ["a013_ym_order_items", "a013 - Yandex Market order items", "id", "'order_id=' || COALESCE(order_id, '') || ', line_id=' || COALESCE(line_id, '') || ', offer_id=' || COALESCE(offer_id, '')", "order_id DESC, id"],
  ["a015_wb_orders", "a015 - WB orders", "id", "'document_date=' || COALESCE(document_date, '') || ', document_no=' || COALESCE(document_no, '') || ', g_number=' || COALESCE(g_number, '')", "document_date DESC, id"],
  ["p900_sales_register", "p900 - marketplace sales register", "NULL", "'sale_date=' || COALESCE(sale_date, '') || ', registrator_ref=' || COALESCE(registrator_ref, '') || ', line_id=' || COALESCE(line_id, '')", "sale_date DESC, registrator_ref"],
  ["p904_sales_data", "p904 - sales data", "id", "'date=' || COALESCE(date, '') || ', registrator_ref=' || COALESCE(registrator_ref, '') || ', article=' || COALESCE(article, '')", "date DESC, id"],
  ["p909_mp_order_line_turnovers", "p909 - order line turnovers", "id", "'entry_date=' || COALESCE(entry_date, '') || ', registrator_ref=' || COALESCE(registrator_ref, '') || ', order_key=' || COALESCE(order_key, '')", "entry_date DESC, id"],
  ["p911_wb_advert_by_items", "p911 - WB advert by items", "id", "'entry_date=' || COALESCE(entry_date, '') || ', registrator_ref=' || COALESCE(registrator_ref, '') || ', campaign=' || COALESCE(wb_advert_campaign_code, '')", "entry_date DESC, id"]
];

export async function run(_input, host) {
  const metrics = [];
  const violations = [];
  for (const [table, label, idExpr, detailExpr, orderExpr] of TABLES) {
    const counts = await host.db.query(`
      SELECT CAST(COUNT(*) AS INTEGER) population,
             CAST(COALESCE(SUM(CASE WHEN marketplace_product_ref IS NULL OR TRIM(marketplace_product_ref) = '' THEN 1 ELSE 0 END), 0) AS INTEGER) violations
      FROM ${table}`);
    const population = Number(counts[0]?.population || 0);
    const missing = Number(counts[0]?.violations || 0);
    metrics.push({ label, population, violations: missing, unit: "строк" });

    const remaining = 20 - violations.length;
    if (missing > 0 && remaining > 0) {
      const rows = await host.db.query(`
        SELECT ${idExpr} projection_id, ${detailExpr} detail
        FROM ${table}
        WHERE marketplace_product_ref IS NULL OR TRIM(marketplace_product_ref) = ''
        ORDER BY ${orderExpr}
        LIMIT ${remaining}`);
      for (const row of rows) {
        violations.push({
          violation_type: "missing_marketplace_product_ref",
          projection_id: row.projection_id == null ? null : String(row.projection_id),
          projection_table: table,
          detail: row.detail == null ? null : String(row.detail)
        });
      }
    }
  }
  return { metrics, violations, breakdowns: [], sources: [] };
}
