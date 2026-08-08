// Покрытие воронки Яндекс.Маркета в p916.
//
// Матрица источников (та же, что в builder'е p916 — держать синхронной):
//   a013 → заказ / отмена / выкуп;
//   a016 → только возврат и только return_type='RETURN' (UNREDEEMED не проводится:
//          это то же событие, что отмена из a013, и его проведение задвоило бы отказы);
//   a041 → стадия marketing (показы/клики/корзина/заказы/отказы по счётчику YM).
//
// Проверяется наличие движения на документ-источник и отсутствие движений без
// живого источника. Значения метрик не сверяются — это отдельный класс проверок.

function n(value) { return Number(value || 0); }

function selectedConnections(input) {
  return Array.isArray(input?.connection_mp_refs)
    ? input.connection_mp_refs.map(String).filter(Boolean)
    : [];
}

function filterConnections(rows, refs) {
  if (refs.length === 0) return rows;
  const selected = new Set(refs);
  return rows.filter(row => selected.has(String(row.connection_id || "")));
}

async function periodBounds(input, host) {
  if (input?.date_from && input?.date_to) {
    const rows = await host.db.query(`SELECT ? start_date,date(?,'+1 day') end_date`, [String(input.date_from),String(input.date_to)]);
    return {start:String(rows[0].start_date),end:String(rows[0].end_date),exact:true};
  }
  const months = Number.isInteger(input?.months) ? input.months : 12;
  const rows = await host.db.query(`SELECT date('now','start of month',printf('-%d months',?)) start_date,date('now','start of month') end_date,date('now','start of month','+1 month') next_date`,[months]);
  return {start:String(rows[0].start_date),end:String(rows[0].end_date),next:String(rows[0].next_date),exact:false};
}

function mergeRows(target, rows, key) {
  for (const row of rows) {
    target.push({
      key,
      month: String(row.month || ""),
      connection_id: String(row.connection_id || ""),
      cabinet: String(row.cabinet || row.connection_id || "(не указано)"),
      source: n(row.source_count),
      missing: n(row.missing_count),
      extra: n(row.extra_count)
    });
  }
}

/// Покрытие «документ a013 → движение p916» для одного вида движения.
/// `srcFilter` дополнительно сужает документы (например, только отменённые).
async function a013Coverage(host, start, end, key, measure, srcFilter) {
  const filter = srcFilter ? `AND ${srcFilter}` : "";
  return host.db.query(`
    WITH src AS (
      SELECT id, substr(creation_date,1,7) month, connection_id
      FROM a013_ym_order
      WHERE is_deleted=0 AND creation_date >= ? AND creation_date < ? ${filter}
    ), proj AS (
      SELECT registrator_ref, connection_mp_ref
      FROM p916_mp_sales_funnel_turnovers
      WHERE registrator_type='a013_ym_order' AND ${measure} <> 0
      GROUP BY registrator_ref, connection_mp_ref
    ), extra AS (
      SELECT substr(p.cohort_date,1,7) month, p.connection_mp_ref connection_id,
             COUNT(DISTINCT p.registrator_ref) extra_count
      FROM p916_mp_sales_funnel_turnovers p
      WHERE p.registrator_type='a013_ym_order' AND p.${measure} <> 0
        AND p.cohort_date >= ? AND p.cohort_date < ?
        AND NOT EXISTS (SELECT 1 FROM a013_ym_order s WHERE s.id=p.registrator_ref AND s.is_deleted=0)
      GROUP BY month, connection_id
    )
    SELECT s.month, s.connection_id, COALESCE(c.description,s.connection_id) cabinet,
           COUNT(*) source_count,
           SUM(CASE WHEN p.registrator_ref IS NULL THEN 1 ELSE 0 END) missing_count,
           COALESCE(MAX(e.extra_count),0) extra_count
    FROM src s LEFT JOIN proj p ON p.registrator_ref=s.id AND p.connection_mp_ref=s.connection_id
    LEFT JOIN extra e ON e.month=s.month AND e.connection_id=s.connection_id
    LEFT JOIN a006_connection_mp c ON c.id=s.connection_id
    GROUP BY s.month,s.connection_id,c.description
    UNION ALL
    SELECT e.month,e.connection_id,COALESCE(c.description,e.connection_id),0,0,e.extra_count
    FROM extra e LEFT JOIN a006_connection_mp c ON c.id=e.connection_id
    WHERE NOT EXISTS (SELECT 1 FROM src s WHERE s.month=e.month AND s.connection_id=e.connection_id)
    ORDER BY month,connection_id`, [start, end, start, end]);
}

async function sourceCoverage(host, start, end) {
  const rows = [];

  mergeRows(rows, await a013Coverage(host, start, end, "a013_orders", "order_count", null), "a013_orders");
  mergeRows(rows, await a013Coverage(
    host, start, end, "a013_cancels", "cancel_count",
    // Отмена движения возникает и у доставленных заказов с позициями REJECTED
    // (частичный отказ при получении), поэтому фильтр шире статуса CANCELLED.
    `(status_norm='CANCELLED' OR lines_json LIKE '%REJECTED%')`
  ), "a013_cancels");
  mergeRows(rows, await a013Coverage(
    host, start, end, "a013_buyouts", "buyout_count", `status_norm='DELIVERED'`
  ), "a013_buyouts");

  // a016: когорта возврата — дата заказа, поэтому период режем по заказу из a013.
  mergeRows(rows, await host.db.query(`
    WITH src AS (
      SELECT r.id, substr(o.creation_date,1,7) month, o.connection_id
      FROM a016_ym_returns r
      JOIN a013_ym_order o ON o.document_no = CAST(r.order_id AS TEXT) AND o.is_deleted=0
      WHERE r.is_deleted=0
        AND UPPER(COALESCE(json_extract(r.header_json,'$.return_type'),'')) = 'RETURN'
        AND o.creation_date >= ? AND o.creation_date < ?
    ), proj AS (
      SELECT registrator_ref, connection_mp_ref
      FROM p916_mp_sales_funnel_turnovers
      WHERE registrator_type='a016_ym_returns' AND return_count <> 0
      GROUP BY registrator_ref, connection_mp_ref
    ), extra AS (
      SELECT substr(p.cohort_date,1,7) month, p.connection_mp_ref connection_id,
             COUNT(DISTINCT p.registrator_ref) extra_count
      FROM p916_mp_sales_funnel_turnovers p
      WHERE p.registrator_type='a016_ym_returns' AND p.return_count <> 0
        AND p.cohort_date >= ? AND p.cohort_date < ?
        AND NOT EXISTS (SELECT 1 FROM a016_ym_returns s WHERE s.id=p.registrator_ref AND s.is_deleted=0)
      GROUP BY month, connection_id
    )
    SELECT s.month, s.connection_id, COALESCE(c.description,s.connection_id) cabinet,
           COUNT(*) source_count,
           SUM(CASE WHEN p.registrator_ref IS NULL THEN 1 ELSE 0 END) missing_count,
           COALESCE(MAX(e.extra_count),0) extra_count
    FROM src s LEFT JOIN proj p ON p.registrator_ref=s.id AND p.connection_mp_ref=s.connection_id
    LEFT JOIN extra e ON e.month=s.month AND e.connection_id=s.connection_id
    LEFT JOIN a006_connection_mp c ON c.id=s.connection_id
    GROUP BY s.month,s.connection_id,c.description
    UNION ALL
    SELECT e.month,e.connection_id,COALESCE(c.description,e.connection_id),0,0,e.extra_count
    FROM extra e LEFT JOIN a006_connection_mp c ON c.id=e.connection_id
    WHERE NOT EXISTS (SELECT 1 FROM src s WHERE s.month=e.month AND s.connection_id=e.connection_id)
    ORDER BY month,connection_id`, [start, end, start, end]), "a016_returns");

  // a041: дневная воронка. Документ без активности строк движений не порождает —
  // поэтому в популяцию берём только документы с ненулевыми итогами.
  mergeRows(rows, await host.db.query(`
    WITH src AS (
      SELECT id, substr(document_date,1,7) month, connection_id
      FROM a041_ym_shows_sales_daily
      WHERE is_deleted=0 AND document_date >= ? AND document_date < ?
        AND (COALESCE(total_shows,0) <> 0 OR COALESCE(total_clicks,0) <> 0
          OR COALESCE(total_to_cart,0) <> 0 OR COALESCE(total_order_items,0) <> 0
          OR COALESCE(total_canceled_count,0) <> 0)
    ), proj AS (
      SELECT registrator_ref, connection_mp_ref
      FROM p916_mp_sales_funnel_turnovers
      WHERE registrator_type='a041_ym_shows_sales_daily'
      GROUP BY registrator_ref, connection_mp_ref
    ), extra AS (
      SELECT substr(p.cohort_date,1,7) month, p.connection_mp_ref connection_id,
             COUNT(DISTINCT p.registrator_ref) extra_count
      FROM p916_mp_sales_funnel_turnovers p
      WHERE p.registrator_type='a041_ym_shows_sales_daily'
        AND p.cohort_date >= ? AND p.cohort_date < ?
        AND NOT EXISTS (SELECT 1 FROM a041_ym_shows_sales_daily s WHERE s.id=p.registrator_ref AND s.is_deleted=0)
      GROUP BY month, connection_id
    )
    SELECT s.month, s.connection_id, COALESCE(c.description,s.connection_id) cabinet,
           COUNT(*) source_count,
           SUM(CASE WHEN p.registrator_ref IS NULL THEN 1 ELSE 0 END) missing_count,
           COALESCE(MAX(e.extra_count),0) extra_count
    FROM src s LEFT JOIN proj p ON p.registrator_ref=s.id AND p.connection_mp_ref=s.connection_id
    LEFT JOIN extra e ON e.month=s.month AND e.connection_id=s.connection_id
    LEFT JOIN a006_connection_mp c ON c.id=s.connection_id
    GROUP BY s.month,s.connection_id,c.description
    UNION ALL
    SELECT e.month,e.connection_id,COALESCE(c.description,e.connection_id),0,0,e.extra_count
    FROM extra e LEFT JOIN a006_connection_mp c ON c.id=e.connection_id
    WHERE NOT EXISTS (SELECT 1 FROM src s WHERE s.month=e.month AND s.connection_id=e.connection_id)
    ORDER BY month,connection_id`, [start, end, start, end]), "a041_marketing");

  return rows;
}

/// Санитарные проверки движений YM: событие не может быть раньше собственного
/// заказа, а метрики p916 всегда положительны.
async function movementSanity(host, start, end, connectionRefs) {
  const rows = await host.db.query(`
    SELECT COUNT(*) total_rows,
           SUM(CASE WHEN event_date < cohort_date THEN 1 ELSE 0 END) inverted_rows,
           SUM(CASE WHEN order_count < 0 OR cancel_count < 0 OR buyout_count < 0
                      OR return_count < 0 THEN 1 ELSE 0 END) negative_rows
    FROM p916_mp_sales_funnel_turnovers
    WHERE registrator_type IN ('a013_ym_order','a016_ym_returns')
      AND cohort_date >= ? AND cohort_date < ?
      ${connectionRefs.length > 0 ? `AND connection_mp_ref IN (${connectionRefs.map(() => "?").join(",")})` : ""}`,
    [start, end, ...connectionRefs]);
  return {
    totalRows: n(rows[0]?.total_rows),
    invertedRows: n(rows[0]?.inverted_rows),
    negativeRows: n(rows[0]?.negative_rows)
  };
}

const LABELS = {
  a013_orders: "a013 → p916: заказы",
  a013_cancels: "a013 → p916: отмены и отказы",
  a013_buyouts: "a013 → p916: выкупы",
  a016_returns: "a016 → p916: возвраты (RETURN)",
  a041_marketing: "a041 → p916: воронка (marketing)"
};

function summarize(rows, exact) {
  const grouped = new Map();
  for (const row of rows) {
    const current = grouped.get(row.key) || { source: 0, missing: 0, extra: 0 };
    current.source += row.source; current.missing += row.missing; current.extra += row.extra;
    grouped.set(row.key, current);
  }
  const metrics = [];
  const violations = [];
  for (const key of Object.keys(LABELS)) {
    const value = grouped.get(key) || { source: 0, missing: 0, extra: 0 };
    const sourceMissing = !exact && value.source === 0 ? 1 : 0;
    metrics.push({
      label: LABELS[key],
      population: value.source + value.extra + sourceMissing,
      violations: value.missing + value.extra + sourceMissing,
      unit: "документов/источников"
    });
    if (sourceMissing && violations.length < 20) {
      violations.push({
        violation_type: "source_missing",
        detail: `${LABELS[key]}: источник пуст за проверяемые полные месяцы`
      });
    }
  }
  for (const row of rows) {
    if (violations.length >= 20) break;
    if (row.missing > 0) {
      violations.push({
        violation_type: "projection_missing",
        projection_table: "p916_mp_sales_funnel_turnovers",
        detail: `${LABELS[row.key]}: ${row.month}, ${row.cabinet}, отсутствует ${row.missing} из ${row.source}`
      });
    }
    if (violations.length < 20 && row.extra > 0) {
      violations.push({
        violation_type: "projection_extra",
        projection_table: "p916_mp_sales_funnel_turnovers",
        detail: `${LABELS[row.key]}: ${row.month}, ${row.cabinet}, лишних регистраторов ${row.extra}`
      });
    }
  }
  return { metrics, violations };
}

export async function run(input, host) {
  const bounds = await periodBounds(input,host);
  const connectionRefs = selectedConnections(input);
  const rows = filterConnections(await sourceCoverage(host,bounds.start,bounds.end),connectionRefs);
  const currentRows = bounds.exact ? [] : filterConnections(await sourceCoverage(host,bounds.end,bounds.next),connectionRefs);
  const { metrics, violations } = summarize(rows, bounds.exact);

  const sanity = await movementSanity(host,bounds.start,bounds.end,connectionRefs);
  metrics.push({
    label: "p916 YM: event_date раньше cohort_date",
    population: sanity.totalRows,
    violations: sanity.invertedRows,
    unit: "движений fulfillment"
  });
  metrics.push({
    label: "p916 YM: отрицательные метрики (должны быть положительными)",
    population: sanity.totalRows,
    violations: sanity.negativeRows,
    unit: "движений fulfillment"
  });
  if (sanity.invertedRows > 0 && violations.length < 20) {
    violations.push({
      violation_type: "event_before_cohort",
      projection_table: "p916_mp_sales_funnel_turnovers",
      detail: `Движений YM с event_date < cohort_date: ${sanity.invertedRows}`
    });
  }
  if (sanity.negativeRows > 0 && violations.length < 20) {
    violations.push({
      violation_type: "negative_metric",
      projection_table: "p916_mp_sales_funnel_turnovers",
      detail: `Движений YM с отрицательными метриками: ${sanity.negativeRows} — нужен пересбор воронки (u508)`
    });
  }

  const breakdowns = [{
    key: "by_month_connection",
    title: bounds.exact ? "Расхождения в заданном периоде" : "Расхождения по месяцу и кабинету",
    dimension_label: "Период / кабинет / источник",
    is_partition: false,
    rows: rows.map(row => ({
      label: `${row.month} / ${row.cabinet} / ${LABELS[row.key]}`,
      population: row.source + row.extra,
      violations: row.missing + row.extra
    }))
  }, {
    key: "current_month_observation",
    title: "Текущий незрелый месяц (не входит в итог)",
    dimension_label: "Период / кабинет / источник",
    is_partition: false,
    rows: currentRows.map(row => ({
      label: `${row.month} / ${row.cabinet} / ${LABELS[row.key]}`,
      population: row.source + row.extra,
      violations: row.missing + row.extra
    }))
  }];

  return { metrics, violations, breakdowns, sources: [] };
}
