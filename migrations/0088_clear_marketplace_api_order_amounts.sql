-- Marketplace API (/api/v3/orders, /api/v3/orders/new) does not return
-- Statistics API amount fields totalPrice, priceWithDisc, finishedPrice.
-- Older imports copied Marketplace `price` into those fields; clear them so
-- stored a015_wb_orders line_json matches the source endpoint semantics.
UPDATE a015_wb_orders
SET line_json = json_set(
    line_json,
    '$.total_price', NULL,
    '$.price_with_disc', NULL,
    '$.finished_price', NULL,
    '$.discount_percent', NULL,
    '$.spp', NULL,
    '$.margin_pro', NULL
)
WHERE is_deleted = 0
  AND EXISTS (
      SELECT 1
      FROM document_raw_storage r
      WHERE r.id = json_extract(a015_wb_orders.source_meta_json, '$.raw_payload_ref')
        AND json_extract(r.raw_json, '$.price') IS NOT NULL
        AND json_extract(r.raw_json, '$.totalPrice') IS NULL
        AND json_extract(r.raw_json, '$.priceWithDisc') IS NULL
        AND json_extract(r.raw_json, '$.finishedPrice') IS NULL
  );
