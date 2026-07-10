-- Convert the audited WB chart to the canonical declarative live/snapshot path.
-- The WHERE clauses make this migration a no-op on installations without this plugin.

UPDATE plugin
SET data_json = json_object(
        'source', json_object(
            'kind', 'sql',
            'sql', 'SELECT COALESCE(NULLIF(a004_nomenclature.dim1_category, ''''), ''Без категории'') AS category, SUM(COALESCE(p904_sales_data.customer_in, 0)) AS sales_amount
FROM p904_sales_data
LEFT JOIN a004_nomenclature ON p904_sales_data.nomenclature_ref = a004_nomenclature.id
WHERE p904_sales_data.connection_mp_ref IN (''1386a311-1e26-4676-b696-8d577a119eec'', ''42e29532-72b1-4f38-be6e-38c331c61fe6'')
  AND substr(p904_sales_data.date, 1, 10) BETWEEN ''2026-05-01'' AND ''2026-05-31''
GROUP BY COALESCE(NULLIF(a004_nomenclature.dim1_category, ''''), ''Без категории'')
HAVING SUM(COALESCE(p904_sales_data.customer_in, 0)) <> 0
ORDER BY SUM(COALESCE(p904_sales_data.customer_in, 0)) DESC',
            'params', json('[]')
        ),
        'default_mode', 'live'
    ),
    manifest_json = json_set(manifest_json, '$.runtime', 'client'),
    runtime = 'client',
    server_script = NULL,
    assets_json = json_set(assets_json, '$.sql_resources', json('{}')),
    view_spec_json = json_object(
        'widgets', json_array(json_object(
            'kind', 'chart',
            'title', 'Продажи Wildberries по категориям номенклатуры — май 2026',
            'config', json_object(
                'type', 'bar',
                'title', 'Продажи Wildberries по категориям номенклатуры — май 2026',
                'horizontal', json('true'),
                'format', 'money',
                'x', 'category',
                'series', json_array(json_object('label', 'Продажи', 'y', 'sales_amount')),
                'alternatives', json_array('doughnut', 'pie')
            )
        ))
    ),
    version = version + 1,
    updated_at = datetime('now')
WHERE id = 'e84d00c3-c269-48d4-a200-5dd099683bb2'
  AND json_extract(data_json, '$.source') IS NULL;

INSERT OR REPLACE INTO plugin_snapshot
    (id, plugin_id, plugin_version, method, payload_json, row_count, size_bytes, source_hash, created_at)
SELECT
    'wb-chart-may-2026-data',
    plugin.id,
    plugin.version,
    'data',
    snapshot.payload,
    snapshot.row_count,
    length(CAST(snapshot.payload AS BLOB)),
    '54aae3d71a795a41d82aebb453f9ff00693f7261d5ced517ef0c4196ab5d7dcc',
    datetime('now')
FROM plugin
CROSS JOIN (
    SELECT
        json_group_array(json_object('category', category, 'sales_amount', sales_amount)) AS payload,
        COUNT(*) AS row_count
    FROM (
        SELECT
            COALESCE(NULLIF(a004_nomenclature.dim1_category, ''), 'Без категории') AS category,
            SUM(COALESCE(p904_sales_data.customer_in, 0)) AS sales_amount
        FROM p904_sales_data
        LEFT JOIN a004_nomenclature
            ON p904_sales_data.nomenclature_ref = a004_nomenclature.id
        WHERE p904_sales_data.connection_mp_ref IN (
            '1386a311-1e26-4676-b696-8d577a119eec',
            '42e29532-72b1-4f38-be6e-38c331c61fe6'
        )
          AND substr(p904_sales_data.date, 1, 10) BETWEEN '2026-05-01' AND '2026-05-31'
        GROUP BY COALESCE(NULLIF(a004_nomenclature.dim1_category, ''), 'Без категории')
        HAVING SUM(COALESCE(p904_sales_data.customer_in, 0)) <> 0
        ORDER BY SUM(COALESCE(p904_sales_data.customer_in, 0)) DESC
    ) AS rows
) AS snapshot
WHERE plugin.id = 'e84d00c3-c269-48d4-a200-5dd099683bb2';

INSERT OR IGNORE INTO plugin_revision
    (id, plugin_id, version, bundle_json, validate_report_json, smoke_report_json,
     created_by_agent_id, source_spec_json, source_hash, snapshot_meta_json, origin_json, created_at)
SELECT
    'wb-chart-may-2026-revision',
    plugin.id,
    plugin.version,
    json_object(
        'manifest', json(plugin.manifest_json),
        'params', json(plugin.params_json),
        'data', json(plugin.data_json),
        'client_script', plugin.client_script,
        'server_script', plugin.server_script,
        'view_spec', json(plugin.view_spec_json),
        'styles', plugin.styles,
        'sql_resources', COALESCE(json_extract(plugin.assets_json, '$.sql_resources'), json('{}')),
        'assets', COALESCE(json_extract(plugin.assets_json, '$.assets'), json('{}'))
    ),
    json_object('ok', json('true'), 'errors', json('[]')),
    json_object('ok', json('true'), 'validation', 'migration', 'source_resolved', json('true'), 'snapshot_created', json('true')),
    plugin.created_by_agent_id,
    json_extract(plugin.data_json, '$.source'),
    snapshot.source_hash,
    json_object(
        'plugin_version', plugin.version,
        'created_at', snapshot.created_at,
        'row_count', snapshot.row_count,
        'size_bytes', snapshot.size_bytes,
        'source_hash', snapshot.source_hash
    ),
    json_object('migration', '0153', 'chat_id', '7d860f47-b10f-436a-8428-26c2abce8710'),
    datetime('now')
FROM plugin
JOIN plugin_snapshot AS snapshot
  ON snapshot.plugin_id = plugin.id
 AND snapshot.plugin_version = plugin.version
 AND snapshot.method = 'data'
WHERE plugin.id = 'e84d00c3-c269-48d4-a200-5dd099683bb2';
