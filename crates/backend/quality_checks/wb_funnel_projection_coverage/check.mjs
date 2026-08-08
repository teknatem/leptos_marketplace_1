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
    const rows = await host.db.query(
      `SELECT ? start_date, date(?,'+1 day') end_date`,
      [String(input.date_from), String(input.date_to)]
    );
    return { start: String(rows[0].start_date), end: String(rows[0].end_date), exact: true };
  }
  const months = Number.isInteger(input?.months) ? input.months : 12;
  const rows = await host.db.query(
    `SELECT date('now','start of month',printf('-%d months',?)) start_date,
            date('now','start of month') end_date,
            date('now','start of month','+1 month') next_date`, [months]
  );
  return {
    start: String(rows[0].start_date),
    end: String(rows[0].end_date),
    next: String(rows[0].next_date),
    exact: false
  };
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

async function sourceCoverage(host, cfg, start, end) {
  const common = [start, end];
  const rows = [];

  mergeRows(rows, await host.db.query(`
    WITH src AS (
      SELECT id, substr(json_extract(state_json,'$.order_dt'),1,7) month,
             json_extract(header_json,'$.connection_id') connection_id
      FROM a015_wb_orders
      WHERE is_deleted=0 AND json_extract(state_json,'$.order_dt') >= ? AND json_extract(state_json,'$.order_dt') < ?
    ), proj AS (
      SELECT registrator_ref, connection_mp_ref
      FROM p916_mp_sales_funnel_turnovers
      WHERE registrator_type='a015_wb_orders' AND order_count <> 0
      GROUP BY registrator_ref, connection_mp_ref
    ), extra AS (
      SELECT substr(p.cohort_date,1,7) month, p.connection_mp_ref connection_id, COUNT(DISTINCT p.registrator_ref) extra_count
      FROM p916_mp_sales_funnel_turnovers p
      WHERE p.registrator_type='a015_wb_orders' AND p.order_count <> 0
        AND p.cohort_date >= ? AND p.cohort_date < ?
        AND NOT EXISTS (SELECT 1 FROM a015_wb_orders s WHERE s.id=p.registrator_ref AND s.is_deleted=0)
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
    ORDER BY month,connection_id`, [...common, ...common]), "a015_orders");

  mergeRows(rows, await host.db.query(`
    WITH src AS (
      SELECT id, substr(json_extract(state_json,'$.order_dt'),1,7) month,
             json_extract(header_json,'$.connection_id') connection_id
      FROM a015_wb_orders
      WHERE is_deleted=0 AND COALESCE(is_cancel,0)=1
        AND json_extract(state_json,'$.order_dt') >= ? AND json_extract(state_json,'$.order_dt') < ?
    ), proj AS (
      SELECT registrator_ref, connection_mp_ref FROM p916_mp_sales_funnel_turnovers
      WHERE registrator_type='a015_wb_orders' AND cancel_count <> 0
      GROUP BY registrator_ref,connection_mp_ref
    ), extra AS (
      SELECT substr(p.cohort_date,1,7) month, p.connection_mp_ref connection_id,
             COUNT(DISTINCT p.registrator_ref) extra_count
      FROM p916_mp_sales_funnel_turnovers p
      WHERE p.registrator_type='a015_wb_orders' AND p.cancel_count <> 0
        AND p.cohort_date >= ? AND p.cohort_date < ?
        AND NOT EXISTS (
          SELECT 1 FROM a015_wb_orders s
          WHERE s.id=p.registrator_ref AND s.is_deleted=0 AND COALESCE(s.is_cancel,0)=1
        )
      GROUP BY month,connection_id
    )
    SELECT s.month,s.connection_id,COALESCE(c.description,s.connection_id) cabinet,
           COUNT(*) source_count,SUM(CASE WHEN p.registrator_ref IS NULL THEN 1 ELSE 0 END) missing_count,
           COALESCE(MAX(e.extra_count),0) extra_count
    FROM src s LEFT JOIN proj p ON p.registrator_ref=s.id AND p.connection_mp_ref=s.connection_id
    LEFT JOIN extra e ON e.month=s.month AND e.connection_id=s.connection_id
    LEFT JOIN a006_connection_mp c ON c.id=s.connection_id
    GROUP BY s.month,s.connection_id,c.description
    UNION ALL
    SELECT e.month,e.connection_id,COALESCE(c.description,e.connection_id),0,0,e.extra_count
    FROM extra e LEFT JOIN a006_connection_mp c ON c.id=e.connection_id
    WHERE NOT EXISTS (SELECT 1 FROM src s WHERE s.month=e.month AND s.connection_id=e.connection_id)
    ORDER BY month,connection_id`, [...common, ...common]), "a015_cancels");

  for (const [key, returnFlag, measure] of [
    ["a012_buyouts", 0, "buyout_count"],
    ["a012_returns", 1, "return_count"]
  ]) {
    mergeRows(rows, await host.db.query(`
      WITH src AS (
        SELECT DISTINCT s.id,substr(json_extract(o.state_json,'$.order_dt'),1,7) month,
               s.connection_id
        FROM a012_wb_sales s
        JOIN a015_wb_orders o
          ON o.document_no=s.document_no
         AND json_extract(o.header_json,'$.connection_id')=s.connection_id
         AND o.is_deleted=0
        WHERE s.is_deleted=0 AND COALESCE(s.is_customer_return,0)=?
          AND json_extract(o.state_json,'$.order_dt') >= ?
          AND json_extract(o.state_json,'$.order_dt') < ?
      ), proj AS (
        SELECT registrator_ref,connection_mp_ref FROM p916_mp_sales_funnel_turnovers
        WHERE registrator_type='a012_wb_sales' AND ${measure}<>0
        GROUP BY registrator_ref,connection_mp_ref
      ), extra AS (
        SELECT substr(p.cohort_date,1,7) month,p.connection_mp_ref connection_id,
               COUNT(DISTINCT p.registrator_ref) extra_count
        FROM p916_mp_sales_funnel_turnovers p
        WHERE p.registrator_type='a012_wb_sales' AND p.${measure}<>0
          AND p.cohort_date >= ? AND p.cohort_date < ?
          AND NOT EXISTS (
            SELECT 1 FROM a012_wb_sales s
            JOIN a015_wb_orders o
              ON o.document_no=s.document_no
             AND json_extract(o.header_json,'$.connection_id')=s.connection_id
             AND o.is_deleted=0
            WHERE s.id=p.registrator_ref AND s.is_deleted=0
              AND COALESCE(s.is_customer_return,0)=?
          )
        GROUP BY month,connection_id
      )
      SELECT s.month,s.connection_id,COALESCE(c.description,s.connection_id) cabinet,
             COUNT(*) source_count,SUM(CASE WHEN p.registrator_ref IS NULL THEN 1 ELSE 0 END) missing_count,
             COALESCE(MAX(e.extra_count),0) extra_count
      FROM src s LEFT JOIN proj p ON p.registrator_ref=s.id AND p.connection_mp_ref=s.connection_id
      LEFT JOIN extra e ON e.month=s.month AND e.connection_id=s.connection_id
      LEFT JOIN a006_connection_mp c ON c.id=s.connection_id
      GROUP BY s.month,s.connection_id,c.description
      UNION ALL
      SELECT e.month,e.connection_id,COALESCE(c.description,e.connection_id),0,0,e.extra_count
      FROM extra e LEFT JOIN a006_connection_mp c ON c.id=e.connection_id
      WHERE NOT EXISTS (SELECT 1 FROM src s WHERE s.month=e.month AND s.connection_id=e.connection_id)
      ORDER BY month,connection_id`,
      [returnFlag, ...common, ...common, returnFlag]), key);
  }

  return rows;
}

/// Санитарные проверки самих движений p916 (в дополнение к покрытию «документ →
/// строка есть»). Ловят три класса дефектов, невидимых для проверки наличия:
///   1. отмена на дате заказа при пустой cancel_dt — событие село на неверный день
///      (фолбэк builder'а). Для новых периодов доля должна стремиться к нулю;
///   2. event_date < cohort_date — событие раньше собственного заказа;
///   3. отрицательные метрики — в p916 все величины положительные.
async function movementSanity(host, start, end, connectionRefs) {
  const scoped = connectionRefs.length > 0
    ? ` AND p.connection_mp_ref IN (${connectionRefs.map(() => "?").join(",")})`
    : "";
  const rows = await host.db.query(`
    SELECT
      COUNT(*) total_cancels,
      SUM(CASE WHEN p.event_date = p.cohort_date
                AND json_extract(o.state_json,'$.cancel_dt') IS NULL
               THEN 1 ELSE 0 END) fallback_cancels
    FROM p916_mp_sales_funnel_turnovers p
    JOIN a015_wb_orders o ON o.id = p.registrator_ref AND o.is_deleted = 0
    WHERE p.registrator_type='a015_wb_orders' AND p.cancel_count <> 0
      AND p.cohort_date >= ? AND p.cohort_date < ?${scoped}`,
    [start, end, ...connectionRefs]);

  const order = await host.db.query(`
    SELECT COUNT(*) total_rows,
           SUM(CASE WHEN event_date < cohort_date THEN 1 ELSE 0 END) inverted_rows,
           SUM(CASE WHEN order_count < 0 OR cancel_count < 0 OR buyout_count < 0
                      OR return_count < 0 OR order_sum < 0 OR cancel_sum < 0
                      OR buyout_sum < 0 OR return_sum < 0
                    THEN 1 ELSE 0 END) negative_rows
    FROM p916_mp_sales_funnel_turnovers
    WHERE stage='fulfillment' AND cohort_date >= ? AND cohort_date < ?
      ${connectionRefs.length > 0 ? `AND connection_mp_ref IN (${connectionRefs.map(() => "?").join(",")})` : ""}`,
    [start, end, ...connectionRefs]);

  return {
    totalCancels: n(rows[0]?.total_cancels),
    fallbackCancels: n(rows[0]?.fallback_cancels),
    totalRows: n(order[0]?.total_rows),
    invertedRows: n(order[0]?.inverted_rows),
    negativeRows: n(order[0]?.negative_rows)
  };
}

function summarize(rows, exact) {
  const labels = {
    a015_orders: "a015 → p916: заказы",
    a015_cancels: "a015 → p916: отмены",
    a012_sales: "a012 → p916: выкупы/возвраты"
  };
  labels.a012_buyouts = `${labels.a012_sales}: buyouts`;
  labels.a012_returns = `${labels.a012_sales}: returns`;
  delete labels.a012_sales;
  const grouped = new Map();
  for (const row of rows) {
    const current = grouped.get(row.key) || { source: 0, missing: 0, extra: 0 };
    current.source += row.source; current.missing += row.missing; current.extra += row.extra;
    grouped.set(row.key, current);
  }
  const metrics = [];
  const violations = [];
  for (const key of Object.keys(labels)) {
    const value = grouped.get(key) || { source: 0, missing: 0, extra: 0 };
    const sourceMissing = !exact && value.source === 0 ? 1 : 0;
    metrics.push({
      label: labels[key],
      population: value.source + value.extra + sourceMissing,
      violations: value.missing + value.extra + sourceMissing,
      unit: "документов/источников"
    });
    if (sourceMissing && violations.length < 20) {
      violations.push({ violation_type: "source_missing", detail: `${labels[key]}: источник пуст за проверяемые полные месяцы` });
    }
  }
  for (const row of rows) {
    if (violations.length >= 20) break;
    if (row.missing > 0) violations.push({ violation_type: "projection_missing", projection_table: "p916_mp_sales_funnel_turnovers", detail: `${labels[row.key]}: ${row.month}, ${row.cabinet}, отсутствует ${row.missing} из ${row.source}` });
    if (violations.length < 20 && row.extra > 0) violations.push({ violation_type: "projection_extra", projection_table: "p916_mp_sales_funnel_turnovers", detail: `${labels[row.key]}: ${row.month}, ${row.cabinet}, лишних регистраторов ${row.extra}` });
  }
  return { metrics, violations, labels };
}

export async function run(input, host) {
  const bounds = await periodBounds(input, host);
  const connectionRefs = selectedConnections(input);
  const rows = filterConnections(
    await sourceCoverage(host, input, bounds.start, bounds.end),
    connectionRefs
  );
  const currentRows = bounds.exact ? [] : filterConnections(
    await sourceCoverage(host, input, bounds.end, bounds.next),
    connectionRefs
  );
  const { metrics, violations, labels } = summarize(rows, bounds.exact);

  const sanity = await movementSanity(host, bounds.start, bounds.end, connectionRefs);
  metrics.push({
    label: "p916: отмены с датой-фолбэком (cancel_dt пуста)",
    population: sanity.totalCancels,
    violations: sanity.fallbackCancels,
    unit: "движений отмены"
  });
  metrics.push({
    label: "p916: event_date раньше cohort_date",
    population: sanity.totalRows,
    violations: sanity.invertedRows,
    unit: "движений fulfillment"
  });
  metrics.push({
    label: "p916: отрицательные метрики (должны быть положительными)",
    population: sanity.totalRows,
    violations: sanity.negativeRows,
    unit: "движений fulfillment"
  });
  if (sanity.fallbackCancels > 0 && violations.length < 20) {
    violations.push({
      violation_type: "cancel_date_fallback",
      projection_table: "p916_mp_sales_funnel_turnovers",
      detail: `Отмен без даты отмены: ${sanity.fallbackCancels} из ${sanity.totalCancels} — событие село на дату заказа, потоковая ось искажена`
    });
  }
  if (sanity.invertedRows > 0 && violations.length < 20) {
    violations.push({
      violation_type: "event_before_cohort",
      projection_table: "p916_mp_sales_funnel_turnovers",
      detail: `Движений с event_date < cohort_date: ${sanity.invertedRows}`
    });
  }
  if (sanity.negativeRows > 0 && violations.length < 20) {
    violations.push({
      violation_type: "negative_metric",
      projection_table: "p916_mp_sales_funnel_turnovers",
      detail: `Движений с отрицательными метриками: ${sanity.negativeRows} — нужен пересбор воронки (u508)`
    });
  }
  const breakdowns = [{
    key: "by_month_connection",
    title: bounds.exact ? "Расхождения в заданном периоде" : "Расхождения по месяцу и кабинету",
    dimension_label: "Период / кабинет / источник",
    is_partition: false,
    rows: rows.map(row => ({ label: `${row.month} / ${row.cabinet} / ${labels[row.key]}`, population: row.source + row.extra, violations: row.missing + row.extra }))
  }, {
    key: "current_month_observation",
    title: "Текущий незрелый месяц (не входит в итог)",
    dimension_label: "Период / кабинет / источник",
    is_partition: false,
    rows: currentRows.map(row => ({ label: `${row.month} / ${row.cabinet} / ${labels[row.key]}`, population: row.source + row.extra, violations: row.missing + row.extra }))
  }];
  return { metrics, violations, breakdowns, sources: [] };
}
