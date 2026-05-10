-- Repair marketplace_raw_payload_ref with actual Marketplace API payloads.
--
-- Migration 0090 intentionally avoided expensive raw_json LIKE scans, but for
-- orders that had several Statistics payloads it could choose an older
-- Statistics raw row instead of the /api/v3/orders raw row. This migration
-- identifies Marketplace API payloads by JSON fields that are specific to
-- /api/v3/orders (`id` + `createdAt`) and stores the latest one per document.

DROP TABLE IF EXISTS _tmp_wb_orders_marketplace_raw;

CREATE TABLE _tmp_wb_orders_marketplace_raw (
    document_no TEXT PRIMARY KEY,
    raw_payload_ref TEXT NOT NULL
);

INSERT OR REPLACE INTO _tmp_wb_orders_marketplace_raw (document_no, raw_payload_ref)
SELECT document_no, id
FROM (
    SELECT
        document_no,
        id,
        ROW_NUMBER() OVER (
            PARTITION BY document_no
            ORDER BY created_at DESC
        ) AS rn
    FROM document_raw_storage
    WHERE document_type = 'WB_Orders'
      AND json_type(raw_json, '$.id') IS NOT NULL
      AND json_type(raw_json, '$.createdAt') IS NOT NULL
)
WHERE rn = 1;

UPDATE a015_wb_orders
SET source_meta_json = json_set(
    source_meta_json,
    '$.marketplace_raw_payload_ref',
    (
        SELECT raw_payload_ref
        FROM _tmp_wb_orders_marketplace_raw
        WHERE _tmp_wb_orders_marketplace_raw.document_no = a015_wb_orders.document_no
    )
)
WHERE is_deleted = 0
  AND EXISTS (
      SELECT 1
      FROM _tmp_wb_orders_marketplace_raw
      WHERE _tmp_wb_orders_marketplace_raw.document_no = a015_wb_orders.document_no
  );

DROP TABLE IF EXISTS _tmp_wb_orders_marketplace_raw;
