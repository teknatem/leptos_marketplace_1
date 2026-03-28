ALTER TABLE p903_wb_finance_report
    RENAME TO p903_wb_finance_report_old;

CREATE TABLE p903_wb_finance_report (
    id TEXT NOT NULL PRIMARY KEY,
    rr_dt TEXT NOT NULL,
    rrd_id INTEGER NOT NULL,
    source_row_ref TEXT NOT NULL,
    connection_mp_ref TEXT NOT NULL,
    organization_ref TEXT NOT NULL,
    acquiring_fee REAL,
    acquiring_percent REAL,
    additional_payment REAL,
    bonus_type_name TEXT,
    commission_percent REAL,
    delivery_amount REAL,
    delivery_rub REAL,
    nm_id INTEGER,
    penalty REAL,
    ppvz_vw REAL,
    ppvz_vw_nds REAL,
    ppvz_sales_commission REAL,
    quantity INTEGER,
    rebill_logistic_cost REAL,
    retail_amount REAL,
    retail_price REAL,
    retail_price_withdisc_rub REAL,
    return_amount REAL,
    sa_name TEXT,
    storage_fee REAL,
    subject_name TEXT,
    supplier_oper_name TEXT,
    cashback_amount REAL,
    ppvz_for_pay REAL,
    ppvz_kvw_prc REAL,
    ppvz_kvw_prc_base REAL,
    srv_dbs INTEGER,
    loaded_at_utc TEXT NOT NULL,
    payload_version INTEGER NOT NULL DEFAULT 1,
    extra TEXT,
    srid TEXT
);

INSERT INTO p903_wb_finance_report (
    id,
    rr_dt,
    rrd_id,
    source_row_ref,
    connection_mp_ref,
    organization_ref,
    acquiring_fee,
    acquiring_percent,
    additional_payment,
    bonus_type_name,
    commission_percent,
    delivery_amount,
    delivery_rub,
    nm_id,
    penalty,
    ppvz_vw,
    ppvz_vw_nds,
    ppvz_sales_commission,
    quantity,
    rebill_logistic_cost,
    retail_amount,
    retail_price,
    retail_price_withdisc_rub,
    return_amount,
    sa_name,
    storage_fee,
    subject_name,
    supplier_oper_name,
    cashback_amount,
    ppvz_for_pay,
    ppvz_kvw_prc,
    ppvz_kvw_prc_base,
    srv_dbs,
    loaded_at_utc,
    payload_version,
    extra,
    srid
)
SELECT
    id,
    rr_dt,
    rrd_id,
    source_row_ref,
    connection_mp_ref,
    organization_ref,
    acquiring_fee,
    acquiring_percent,
    additional_payment,
    bonus_type_name,
    commission_percent,
    delivery_amount,
    delivery_rub,
    nm_id,
    penalty,
    ppvz_vw,
    ppvz_vw_nds,
    ppvz_sales_commission,
    quantity,
    rebill_logistic_cost,
    retail_amount,
    retail_price,
    retail_price_withdisc_rub,
    return_amount,
    sa_name,
    storage_fee,
    subject_name,
    supplier_oper_name,
    cashback_amount,
    ppvz_for_pay,
    ppvz_kvw_prc,
    ppvz_kvw_prc_base,
    srv_dbs,
    loaded_at_utc,
    payload_version,
    extra,
    srid
FROM p903_wb_finance_report_old;

DROP TABLE p903_wb_finance_report_old;

CREATE UNIQUE INDEX idx_p903_rrd_id
    ON p903_wb_finance_report (rrd_id);

CREATE UNIQUE INDEX idx_p903_source_row_ref
    ON p903_wb_finance_report (source_row_ref);

CREATE INDEX idx_p903_rr_dt
    ON p903_wb_finance_report (rr_dt);

CREATE INDEX idx_p903_nm_id
    ON p903_wb_finance_report (nm_id);

CREATE INDEX idx_p903_connection_mp_ref
    ON p903_wb_finance_report (connection_mp_ref);

CREATE INDEX idx_p903_organization_ref
    ON p903_wb_finance_report (organization_ref);

CREATE INDEX idx_p903_supplier_oper_name
    ON p903_wb_finance_report (supplier_oper_name);

CREATE INDEX idx_p903_rr_dt_org
    ON p903_wb_finance_report (rr_dt, organization_ref);
