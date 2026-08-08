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

const SOURCES = [
  { table: "a026_wb_advert_daily", registrator: "a026_wb_advert_daily", label: "a026 → p916: платная реклама" },
  { table: "a036_wb_sales_funnel_daily", registrator: "a036_wb_sales_funnel_daily", label: "a036 → p916: воронка" }
];

async function coverage(host, source, start, end) {
  return await host.db.query(`
    WITH src AS (
      SELECT id,substr(document_date,1,7) month,connection_id
      FROM ${source.table}
      WHERE is_deleted=0 AND document_date>=? AND document_date<?
    ), proj AS (
      SELECT registrator_ref,connection_mp_ref
      FROM p916_mp_sales_funnel_turnovers
      WHERE registrator_type='${source.registrator}'
      GROUP BY registrator_ref,connection_mp_ref
    ), extra AS (
      SELECT substr(p.event_date,1,7) month,p.connection_mp_ref connection_id,
             COUNT(DISTINCT p.registrator_ref) extra_count
      FROM p916_mp_sales_funnel_turnovers p
      WHERE p.registrator_type='${source.registrator}' AND p.event_date>=? AND p.event_date<?
        AND NOT EXISTS (SELECT 1 FROM ${source.table} s WHERE s.id=p.registrator_ref AND s.is_deleted=0)
      GROUP BY month,connection_id
    )
    SELECT s.month,s.connection_id,COALESCE(c.description,s.connection_id) cabinet,COUNT(*) source_count,
           SUM(CASE WHEN p.registrator_ref IS NULL THEN 1 ELSE 0 END) missing_count,
           COALESCE(MAX(e.extra_count),0) extra_count
    FROM src s
    LEFT JOIN proj p ON p.registrator_ref=s.id AND p.connection_mp_ref=s.connection_id
    LEFT JOIN extra e ON e.month=s.month AND e.connection_id=s.connection_id
    LEFT JOIN a006_connection_mp c ON c.id=s.connection_id
    GROUP BY s.month,s.connection_id,c.description
    UNION ALL
    SELECT e.month,e.connection_id,COALESCE(c.description,e.connection_id),0,0,e.extra_count
    FROM extra e LEFT JOIN a006_connection_mp c ON c.id=e.connection_id
    WHERE NOT EXISTS (SELECT 1 FROM src s WHERE s.month=e.month AND s.connection_id=e.connection_id)
    ORDER BY month,cabinet`, [start,end,start,end]);
}

function metric(source, rows) {
  const sourceCount = rows.reduce((sum,row) => sum+n(row.source_count),0);
  const extra = rows.reduce((sum,row) => sum+n(row.extra_count),0);
  const violations = rows.reduce((sum,row) => sum+n(row.missing_count)+n(row.extra_count),0);
  const sourceMissing = sourceCount === 0 ? 1 : 0;
  return { label: source.label, population: sourceCount+extra+sourceMissing, violations: violations+sourceMissing, unit: "дневных срезов/источников" };
}

export async function run(input, host) {
  const bounds = await periodBounds(input,host);
  const connectionRefs = selectedConnections(input);
  const metrics=[]; const violations=[]; const mature=[]; const current=[];
  for (const source of SOURCES) {
    const rows=filterConnections(await coverage(host,source,bounds.start,bounds.end),connectionRefs);
    const currentRows=bounds.exact?[]:filterConnections(await coverage(host,source,bounds.end,bounds.next),connectionRefs);
    const item=metric(source,rows); metrics.push(item);
    if (rows.reduce((sum,row)=>sum+n(row.source_count),0)===0) violations.push({violation_type:"source_missing",detail:`${source.label}: источник пуст за проверяемые полные месяцы`});
    for (const row of rows) {
      mature.push({label:`${row.month} / ${row.cabinet} / ${source.label}`,population:n(row.source_count)+n(row.extra_count),violations:n(row.missing_count)+n(row.extra_count)});
      if (n(row.missing_count)>0 && violations.length<20) violations.push({violation_type:"projection_missing",projection_table:"p916_mp_sales_funnel_turnovers",detail:`${source.label}: ${row.month}, ${row.cabinet}, отсутствует ${row.missing_count}`});
      if (n(row.extra_count)>0 && violations.length<20) violations.push({violation_type:"projection_extra",projection_table:"p916_mp_sales_funnel_turnovers",detail:`${source.label}: ${row.month}, ${row.cabinet}, лишних ${row.extra_count}`});
    }
    for (const row of currentRows) current.push({label:`${row.month} / ${row.cabinet} / ${source.label}`,population:n(row.source_count)+n(row.extra_count),violations:n(row.missing_count)+n(row.extra_count)});
  }
  return {metrics,violations,breakdowns:[
    {key:"by_month_connection",title:bounds.exact?"Расхождения в заданном периоде":"Расхождения по месяцу и кабинету",dimension_label:"Период / кабинет / источник",is_partition:false,rows:mature},
    {key:"current_month_observation",title:"Текущий незрелый месяц (не входит в итог)",dimension_label:"Период / кабинет / источник",is_partition:false,rows:current}
  ],sources:[]};
}
