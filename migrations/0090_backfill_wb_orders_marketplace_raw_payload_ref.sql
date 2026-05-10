-- Backfill a separate raw payload reference for WB Marketplace API orders.
--
-- Historically a015_wb_orders had a single source_meta.raw_payload_ref.
-- task002 (Statistics API) overwrote it with /api/v1/supplier/orders payloads,
-- so the details page could no longer show the Marketplace API card even when
-- older /api/v3/orders raw payloads were present in document_raw_storage.

CREATE INDEX IF NOT EXISTS idx_document_raw_storage_type_no_created
ON document_raw_storage(document_type, document_no, created_at);

UPDATE a015_wb_orders
SET source_meta_json = json_set(
    source_meta_json,
    '$.marketplace_raw_payload_ref',
    (
        SELECT drs.id
        FROM document_raw_storage drs
        WHERE drs.document_no = a015_wb_orders.document_no
          AND drs.document_type = 'WB_Orders'
          AND drs.created_at < json_extract(a015_wb_orders.source_meta_json, '$.fetched_at')
        ORDER BY drs.created_at DESC
        LIMIT 1
    )
)
WHERE is_deleted = 0
  AND json_extract(source_meta_json, '$.marketplace_raw_payload_ref') IS NULL
  AND EXISTS (
      SELECT 1
      FROM document_raw_storage drs
      WHERE drs.document_no = a015_wb_orders.document_no
        AND drs.document_type = 'WB_Orders'
        AND drs.created_at < json_extract(a015_wb_orders.source_meta_json, '$.fetched_at')
  );
