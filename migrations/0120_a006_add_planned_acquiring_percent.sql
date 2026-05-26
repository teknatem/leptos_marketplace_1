-- a006_connection_mp: planned acquiring percent (used together with planned
-- commission percent in a015_wb_orders margin_pro calculation). Fractional value.

ALTER TABLE a006_connection_mp
    ADD COLUMN planned_acquiring_percent REAL;
