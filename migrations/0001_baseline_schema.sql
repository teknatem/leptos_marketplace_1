-- 0001 baseline schema
-- Generated from db.rs and migrate_*.sql
-- Used for fresh installations.


CREATE TABLE IF NOT EXISTS a001_connection_1c_database (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                url TEXT NOT NULL,
                login TEXT NOT NULL,
                password TEXT NOT NULL,
                is_primary INTEGER NOT NULL DEFAULT 0,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            
);

CREATE TABLE IF NOT EXISTS a002_organization (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                full_name TEXT NOT NULL,
                inn TEXT NOT NULL,
                kpp TEXT NOT NULL DEFAULT '',
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            
);

CREATE TABLE IF NOT EXISTS a003_counterparty (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                is_folder INTEGER NOT NULL DEFAULT 0,
                parent_id TEXT,
                inn TEXT NOT NULL DEFAULT '',
                kpp TEXT NOT NULL DEFAULT '',
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            
);

CREATE TABLE IF NOT EXISTS a004_nomenclature (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                full_description TEXT NOT NULL DEFAULT '',
                comment TEXT,
                is_folder INTEGER NOT NULL DEFAULT 0,
                parent_id TEXT,
                article TEXT NOT NULL DEFAULT '',
                mp_ref_count INTEGER NOT NULL DEFAULT 0,
                dim1_category TEXT NOT NULL DEFAULT '',
                dim2_line TEXT NOT NULL DEFAULT '',
                dim3_model TEXT NOT NULL DEFAULT '',
                dim4_format TEXT NOT NULL DEFAULT '',
                dim5_sink TEXT NOT NULL DEFAULT '',
                dim6_size TEXT NOT NULL DEFAULT '',
                is_assembly INTEGER NOT NULL DEFAULT 0,
                base_nomenclature_ref TEXT,
                is_derivative INTEGER NOT NULL DEFAULT 0,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            
);

CREATE TABLE IF NOT EXISTS a005_marketplace (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                url TEXT NOT NULL,
                logo_path TEXT,
                marketplace_type TEXT,
                acquiring_fee_pro REAL DEFAULT 0.0,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            
);

CREATE TABLE IF NOT EXISTS a006_connection_mp (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                marketplace TEXT NOT NULL,
                organization TEXT NOT NULL,
                organization_ref TEXT NOT NULL DEFAULT '',
                api_key TEXT NOT NULL,
                supplier_id TEXT,
                application_id TEXT,
                is_used INTEGER NOT NULL DEFAULT 0,
                business_account_id TEXT,
                api_key_stats TEXT,
                test_mode INTEGER NOT NULL DEFAULT 0,
                planned_commission_percent REAL,
                authorization_type TEXT NOT NULL DEFAULT 'API Key',
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            
);

CREATE TABLE IF NOT EXISTS a007_marketplace_product (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                marketplace_ref TEXT NOT NULL,
                connection_mp_ref TEXT NOT NULL DEFAULT '',
                marketplace_sku TEXT NOT NULL,
                barcode TEXT,
                article TEXT NOT NULL,
                brand TEXT,
                category_id TEXT,
                category_name TEXT,
                last_update TEXT,
                nomenclature_ref TEXT,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            
);

CREATE TABLE IF NOT EXISTS a008_marketplace_sales (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                connection_id TEXT NOT NULL,
                organization_id TEXT NOT NULL,
                marketplace_id TEXT NOT NULL,
                accrual_date TEXT NOT NULL,
                product_id TEXT NOT NULL,
                quantity INTEGER NOT NULL,
                revenue REAL NOT NULL,
                operation_type TEXT NOT NULL DEFAULT '',
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            
);

CREATE TABLE IF NOT EXISTS a009_ozon_returns (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                connection_id TEXT NOT NULL,
                organization_id TEXT NOT NULL,
                marketplace_id TEXT NOT NULL,
                return_id TEXT NOT NULL,
                return_date TEXT NOT NULL,
                return_reason_name TEXT NOT NULL,
                return_type TEXT NOT NULL,
                order_id TEXT NOT NULL,
                order_number TEXT NOT NULL,
                sku TEXT NOT NULL,
                product_name TEXT NOT NULL,
                price REAL NOT NULL,
                quantity INTEGER NOT NULL,
                posting_number TEXT NOT NULL,
                clearing_id TEXT,
                return_clearing_id TEXT,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            
);

CREATE TABLE IF NOT EXISTS system_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp TEXT NOT NULL,
                source TEXT NOT NULL,
                category TEXT NOT NULL,
                message TEXT NOT NULL
            
);

CREATE TABLE IF NOT EXISTS document_raw_storage (
                id TEXT PRIMARY KEY NOT NULL,
                marketplace TEXT NOT NULL,
                document_type TEXT NOT NULL,
                document_no TEXT NOT NULL,
                raw_json TEXT NOT NULL,
                fetched_at TEXT NOT NULL,
                created_at TEXT NOT NULL
            
);

CREATE TABLE IF NOT EXISTS p900_sales_register (
                -- NK (Natural Key)
                marketplace TEXT NOT NULL,
                document_no TEXT NOT NULL,
                line_id TEXT NOT NULL,

                -- Metadata
                scheme TEXT,
                document_type TEXT NOT NULL,
                document_version INTEGER NOT NULL DEFAULT 1,

                -- References to aggregates (UUID)
                connection_mp_ref TEXT NOT NULL,
                organization_ref TEXT NOT NULL,
                marketplace_product_ref TEXT,
                nomenclature_ref TEXT,
                registrator_ref TEXT NOT NULL,
                
                -- Timestamps and status
                event_time_source TEXT NOT NULL,
                sale_date TEXT NOT NULL,
                source_updated_at TEXT,
                status_source TEXT NOT NULL,
                status_norm TEXT NOT NULL,
                
                -- Product identification
                seller_sku TEXT,
                mp_item_id TEXT NOT NULL,
                barcode TEXT,
                title TEXT,
                
                -- Quantities and money
                qty REAL NOT NULL,
                price_list REAL,
                discount_total REAL,
                price_effective REAL,
                amount_line REAL,
                cost REAL,
                dealer_price_ut REAL,
                currency_code TEXT,
                is_fact INTEGER,
                
                -- Technical fields
                loaded_at_utc TEXT NOT NULL,
                payload_version INTEGER NOT NULL DEFAULT 1,
                extra TEXT,
                
                PRIMARY KEY (marketplace, document_no, line_id)
            
);

CREATE TABLE IF NOT EXISTS p901_nomenclature_barcodes (
                    barcode TEXT NOT NULL,
                    source TEXT NOT NULL,
                    nomenclature_ref TEXT,
                    article TEXT,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    is_active INTEGER NOT NULL DEFAULT 1,
                    PRIMARY KEY (barcode, source)
                
);

CREATE TABLE IF NOT EXISTS a010_ozon_fbs_posting (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                document_no TEXT NOT NULL UNIQUE,
                status_norm TEXT NOT NULL DEFAULT '',
                substatus_raw TEXT,
                header_json TEXT NOT NULL,
                lines_json TEXT NOT NULL,
                state_json TEXT NOT NULL,
                source_meta_json TEXT NOT NULL,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            
);

CREATE TABLE IF NOT EXISTS a011_ozon_fbo_posting (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                document_no TEXT NOT NULL UNIQUE,
                status_norm TEXT NOT NULL DEFAULT '',
                substatus_raw TEXT,
                created_at_source TEXT,
                header_json TEXT NOT NULL,
                lines_json TEXT NOT NULL,
                state_json TEXT NOT NULL,
                source_meta_json TEXT NOT NULL,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            
);

CREATE TABLE IF NOT EXISTS a012_wb_sales (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                document_no TEXT NOT NULL,
                sale_id TEXT NOT NULL UNIQUE,
                -- Denormalized fields from JSON for fast queries
                sale_date TEXT,
                organization_id TEXT,
                connection_id TEXT,
                supplier_article TEXT,
                nm_id INTEGER,
                barcode TEXT,
                product_name TEXT,
                qty REAL,
                amount_line REAL,
                total_price REAL,
                finished_price REAL,
                event_type TEXT,
                -- JSON storage (kept for backward compatibility and full data)
                header_json TEXT NOT NULL,
                line_json TEXT NOT NULL,
                state_json TEXT NOT NULL,
                warehouse_json TEXT,
                source_meta_json TEXT NOT NULL,
                marketplace_product_ref TEXT,
                nomenclature_ref TEXT,
                base_nomenclature_ref TEXT,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            ,
    is_fact INTEGER,
    sell_out_plan REAL,
    sell_out_fact REAL,
    acquiring_fee_plan REAL,
    acquiring_fee_fact REAL,
    other_fee_plan REAL,
    other_fee_fact REAL,
    supplier_payout_plan REAL,
    supplier_payout_fact REAL,
    profit_plan REAL,
    profit_fact REAL,
    cost_of_production REAL,
    commission_plan REAL,
    commission_fact REAL
);

CREATE TABLE IF NOT EXISTS a013_ym_order (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                document_no TEXT NOT NULL UNIQUE,
                header_json TEXT NOT NULL,
                lines_json TEXT NOT NULL,
                state_json TEXT NOT NULL,
                source_meta_json TEXT NOT NULL,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                is_error INTEGER NOT NULL DEFAULT 0,
                -- Р”РµРЅРѕСЂРјР°Р»РёР·РѕРІР°РЅРЅС‹Рµ РїРѕР»СЏ РґР»СЏ Р±С‹СЃС‚СЂС‹С… Р·Р°РїСЂРѕСЃРѕРІ СЃРїРёСЃРєР°
                status_changed_at TEXT,
                creation_date TEXT,
                delivery_date TEXT,
                campaign_id TEXT,
                status_norm TEXT,
                total_qty REAL DEFAULT 0,
                total_amount REAL DEFAULT 0,
                total_amount_api REAL,
                lines_count INTEGER DEFAULT 0,
                delivery_total REAL,
                subsidies_total REAL DEFAULT 0,
                organization_id TEXT,
                connection_id TEXT,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            
);

CREATE TABLE IF NOT EXISTS a013_ym_order_items (
                id TEXT PRIMARY KEY NOT NULL,
                order_id TEXT NOT NULL,
                line_id TEXT NOT NULL,
                shop_sku TEXT NOT NULL,
                offer_id TEXT NOT NULL,
                name TEXT NOT NULL DEFAULT '',
                qty REAL NOT NULL DEFAULT 1.0,
                price_list REAL,
                discount_total REAL,
                price_effective REAL,
                amount_line REAL,
                price_plan REAL DEFAULT 0,
                marketplace_product_ref TEXT,
                nomenclature_ref TEXT,
                currency_code TEXT,
                buyer_price REAL,
                subsidies_json TEXT,
                status TEXT,
                FOREIGN KEY (order_id) REFERENCES a013_ym_order(id)
            
);

CREATE TABLE IF NOT EXISTS a014_ozon_transactions (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                operation_id INTEGER NOT NULL UNIQUE,
                posting_number TEXT NOT NULL,
                header_json TEXT NOT NULL,
                posting_json TEXT NOT NULL,
                items_json TEXT NOT NULL,
                services_json TEXT NOT NULL,
                source_meta_json TEXT NOT NULL,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            
);

CREATE TABLE IF NOT EXISTS a015_wb_orders (
    id TEXT PRIMARY KEY NOT NULL,
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL,
    comment TEXT,
    document_no TEXT NOT NULL,
    document_date TEXT,
    g_number TEXT,
    spp REAL,
    is_cancel INTEGER,
    cancel_date TEXT,
    header_json TEXT NOT NULL,
    line_json TEXT NOT NULL,
    state_json TEXT NOT NULL,
    warehouse_json TEXT NOT NULL,
    geography_json TEXT NOT NULL,
    source_meta_json TEXT NOT NULL,
    marketplace_product_ref TEXT,
    nomenclature_ref TEXT,
    base_nomenclature_ref TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS a016_ym_returns (
                id TEXT PRIMARY KEY NOT NULL,
                code TEXT NOT NULL DEFAULT '',
                description TEXT NOT NULL,
                comment TEXT,
                return_id INTEGER NOT NULL UNIQUE,
                order_id INTEGER NOT NULL,
                header_json TEXT NOT NULL,
                lines_json TEXT NOT NULL,
                state_json TEXT NOT NULL,
                source_meta_json TEXT NOT NULL,
                is_deleted INTEGER NOT NULL DEFAULT 0,
                is_posted INTEGER NOT NULL DEFAULT 0,
                created_at TEXT,
                updated_at TEXT,
                version INTEGER NOT NULL DEFAULT 0
            
);

CREATE TABLE IF NOT EXISTS p902_ozon_finance_realization_new (
                    posting_number TEXT NOT NULL,
                    sku TEXT NOT NULL,
                    document_type TEXT NOT NULL,
                    registrator_ref TEXT NOT NULL,
                    connection_mp_ref TEXT NOT NULL,
                    organization_ref TEXT NOT NULL,
                    posting_ref TEXT,
                    accrual_date TEXT NOT NULL,
                    operation_date TEXT,
                    delivery_date TEXT,
                    delivery_schema TEXT,
                    delivery_region TEXT,
                    delivery_city TEXT,
                    quantity REAL NOT NULL,
                    price REAL,
                    amount REAL NOT NULL,
                    commission_amount REAL,
                    commission_percent REAL,
                    services_amount REAL,
                    payout_amount REAL,
                    operation_type TEXT NOT NULL,
                    operation_type_name TEXT,
                    is_return INTEGER NOT NULL DEFAULT 0,
                    currency_code TEXT,
                    loaded_at_utc TEXT NOT NULL,
                    payload_version INTEGER NOT NULL DEFAULT 1,
                    extra TEXT,
                    PRIMARY KEY (posting_number, sku, operation_type)
                
);

CREATE TABLE IF NOT EXISTS p902_ozon_finance_realization (
                -- Composite Key (posting_number + sku + operation_type)
                posting_number TEXT NOT NULL,
                sku TEXT NOT NULL,

                -- Metadata
                document_type TEXT NOT NULL,
                registrator_ref TEXT NOT NULL,

                -- References
                connection_mp_ref TEXT NOT NULL,
                organization_ref TEXT NOT NULL,
                posting_ref TEXT,

                -- Р”Р°С‚С‹
                accrual_date TEXT NOT NULL,
                operation_date TEXT,
                delivery_date TEXT,

                -- РРЅС„РѕСЂРјР°С†РёСЏ Рѕ РґРѕСЃС‚Р°РІРєРµ
                delivery_schema TEXT,
                delivery_region TEXT,
                delivery_city TEXT,

                -- РљРѕР»РёС‡РµСЃС‚РІРѕ Рё СЃСѓРјРјС‹
                quantity REAL NOT NULL,
                price REAL,
                amount REAL NOT NULL,
                commission_amount REAL,
                commission_percent REAL,
                services_amount REAL,
                payout_amount REAL,

                -- РўРёРї РѕРїРµСЂР°С†РёРё
                operation_type TEXT NOT NULL,
                operation_type_name TEXT,
                is_return INTEGER NOT NULL DEFAULT 0,

                -- Р’Р°Р»СЋС‚Р°
                currency_code TEXT,

                -- РўРµС…РЅРёС‡РµСЃРєРёРµ РїРѕР»СЏ
                loaded_at_utc TEXT NOT NULL,
                payload_version INTEGER NOT NULL DEFAULT 1,
                extra TEXT,

                PRIMARY KEY (posting_number, sku, operation_type)
            
);

CREATE TABLE IF NOT EXISTS p903_wb_finance_report (
                -- Composite Primary Key
                rr_dt TEXT NOT NULL,              -- Р”Р°С‚Р° СЃС‚СЂРѕРєРё С„РёРЅР°РЅСЃРѕРІРѕРіРѕ РѕС‚С‡С‘С‚Р°
                rrd_id INTEGER NOT NULL,          -- Р’РЅСѓС‚СЂРµРЅРЅРёР№ ID СЃС‚СЂРѕРєРё РѕС‚С‡РµС‚Р°

                -- Metadata
                connection_mp_ref TEXT NOT NULL,
                organization_ref TEXT NOT NULL,

                -- Main Fields (22 specified fields)
                acquiring_fee REAL,               -- РљРѕРјРёСЃСЃРёСЏ Р·Р° СЌРєРІР°Р№СЂРёРЅРі
                acquiring_percent REAL,           -- РџСЂРѕС†РµРЅС‚ РєРѕРјРёСЃСЃРёРё Р·Р° СЌРєРІР°Р№СЂРёРЅРі
                additional_payment REAL,          -- Р”РѕРїРѕР»РЅРёС‚РµР»СЊРЅС‹Рµ РІС‹РїР»Р°С‚С‹
                bonus_type_name TEXT,             -- РўРёРї Р±РѕРЅСѓСЃР° РёР»Рё С€С‚СЂР°С„Р°
                commission_percent REAL,          -- РџСЂРѕС†РµРЅС‚ РєРѕРјРёСЃСЃРёРё WB
                delivery_amount REAL,             -- РЎСѓРјРјР° Р·Р° РґРѕСЃС‚Р°РІРєСѓ РѕС‚ РїРѕРєСѓРїР°С‚РµР»СЏ
                delivery_rub REAL,                -- РЎС‚РѕРёРјРѕСЃС‚СЊ РґРѕСЃС‚Р°РІРєРё РґР»СЏ РїСЂРѕРґР°РІС†Р°
                nm_id INTEGER,                    -- РђСЂС‚РёРєСѓР» WB
                penalty REAL,                     -- РЁС‚СЂР°С„
                ppvz_vw REAL,                     -- РЈСЃР»СѓРіР° РІРѕР·РІСЂР°С‚Р° СЃСЂРµРґСЃС‚РІ
                ppvz_vw_nds REAL,                 -- РќР”РЎ РїРѕ СѓСЃР»СѓРіРµ РІРѕР·РІСЂР°С‚Р°
                ppvz_sales_commission REAL,       -- РљРѕРјРёСЃСЃРёСЏ WB Р·Р° РїСЂРѕРґР°Р¶Сѓ
                quantity INTEGER,                 -- РљРѕР»РёС‡РµСЃС‚РІРѕ С‚РѕРІР°СЂРѕРІ
                rebill_logistic_cost REAL,        -- Р Р°СЃС…РѕРґС‹ РЅР° Р»РѕРіРёСЃС‚РёРєСѓ
                retail_amount REAL,               -- РћР±С‰Р°СЏ СЃСѓРјРјР° РїСЂРѕРґР°Р¶Рё
                retail_price REAL,                -- Р РѕР·РЅРёС‡РЅР°СЏ С†РµРЅР° Р·Р° РµРґРёРЅРёС†Сѓ
                retail_price_withdisc_rub REAL,   -- Р¦РµРЅР° СЃ СѓС‡РµС‚РѕРј СЃРєРёРґРѕРє
                return_amount REAL,               -- РЎСѓРјРјР° РІРѕР·РІСЂР°С‚Р°
                sa_name TEXT,                     -- РђСЂС‚РёРєСѓР» РїСЂРѕРґР°РІС†Р°
                storage_fee REAL,                 -- РџР»Р°С‚Р° Р·Р° С…СЂР°РЅРµРЅРёРµ
                subject_name TEXT,                -- РљР°С‚РµРіРѕСЂРёСЏ С‚РѕРІР°СЂР°
                supplier_oper_name TEXT,          -- РўРёРї РѕРїРµСЂР°С†РёРё
                cashback_amount REAL,             -- РЎСѓРјРјР° РєСЌС€Р±СЌРєР°
                ppvz_for_pay REAL,                -- Рљ РїРµСЂРµС‡РёСЃР»РµРЅРёСЋ Р·Р° С‚РѕРІР°СЂ
                ppvz_kvw_prc REAL,                -- РџСЂРѕС†РµРЅС‚ РєРѕРјРёСЃСЃРёРё
                ppvz_kvw_prc_base REAL,           -- Р‘Р°Р·РѕРІС‹Р№ РїСЂРѕС†РµРЅС‚ РєРѕРјРёСЃСЃРёРё
                srv_dbs INTEGER,                  -- Р”РѕСЃС‚Р°РІРєР° СЃРёР»Р°РјРё РїСЂРѕРґР°РІС†Р° (0/1)

                -- Technical fields
                loaded_at_utc TEXT NOT NULL,
                payload_version INTEGER NOT NULL DEFAULT 1,
                extra TEXT,                       -- Full JSON from API

                PRIMARY KEY (rr_dt, rrd_id)
            
);

CREATE TABLE IF NOT EXISTS p904_sales_data (
                -- Technical fields
                registrator_ref TEXT NOT NULL,
                registrator_type TEXT NOT NULL DEFAULT '',
                
                -- Dimensions
                date TEXT NOT NULL,
                connection_mp_ref TEXT NOT NULL,
                nomenclature_ref TEXT NOT NULL,
                marketplace_product_ref TEXT NOT NULL,
                
                -- Sums
                customer_in REAL NOT NULL DEFAULT 0,
                customer_out REAL NOT NULL DEFAULT 0,
                coinvest_in REAL NOT NULL DEFAULT 0,
                commission_out REAL NOT NULL DEFAULT 0,
                acquiring_out REAL NOT NULL DEFAULT 0,
                penalty_out REAL NOT NULL DEFAULT 0,
                logistics_out REAL NOT NULL DEFAULT 0,
                seller_out REAL NOT NULL DEFAULT 0,
                price_full REAL NOT NULL DEFAULT 0,
                price_list REAL NOT NULL DEFAULT 0,
                price_return REAL NOT NULL DEFAULT 0,
                commission_percent REAL NOT NULL DEFAULT 0,
                coinvest_persent REAL NOT NULL DEFAULT 0,
                total REAL NOT NULL DEFAULT 0,
                
                -- Info fields
                document_no TEXT NOT NULL,
                article TEXT NOT NULL,
                posted_at TEXT NOT NULL,
                
                -- Primary Key
                id TEXT PRIMARY KEY NOT NULL
            
);

CREATE TABLE IF NOT EXISTS p905_wb_commission_history (
                id TEXT PRIMARY KEY NOT NULL,
                date TEXT NOT NULL,
                subject_id INTEGER NOT NULL,
                subject_name TEXT NOT NULL,
                parent_id INTEGER NOT NULL,
                parent_name TEXT NOT NULL,
                kgvp_booking REAL NOT NULL,
                kgvp_marketplace REAL NOT NULL,
                kgvp_pickup REAL NOT NULL,
                kgvp_supplier REAL NOT NULL,
                kgvp_supplier_express REAL NOT NULL,
                paid_storage_kgvp REAL NOT NULL,
                raw_json TEXT NOT NULL,
                loaded_at_utc TEXT NOT NULL,
                payload_version INTEGER NOT NULL DEFAULT 1
            
);

CREATE TABLE IF NOT EXISTS p906_nomenclature_prices (
                id TEXT PRIMARY KEY NOT NULL,
                period TEXT NOT NULL,
                nomenclature_ref TEXT NOT NULL,
                price REAL NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            
);

CREATE TABLE IF NOT EXISTS user_form_settings (
                form_key TEXT PRIMARY KEY NOT NULL,
                settings_json TEXT NOT NULL,
                updated_at TEXT NOT NULL
            
);

CREATE TABLE IF NOT EXISTS sys_dashboard_configs (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    data_source TEXT NOT NULL,
    config_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sys_tasks (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    description TEXT,
    task_type TEXT NOT NULL,
    schedule_cron TEXT,
    config_json TEXT,
    is_enabled INTEGER DEFAULT 1,
    last_run_at TEXT,
    next_run_at TEXT,
    last_run_status TEXT,
    last_run_log_file TEXT,
    created_at TEXT DEFAULT (datetime('now')),
    updated_at TEXT DEFAULT (datetime('now')),
    is_deleted INTEGER DEFAULT 0
);

CREATE TABLE IF NOT EXISTS a012_wb_sales_new (
    id TEXT PRIMARY KEY NOT NULL,
    code TEXT NOT NULL DEFAULT '',
    description TEXT NOT NULL,
    comment TEXT,
    document_no TEXT NOT NULL,
    sale_id TEXT NOT NULL UNIQUE,
    -- Denormalized fields from JSON for fast queries
    sale_date TEXT,
    organization_id TEXT,
    connection_id TEXT,
    supplier_article TEXT,
    nm_id INTEGER,
    barcode TEXT,
    product_name TEXT,
    qty REAL,
    amount_line REAL,
    total_price REAL,
    finished_price REAL,
    event_type TEXT,
    -- JSON storage (kept for backward compatibility and full data)
    header_json TEXT NOT NULL,
    line_json TEXT NOT NULL,
    state_json TEXT NOT NULL,
    warehouse_json TEXT,
    source_meta_json TEXT NOT NULL,
    marketplace_product_ref TEXT,
    nomenclature_ref TEXT,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS a017_llm_agent (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    comment TEXT,
    provider_type TEXT NOT NULL DEFAULT 'OpenAI',
    api_endpoint TEXT NOT NULL,
    api_key TEXT NOT NULL,
    model_name TEXT NOT NULL,
    temperature REAL NOT NULL DEFAULT 0.7,
    max_tokens INTEGER NOT NULL DEFAULT 4096,
    system_prompt TEXT,
    is_primary INTEGER NOT NULL DEFAULT 0,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    available_models TEXT
);

CREATE TABLE IF NOT EXISTS a018_llm_chat (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    comment TEXT,
    agent_id TEXT NOT NULL,
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    model_name TEXT NOT NULL DEFAULT 'gpt-4o',
    FOREIGN KEY (agent_id) REFERENCES a017_llm_agent(id)
);

CREATE TABLE IF NOT EXISTS a018_llm_chat_message (
    id TEXT PRIMARY KEY,
    chat_id TEXT NOT NULL,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    tokens_used INTEGER,
    created_at TEXT NOT NULL,
    model_name TEXT,
    confidence REAL,
    artifact_id TEXT,
    artifact_action TEXT,
    duration_ms INTEGER,
    FOREIGN KEY (chat_id) REFERENCES a018_llm_chat(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS a018_llm_chat_attachment (
    id TEXT PRIMARY KEY NOT NULL,
    message_id TEXT NOT NULL,
    filename TEXT NOT NULL,
    filepath TEXT NOT NULL,
    content_type TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (message_id) REFERENCES a018_llm_chat_message(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS a019_llm_artifact (
    id TEXT PRIMARY KEY,
    code TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL,
    comment TEXT,
    
    -- Связи
    chat_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    
    -- Метаданные
    artifact_type TEXT NOT NULL DEFAULT 'sql_query',
    status TEXT NOT NULL DEFAULT 'active',
    
    -- SQL контент
    sql_query TEXT NOT NULL,
    query_params TEXT,
    visualization_config TEXT,
    
    -- Статистика выполнения
    last_executed_at TEXT,
    execution_count INTEGER NOT NULL DEFAULT 0,
    
    -- Стандартные поля
    is_deleted INTEGER NOT NULL DEFAULT 0,
    is_posted INTEGER NOT NULL DEFAULT 0,
    created_at TEXT,
    updated_at TEXT,
    version INTEGER NOT NULL DEFAULT 1,
    
    FOREIGN KEY (chat_id) REFERENCES a018_llm_chat(id),
    FOREIGN KEY (agent_id) REFERENCES a017_llm_agent(id)
);

CREATE TABLE IF NOT EXISTS sys_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS sys_users (
    id TEXT PRIMARY KEY,
    username TEXT UNIQUE NOT NULL,
    email TEXT UNIQUE,
    password_hash TEXT NOT NULL,
    full_name TEXT,
    is_active INTEGER DEFAULT 1 NOT NULL,
    is_admin INTEGER DEFAULT 0 NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    last_login_at TEXT,
    created_by TEXT,
    FOREIGN KEY (created_by) REFERENCES sys_users(id)
);

CREATE TABLE IF NOT EXISTS sys_refresh_tokens (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL,
    revoked_at TEXT,
    ip_address TEXT,
    user_agent TEXT,
    FOREIGN KEY (user_id) REFERENCES sys_users(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS sys_audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id TEXT,
    action TEXT NOT NULL,
    entity_type TEXT,
    entity_id TEXT,
    details TEXT,
    ip_address TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES sys_users(id)
);

-- Indexes

CREATE UNIQUE INDEX IF NOT EXISTS idx_a008_sales_unique ON a008_marketplace_sales (connection_id, product_id, accrual_date, operation_type);

CREATE UNIQUE INDEX IF NOT EXISTS idx_a009_returns_unique ON a009_ozon_returns (connection_id, return_id, sku);

CREATE INDEX IF NOT EXISTS idx_raw_storage_lookup ON document_raw_storage (marketplace, document_type, document_no);

CREATE INDEX IF NOT EXISTS idx_sales_register_sale_date ON p900_sales_register (sale_date);

CREATE INDEX IF NOT EXISTS idx_sales_register_event_time ON p900_sales_register (event_time_source);

CREATE INDEX IF NOT EXISTS idx_sales_register_connection_mp ON p900_sales_register (connection_mp_ref);

CREATE INDEX IF NOT EXISTS idx_sales_register_organization ON p900_sales_register (organization_ref);

CREATE INDEX IF NOT EXISTS idx_sales_register_product ON p900_sales_register (marketplace_product_ref);

CREATE INDEX IF NOT EXISTS idx_sales_register_seller_sku ON p900_sales_register (seller_sku);

CREATE INDEX IF NOT EXISTS idx_sales_register_mp_item_id ON p900_sales_register (mp_item_id);

CREATE INDEX IF NOT EXISTS idx_sales_register_status_norm ON p900_sales_register (status_norm);

CREATE INDEX IF NOT EXISTS idx_barcodes_nomenclature_ref ON p901_nomenclature_barcodes (nomenclature_ref);

CREATE INDEX IF NOT EXISTS idx_barcodes_article ON p901_nomenclature_barcodes (article);

CREATE INDEX IF NOT EXISTS idx_barcodes_is_active ON p901_nomenclature_barcodes (is_active);

CREATE INDEX IF NOT EXISTS idx_barcodes_source ON p901_nomenclature_barcodes (source);

CREATE INDEX IF NOT EXISTS idx_a012_sale_date ON a012_wb_sales(sale_date);

CREATE INDEX IF NOT EXISTS idx_a012_organization ON a012_wb_sales(organization_id);

CREATE INDEX IF NOT EXISTS idx_a012_document_no ON a012_wb_sales(document_no);

CREATE INDEX IF NOT EXISTS idx_a013_delivery_date ON a013_ym_order(delivery_date);

CREATE INDEX IF NOT EXISTS idx_a013_status_norm ON a013_ym_order(status_norm);

CREATE INDEX IF NOT EXISTS idx_a013_organization_id ON a013_ym_order(organization_id);

CREATE INDEX IF NOT EXISTS idx_a013_items_order_id ON a013_ym_order_items(order_id);

CREATE INDEX IF NOT EXISTS idx_a013_items_shop_sku ON a013_ym_order_items(shop_sku);

CREATE INDEX IF NOT EXISTS idx_a013_items_nomenclature_ref ON a013_ym_order_items(nomenclature_ref);

CREATE INDEX IF NOT EXISTS idx_a015_document_no ON a015_wb_orders(document_no);

CREATE INDEX IF NOT EXISTS idx_a015_g_number ON a015_wb_orders(g_number);

CREATE INDEX IF NOT EXISTS idx_a015_is_posted ON a015_wb_orders(is_posted);

CREATE INDEX IF NOT EXISTS idx_a016_order_id ON a016_ym_returns(order_id);

CREATE INDEX IF NOT EXISTS idx_p902_accrual_date ON p902_ozon_finance_realization (accrual_date);

CREATE INDEX IF NOT EXISTS idx_p902_posting_number ON p902_ozon_finance_realization (posting_number);

CREATE INDEX IF NOT EXISTS idx_p902_connection_mp_ref ON p902_ozon_finance_realization (connection_mp_ref);

CREATE INDEX IF NOT EXISTS idx_p902_posting_ref ON p902_ozon_finance_realization (posting_ref);

CREATE INDEX IF NOT EXISTS idx_p903_rr_dt ON p903_wb_finance_report (rr_dt);

CREATE INDEX IF NOT EXISTS idx_p903_nm_id ON p903_wb_finance_report (nm_id);

CREATE INDEX IF NOT EXISTS idx_p903_connection_mp_ref ON p903_wb_finance_report (connection_mp_ref);

CREATE INDEX IF NOT EXISTS idx_p903_organization_ref ON p903_wb_finance_report (organization_ref);

CREATE INDEX IF NOT EXISTS idx_p903_supplier_oper_name ON p903_wb_finance_report (supplier_oper_name);

CREATE INDEX IF NOT EXISTS idx_p903_rr_dt_org ON p903_wb_finance_report (rr_dt, organization_ref);

CREATE INDEX IF NOT EXISTS idx_p904_registrator ON p904_sales_data (registrator_ref);

CREATE UNIQUE INDEX IF NOT EXISTS idx_p905_date_subject ON p905_wb_commission_history(date, subject_id);

CREATE INDEX IF NOT EXISTS idx_p905_date ON p905_wb_commission_history(date);

CREATE INDEX IF NOT EXISTS idx_p905_subject_id ON p905_wb_commission_history(subject_id);

CREATE UNIQUE INDEX IF NOT EXISTS idx_p906_period_nomenclature ON p906_nomenclature_prices(period, nomenclature_ref);

CREATE INDEX IF NOT EXISTS idx_p906_period ON p906_nomenclature_prices(period);

CREATE INDEX IF NOT EXISTS idx_p906_nomenclature_ref ON p906_nomenclature_prices(nomenclature_ref);

CREATE INDEX IF NOT EXISTS idx_dashboard_configs_data_source ON sys_dashboard_configs(data_source);

CREATE INDEX IF NOT EXISTS idx_dashboard_configs_updated_at ON sys_dashboard_configs(updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_a012_sale_id ON a012_wb_sales(sale_id);

CREATE INDEX IF NOT EXISTS idx_a015_is_cancel ON a015_wb_orders(is_cancel);

CREATE INDEX IF NOT EXISTS idx_a017_llm_agent_code ON a017_llm_agent(code);

CREATE INDEX IF NOT EXISTS idx_a017_llm_agent_provider_type ON a017_llm_agent(provider_type);

CREATE INDEX IF NOT EXISTS idx_a017_llm_agent_is_primary ON a017_llm_agent(is_primary);

CREATE INDEX IF NOT EXISTS idx_a017_llm_agent_is_deleted ON a017_llm_agent(is_deleted);

CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_code ON a018_llm_chat(code);

CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_agent_id ON a018_llm_chat(agent_id);

CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_is_deleted ON a018_llm_chat(is_deleted);

CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_created_at ON a018_llm_chat(created_at);

CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_message_chat_id ON a018_llm_chat_message(chat_id);

CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_message_created_at ON a018_llm_chat_message(created_at);

CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_message_role ON a018_llm_chat_message(role);

CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_model_name ON a018_llm_chat(model_name);

CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_message_model_name ON a018_llm_chat_message(model_name);

CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_message_artifact_id ON a018_llm_chat_message(artifact_id);

CREATE INDEX IF NOT EXISTS idx_a018_llm_chat_attachment_message_id ON a018_llm_chat_attachment(message_id);

CREATE INDEX IF NOT EXISTS idx_a019_artifact_code ON a019_llm_artifact(code);

CREATE INDEX IF NOT EXISTS idx_a019_artifact_chat_id ON a019_llm_artifact(chat_id);

CREATE INDEX IF NOT EXISTS idx_a019_artifact_agent_id ON a019_llm_artifact(agent_id);

CREATE INDEX IF NOT EXISTS idx_a019_artifact_type ON a019_llm_artifact(artifact_type);

CREATE INDEX IF NOT EXISTS idx_a019_artifact_status ON a019_llm_artifact(status);

CREATE INDEX IF NOT EXISTS idx_a019_artifact_is_deleted ON a019_llm_artifact(is_deleted);

CREATE INDEX IF NOT EXISTS idx_a019_artifact_created_at ON a019_llm_artifact(created_at);

CREATE INDEX IF NOT EXISTS idx_sys_users_username ON sys_users(username);

CREATE INDEX IF NOT EXISTS idx_sys_users_email ON sys_users(email);

CREATE INDEX IF NOT EXISTS idx_sys_users_active ON sys_users(is_active);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_user ON sys_refresh_tokens(user_id);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_expires ON sys_refresh_tokens(expires_at);

CREATE INDEX IF NOT EXISTS idx_refresh_tokens_hash ON sys_refresh_tokens(token_hash);

CREATE INDEX IF NOT EXISTS idx_audit_log_user ON sys_audit_log(user_id);

CREATE INDEX IF NOT EXISTS idx_audit_log_created ON sys_audit_log(created_at);

CREATE INDEX IF NOT EXISTS idx_audit_log_action ON sys_audit_log(action);

CREATE INDEX IF NOT EXISTS idx_dashboard_configs_data_source ON sys_dashboard_configs(data_source);

CREATE INDEX IF NOT EXISTS idx_dashboard_configs_updated_at ON sys_dashboard_configs(updated_at DESC);
