use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

// Details components for linked aggregates
use crate::domain::a004_nomenclature::ui::details::NomenclatureDetails;
use crate::domain::a007_marketplace_product::ui::details::MarketplaceProductDetails;
use crate::projections::p903_wb_finance_report::ui::details::WbFinanceReportDetail;
use contracts::projections::p903_wb_finance_report::dto::WbFinanceReportDto;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesDetailDto {
    pub id: String,
    pub code: String,
    pub description: String,
    pub header: HeaderDto,
    pub line: LineDto,
    pub state: StateDto,
    pub warehouse: WarehouseDto,
    pub source_meta: SourceMetaDto,
    pub metadata: MetadataDto,
    pub marketplace_product_ref: Option<String>,
    pub nomenclature_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderDto {
    pub document_no: String,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineDto {
    pub line_id: String,
    pub supplier_article: String,
    pub nm_id: i64,
    pub barcode: String,
    pub name: String,
    pub qty: f64,
    pub price_list: Option<f64>,
    pub discount_total: Option<f64>,
    pub price_effective: Option<f64>,
    pub amount_line: Option<f64>,
    pub currency_code: Option<String>,
    pub total_price: Option<f64>,
    pub payment_sale_amount: Option<f64>,
    pub discount_percent: Option<f64>,
    pub spp: Option<f64>,
    pub finished_price: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDto {
    pub event_type: String,
    pub status_norm: String,
    pub sale_dt: String,
    pub last_change_dt: Option<String>,
    pub is_supply: Option<bool>,
    pub is_realization: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarehouseDto {
    pub warehouse_name: Option<String>,
    pub warehouse_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMetaDto {
    pub raw_payload_ref: String,
    pub fetched_at: String,
    pub document_version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataDto {
    pub created_at: String,
    pub updated_at: String,
    pub is_deleted: bool,
    pub is_posted: bool,
    pub version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceProductInfo {
    pub description: String,
    pub article: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NomenclatureInfo {
    pub description: String,
    pub article: String,
}

// Finance Report Link structure
#[component]
pub fn WbSalesDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let (sale, set_sale) = signal::<Option<WbSalesDetailDto>>(None);
    let (raw_json_from_wb, set_raw_json_from_wb) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal::<Option<String>>(None);
    let (active_tab, set_active_tab) = signal("general");
    let (posting, set_posting) = signal(false);
    let (marketplace_product_info, set_marketplace_product_info) =
        signal::<Option<MarketplaceProductInfo>>(None);

    // Linked finance reports
    let (linked_finance_reports, set_linked_finance_reports) =
        signal::<Vec<WbFinanceReportDto>>(Vec::new());
    let (links_loading, set_links_loading) = signal(false);
    let (links_error, set_links_error) = signal(None::<String>);
    let (links_fetch_attempted, set_links_fetch_attempted) = signal(false);
    let (selected_finance_report, set_selected_finance_report) =
        signal::<Option<(String, i64)>>(None);
    let (nomenclature_info, set_nomenclature_info) = signal::<Option<NomenclatureInfo>>(None);
    let (selected_marketplace_product_id, set_selected_marketplace_product_id) =
        signal::<Option<String>>(None);
    let (selected_nomenclature_id, set_selected_nomenclature_id) = signal::<Option<String>>(None);

    // Projections
    let (projections, set_projections) = signal::<Option<serde_json::Value>>(None);
    let (projections_loading, set_projections_loading) = signal(false);

    let is_posted = Memo::new(move |_| sale.get().map(|s| s.metadata.is_posted).unwrap_or(false));

    // Сохраняем id в StoredValue для использования в обработчиках
    let stored_id = StoredValue::new(id.clone());

    // Получаем tabs_store для обновления названия вкладки
    let tabs_store =
        leptos::context::use_context::<crate::layout::global_context::AppGlobalContext>()
            .expect("AppGlobalContext not found");

    // Обновляем название вкладки когда данные загружены
    Effect::new(move || {
        if let Some(sale_data) = sale.get() {
            let tab_key = format!("a012_wb_sales_detail_{}", stored_id.get_value());
            let tab_title = format!("WB Sales {}", sale_data.header.document_no);
            tabs_store.update_tab_title(&tab_key, &tab_title);
        }
    });

    // Функция для загрузки связанных данных (marketplace_product и nomenclature)
    let load_related_data = move |data: &WbSalesDetailDto| {
        // Загружаем данные marketplace_product если есть
        if let Some(ref mp_ref) = data.marketplace_product_ref {
            let mp_ref_clone = mp_ref.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let url = format!(
                    "http://localhost:3000/api/marketplace_product/{}",
                    mp_ref_clone
                );
                if let Ok(response) = Request::get(&url).send().await {
                    if response.status() == 200 {
                        if let Ok(text) = response.text().await {
                            if let Ok(mp) = serde_json::from_str::<serde_json::Value>(&text) {
                                set_marketplace_product_info.set(Some(MarketplaceProductInfo {
                                    description: mp
                                        .get("description")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    article: mp
                                        .get("article")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                }));
                            }
                        }
                    }
                }
            });
        } else {
            set_marketplace_product_info.set(None);
        }

        // Загружаем данные nomenclature если есть
        if let Some(ref nom_ref) = data.nomenclature_ref {
            let nom_ref_clone = nom_ref.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let url = format!("http://localhost:3000/api/nomenclature/{}", nom_ref_clone);
                if let Ok(response) = Request::get(&url).send().await {
                    if response.status() == 200 {
                        if let Ok(text) = response.text().await {
                            if let Ok(nom) = serde_json::from_str::<serde_json::Value>(&text) {
                                set_nomenclature_info.set(Some(NomenclatureInfo {
                                    description: nom
                                        .get("description")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    article: nom
                                        .get("article")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                }));
                            }
                        }
                    }
                }
            });
        } else {
            set_nomenclature_info.set(None);
        }
    };

    // Загрузить детальные данные
    Effect::new(move || {
        let id = stored_id.get_value();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let url = format!("http://localhost:3000/api/a012/wb-sales/{}", id);

            match Request::get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                // Парсим структуру
                                match serde_json::from_str::<WbSalesDetailDto>(&text) {
                                    Ok(data) => {
                                        // Загружаем raw JSON от WB
                                        let raw_payload_ref =
                                            data.source_meta.raw_payload_ref.clone();
                                        let sale_id = data.id.clone();

                                        // Загружаем связанные данные
                                        load_related_data(&data);

                                        set_sale.set(Some(data));
                                        set_loading.set(false);

                                        // Асинхронная загрузка raw JSON
                                        wasm_bindgen_futures::spawn_local(async move {
                                            let raw_url = format!(
                                                "http://localhost:3000/api/a012/raw/{}",
                                                raw_payload_ref
                                            );
                                            match Request::get(&raw_url).send().await {
                                                Ok(resp) => {
                                                    if resp.status() == 200 {
                                                        if let Ok(text) = resp.text().await {
                                                            // Форматируем JSON
                                                            if let Ok(json_value) =
                                                                serde_json::from_str::<
                                                                    serde_json::Value,
                                                                >(
                                                                    &text
                                                                )
                                                            {
                                                                if let Ok(formatted) =
                                                                    serde_json::to_string_pretty(
                                                                        &json_value,
                                                                    )
                                                                {
                                                                    set_raw_json_from_wb
                                                                        .set(Some(formatted));
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    log!(
                                                        "Failed to load raw JSON from WB: {:?}",
                                                        e
                                                    );
                                                }
                                            }
                                        });

                                        // Асинхронная загрузка проекций
                                        wasm_bindgen_futures::spawn_local(async move {
                                            set_projections_loading.set(true);
                                            let projections_url = format!(
                                                "http://localhost:3000/api/a012/wb-sales/{}/projections",
                                                sale_id
                                            );
                                            match Request::get(&projections_url).send().await {
                                                Ok(resp) => {
                                                    if resp.status() == 200 {
                                                        if let Ok(text) = resp.text().await {
                                                            if let Ok(proj_data) =
                                                                serde_json::from_str::<
                                                                    serde_json::Value,
                                                                >(
                                                                    &text
                                                                )
                                                            {
                                                                set_projections
                                                                    .set(Some(proj_data));
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    log!("Failed to load projections: {:?}", e);
                                                }
                                            }
                                            set_projections_loading.set(false);
                                        });
                                    }
                                    Err(e) => {
                                        log!("Failed to parse sale: {:?}", e);
                                        set_error.set(Some(format!("Failed to parse: {}", e)));
                                        set_loading.set(false);
                                    }
                                }
                            }
                            Err(e) => {
                                log!("Failed to read response: {:?}", e);
                                set_error.set(Some(format!("Failed to read response: {}", e)));
                                set_loading.set(false);
                            }
                        }
                    } else {
                        set_error.set(Some(format!("Server error: {}", status)));
                        set_loading.set(false);
                    }
                }
                Err(e) => {
                    log!("Failed to fetch sale: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    });

    // Загрузка связанных финансовых отчетов при активации вкладки Links или Line
    Effect::new(move || {
        let tab = active_tab.get();
        if tab == "links" || tab == "line" {
            if let Some(sale_data) = sale.get() {
                let srid_val = sale_data.header.document_no.clone();
                // Проверяем флаг: загружаем только если еще не пытались загружать
                if !srid_val.is_empty() && !links_fetch_attempted.get() {
                    set_links_loading.set(true);
                    set_links_error.set(None);
                    set_links_fetch_attempted.set(true);

                    spawn_local(async move {
                        let url =
                            format!("/api/p903/finance-report/search-by-srid?srid={}", srid_val);
                        match Request::get(&url).send().await {
                            Ok(resp) => {
                                if resp.status() == 200 {
                                    match resp.json::<Vec<WbFinanceReportDto>>().await {
                                        Ok(reports) => {
                                            set_linked_finance_reports.set(reports);
                                            set_links_loading.set(false);
                                        }
                                        Err(e) => {
                                            log!("Failed to parse finance reports: {:?}", e);
                                            set_links_error
                                                .set(Some(format!("Parse error: {}", e)));
                                            set_links_loading.set(false);
                                        }
                                    }
                                } else {
                                    set_links_error
                                        .set(Some(format!("HTTP error: {}", resp.status())));
                                    set_links_loading.set(false);
                                }
                            }
                            Err(e) => {
                                log!("Failed to fetch finance reports: {:?}", e);
                                set_links_error.set(Some(format!("Fetch error: {}", e)));
                                set_links_loading.set(false);
                            }
                        }
                    });
                }
            }
        }
    });

    // Функция для форматирования даты
    let _format_date = |date_str: &str| -> String {
        if let Some(date_part) = date_str.split('T').next() {
            if let Some((year, rest)) = date_part.split_once('-') {
                if let Some((month, day)) = rest.split_once('-') {
                    return format!("{}.{}.{}", day, month, year);
                }
            }
        }
        date_str.to_string()
    };

    let format_datetime = |datetime_str: &str| -> String {
        if let Some((date_part, time_part)) = datetime_str.split_once('T') {
            if let Some((year, rest)) = date_part.split_once('-') {
                if let Some((month, day)) = rest.split_once('-') {
                    let time = time_part.split('.').next().unwrap_or(time_part);
                    return format!("{}.{}.{} {}", day, month, year, time);
                }
            }
        }
        datetime_str.to_string()
    };

    view! {
        <div class="detail-form">
            <div class="detail-form-header">
                <div class="detail-form-header-left">
                    <h2>
                        {move || {
                            sale.get()
                                .map(|d| format!("WB Sales #{}", d.header.document_no))
                                .unwrap_or_else(|| "Wildberries Sales Details".to_string())
                        }}
                    </h2>
                    <Show when=move || sale.get().is_some()>
                        {move || {
                            let posted = is_posted.get();
                            view! {
                                <div class=move || if posted { "status-badge status-badge-posted" } else { "status-badge status-badge-not-posted" }>
                                    <span class="status-badge-icon">{if posted { "✓" } else { "○" }}</span>
                                    <span>{if posted { "Проведен" } else { "Не проведен" }}</span>
                                </div>
                            }
                        }}
                    </Show>
                </div>
                <div class="detail-form-header-right">
                    <Show when=move || sale.get().is_some()>
                        <Show
                            when=move || !is_posted.get()
                            fallback=move || {
                                view! {
                                    <button
                                        class="btn btn-warning"
                                        on:click=move |_| {
                                            let doc_id = stored_id.get_value();
                                            set_posting.set(true);
                                            wasm_bindgen_futures::spawn_local(async move {
                                                let url = format!("http://localhost:3000/api/a012/wb-sales/{}/unpost", doc_id);
                                                match Request::post(&url).send().await {
                                                    Ok(response) => {
                                                        if response.status() == 200 {
                                                            log!("Document unposted successfully");
                                                            // Перезагрузить данные документа
                                                            let reload_url = format!("http://localhost:3000/api/a012/wb-sales/{}", doc_id);
                                                            if let Ok(resp) = Request::get(&reload_url).send().await {
                                                                if let Ok(text) = resp.text().await {
                                                                    if let Ok(data) = serde_json::from_str::<WbSalesDetailDto>(&text) {
                                                                        log!("Reloaded document, is_posted: {}", data.metadata.is_posted);
                                                                        load_related_data(&data);
                                                                        set_sale.set(Some(data));
                                                                    }
                                                                }
                                                            }
                                                            // Перезагрузить проекции
                                                            let projections_url = format!("http://localhost:3000/api/a012/wb-sales/{}/projections", doc_id);
                                                            if let Ok(resp) = Request::get(&projections_url).send().await {
                                                                if resp.status() == 200 {
                                                                    if let Ok(text) = resp.text().await {
                                                                        if let Ok(proj_data) = serde_json::from_str::<serde_json::Value>(&text) {
                                                                            set_projections.set(Some(proj_data));
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            log!("Failed to unpost: status {}", response.status());
                                                        }
                                                    }
                                                    Err(e) => {
                                                        log!("Error unposting: {:?}", e);
                                                    }
                                                }
                                                set_posting.set(false);
                                            });
                                        }
                                        prop:disabled=move || posting.get()
                                    >
                                        {move || if posting.get() { "Отмена..." } else { "✗ Отменить" }}
                                    </button>
                                }
                            }
                        >
                            {
                                view! {
                                    <button
                                        class="btn btn-success"
                                        on:click=move |_| {
                                            let doc_id = stored_id.get_value();
                                            set_posting.set(true);
                                            wasm_bindgen_futures::spawn_local(async move {
                                                let url = format!("http://localhost:3000/api/a012/wb-sales/{}/post", doc_id);
                                                match Request::post(&url).send().await {
                                                    Ok(response) => {
                                                        if response.status() == 200 {
                                                            log!("Document posted successfully");
                                                            // Перезагрузить данные документа
                                                            let reload_url = format!("http://localhost:3000/api/a012/wb-sales/{}", doc_id);
                                                            if let Ok(resp) = Request::get(&reload_url).send().await {
                                                                if let Ok(text) = resp.text().await {
                                                                    if let Ok(data) = serde_json::from_str::<WbSalesDetailDto>(&text) {
                                                                        log!("Reloaded document, is_posted: {}", data.metadata.is_posted);
                                                                        load_related_data(&data);
                                                                        set_sale.set(Some(data));
                                                                    }
                                                                }
                                                            }
                                                            // Перезагрузить проекции
                                                            let projections_url = format!("http://localhost:3000/api/a012/wb-sales/{}/projections", doc_id);
                                                            if let Ok(resp) = Request::get(&projections_url).send().await {
                                                                if resp.status() == 200 {
                                                                    if let Ok(text) = resp.text().await {
                                                                        if let Ok(proj_data) = serde_json::from_str::<serde_json::Value>(&text) {
                                                                            set_projections.set(Some(proj_data));
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        } else {
                                                            log!("Failed to post: status {}", response.status());
                                                        }
                                                    }
                                                    Err(e) => {
                                                        log!("Error posting: {:?}", e);
                                                    }
                                                }
                                                set_posting.set(false);
                                            });
                                        }
                                        prop:disabled=move || posting.get()
                                    >
                                        {move || if posting.get() { "Проведение..." } else { "✓ Провести" }}
                                    </button>
                                }
                            }
                        </Show>
                    </Show>
                    <button
                        class="btn btn-secondary"
                        on:click=move |_| on_close.run(())
                    >
                        "✕ Закрыть"
                    </button>
                </div>
            </div>

            <div class="detail-form-content">
                {move || {
                    if loading.get() {
                        view! {
                            <div style="text-align: center; padding: var(--space-2xl);">
                                <p style="font-size: var(--font-size-sm);">"Загрузка..."</p>
                            </div>
                        }.into_any()
                    } else if let Some(err) = error.get() {
                        view! {
                            <div style="padding: var(--space-lg); background: var(--color-error-bg); border: 1px solid var(--color-error-border); border-radius: var(--radius-sm); color: var(--color-error); margin: var(--space-lg); font-size: var(--font-size-sm);">
                                <strong>"Ошибка: "</strong>{err}
                            </div>
                        }.into_any()
                    } else if let Some(sale_data) = sale.get() {
                        view! {
                            <div>
                                <div class="detail-tabs">
                                    <button
                                        class="detail-tab"
                                        class:active=move || active_tab.get() == "general"
                                        on:click=move |_| set_active_tab.set("general")
                                    >
                                        "Общие данные"
                                    </button>
                                    <button
                                        class="detail-tab"
                                        class:active=move || active_tab.get() == "line"
                                        on:click=move |_| set_active_tab.set("line")
                                    >
                                        "Детали строки"
                                    </button>
                                    <button
                                        class="detail-tab"
                                        class:active=move || active_tab.get() == "json"
                                        on:click=move |_| set_active_tab.set("json")
                                    >
                                        "Raw JSON"
                                    </button>
                                    <button
                                        class="detail-tab"
                                        class:active=move || active_tab.get() == "links"
                                        on:click=move |_| set_active_tab.set("links")
                                    >
                                        "Связи"
                                    </button>
                                    <button
                                        class="detail-tab"
                                        class:active=move || active_tab.get() == "projections"
                                        on:click=move |_| set_active_tab.set("projections")
                                    >
                                        {move || {
                                            let count = projections.get().as_ref().map(|p| {
                                                let p900_len = p["p900_sales_register"].as_array().map(|a| a.len()).unwrap_or(0);
                                                let p904_len = p["p904_sales_data"].as_array().map(|a| a.len()).unwrap_or(0);
                                                p900_len + p904_len
                                            }).unwrap_or(0);
                                            format!("Проекции ({})", count)
                                        }}
                                    </button>
                                </div>

                                <div style="padding-top: var(--space-lg);">
                                    {move || {
                                let tab = active_tab.get();
                                match tab.as_ref() {
                                    "general" => {
                                        let conn_id = sale_data.header.connection_id.clone();
                                        let org_id = sale_data.header.organization_id.clone();
                                        let mp_id = sale_data.header.marketplace_id.clone();

                                        view! {
                                            <div class="general-info" style="max-width: 1400px;">
                                                // Блоки товара и номенклатуры в две колонки
                                                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-lg); margin-bottom: var(--space-lg);">
                                                    // Блок: Товар маркетплейса
                                                    <div style="padding: var(--space-md) var(--space-xl); background: #e3f2fd; border-radius: var(--radius-md); border-left: 3px solid var(--color-primary);">
                                                        <div style="font-weight: var(--font-weight-semibold); color: #1976d2; font-size: var(--font-size-xs); margin-bottom: var(--space-md);">"📦 Товар маркетплейса"</div>
                                                        {move || if let Some(ref mp_info) = marketplace_product_info.get() {
                                                            view! {
                                                                <div style="font-size: var(--font-size-xs); line-height: var(--line-height-normal);">
                                                                    <div
                                                                        style="color: #0d47a1; font-weight: var(--font-weight-semibold); cursor: pointer; text-decoration: underline;"
                                                                        on:click={
                                                                            let sale_signal = sale.clone();
                                                                            move |_| {
                                                                                if let Some(s) = sale_signal.get() {
                                                                                    if let Some(ref mp_ref) = s.marketplace_product_ref {
                                                                                        set_selected_marketplace_product_id.set(Some(mp_ref.clone()));
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    >
                                                                        {mp_info.description.clone()}
                                                                    </div>
                                                                    <div style="color: var(--color-text-muted); margin-top: 3px;">
                                                                        <span style="font-weight: var(--font-weight-semibold);">"Артикул: "</span>
                                                                        <code style="background: #bbdefb; padding: 2px var(--space-sm); border-radius: var(--radius-sm); font-size: 11px;">{mp_info.article.clone()}</code>
                                                                    </div>
                                                                </div>
                                                            }.into_any()
                                                        } else {
                                                            view! {
                                                                <div style="color: #999; font-style: italic; font-size: var(--font-size-xs);">"Не привязан"</div>
                                                            }.into_any()
                                                        }}
                                                    </div>

                                                    // Блок: Номенклатура 1С
                                                    <div style="padding: var(--space-md) var(--space-xl); background: #e8f5e9; border-radius: var(--radius-md); border-left: 3px solid var(--color-success);">
                                                        <div style="font-weight: var(--font-weight-semibold); color: #2e7d32; font-size: var(--font-size-xs); margin-bottom: var(--space-md);">"📋 Номенклатура 1С"</div>
                                                        {move || if let Some(ref nom_info) = nomenclature_info.get() {
                                                            view! {
                                                                <div style="font-size: var(--font-size-xs); line-height: var(--line-height-normal);">
                                                                    <div
                                                                        style="color: #1b5e20; font-weight: var(--font-weight-semibold); cursor: pointer; text-decoration: underline;"
                                                                        on:click={
                                                                            let sale_signal = sale.clone();
                                                                            move |_| {
                                                                                if let Some(s) = sale_signal.get() {
                                                                                    if let Some(ref nom_ref) = s.nomenclature_ref {
                                                                                        set_selected_nomenclature_id.set(Some(nom_ref.clone()));
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    >
                                                                        {nom_info.description.clone()}
                                                                    </div>
                                                                    <div style="color: var(--color-text-muted); margin-top: 3px;">
                                                                        <span style="font-weight: var(--font-weight-semibold);">"Артикул: "</span>
                                                                        <code style="background: #c8e6c9; padding: 2px var(--space-sm); border-radius: var(--radius-sm); font-size: 11px;">{nom_info.article.clone()}</code>
                                                                    </div>
                                                                </div>
                                                            }.into_any()
                                                        } else {
                                                            view! {
                                                                <div style="color: #999; font-style: italic; font-size: var(--font-size-xs);">"Не привязана"</div>
                                                            }.into_any()
                                                        }}
                                                    </div>
                                                </div>

                                                // 2 колонки для основных данных
                                                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-lg); margin-bottom: var(--space-lg);">
                                                    // Левая колонка
                                                    <div style="background: var(--color-bg-body); padding: var(--space-xl); border-radius: var(--radius-md); border: 1px solid var(--color-border-lighter);">
                                                        <div style="display: grid; grid-template-columns: 120px 1fr; gap: var(--space-md); align-items: start; font-size: var(--font-size-sm);">
                                                            <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Document №:"</div>
                                                            <div>{sale_data.header.document_no.clone()}</div>

                                                            <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Code:"</div>
                                                            <div>{sale_data.code.clone()}</div>

                                                            <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Description:"</div>
                                                            <div>{sale_data.description.clone()}</div>

                                                            <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Event Type:"</div>
                                                            <div>
                                                                <span style="padding: 2px var(--space-md); background: #e3f2fd; color: #1976d2; border-radius: var(--radius-sm); font-weight: var(--font-weight-medium); font-size: var(--font-size-xs);">
                                                                    {sale_data.state.event_type.clone()}
                                                                </span>
                                                            </div>

                                                            <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Status:"</div>
                                                            <div>
                                                                <span style="padding: 2px var(--space-md); background: #e8f5e9; color: #2e7d32; border-radius: var(--radius-sm); font-weight: var(--font-weight-medium); font-size: var(--font-size-xs);">
                                                                    {sale_data.state.status_norm.clone()}
                                                                </span>
                                                            </div>

                                                            <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Sale Date:"</div>
                                                            <div>{format_datetime(&sale_data.state.sale_dt)}</div>

                                                            <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Last Change:"</div>
                                                            <div>{sale_data.state.last_change_dt.as_ref().map(|d| format_datetime(d)).unwrap_or("—".to_string())}</div>
                                                        </div>
                                                    </div>

                                                    // Правая колонка
                                                    <div style="background: var(--color-bg-body); padding: var(--space-xl); border-radius: var(--radius-md); border: 1px solid var(--color-border-lighter);">
                                                        <div style="display: grid; grid-template-columns: 120px 1fr; gap: var(--space-md); align-items: start; font-size: var(--font-size-sm);">
                                                            <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Warehouse Name:"</div>
                                                            <div>{sale_data.warehouse.warehouse_name.clone().unwrap_or("—".to_string())}</div>

                                                            <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Warehouse Type:"</div>
                                                            <div>{sale_data.warehouse.warehouse_type.clone().unwrap_or("—".to_string())}</div>

                                                            <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Is Supply:"</div>
                                                            <div>
                                                                {match sale_data.state.is_supply {
                                                                    Some(true) => view! {
                                                                        <span style="padding: 2px var(--space-md); background: #e3f2fd; color: #1976d2; border-radius: var(--radius-sm); font-weight: var(--font-weight-medium); font-size: var(--font-size-xs);">
                                                                            "Yes"
                                                                        </span>
                                                                    }.into_any(),
                                                                    Some(false) => view! {
                                                                        <span style="padding: 2px var(--space-md); background: #ffebee; color: #c62828; border-radius: var(--radius-sm); font-weight: var(--font-weight-medium); font-size: var(--font-size-xs);">
                                                                            "No"
                                                                        </span>
                                                                    }.into_any(),
                                                                    None => view! { <span>"—"</span> }.into_any(),
                                                                }}
                                                            </div>

                                                            <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Is Realization:"</div>
                                                            <div>
                                                                {match sale_data.state.is_realization {
                                                                    Some(true) => view! {
                                                                        <span style="padding: 2px var(--space-md); background: #e3f2fd; color: #1976d2; border-radius: var(--radius-sm); font-weight: var(--font-weight-medium); font-size: var(--font-size-xs);">
                                                                            "Yes"
                                                                        </span>
                                                                    }.into_any(),
                                                                    Some(false) => view! {
                                                                        <span style="padding: 2px var(--space-md); background: #ffebee; color: #c62828; border-radius: var(--radius-sm); font-weight: var(--font-weight-medium); font-size: var(--font-size-xs);">
                                                                            "No"
                                                                        </span>
                                                                    }.into_any(),
                                                                    None => view! { <span>"—"</span> }.into_any(),
                                                                }}
                                                            </div>

                                                            <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Created At:"</div>
                                                            <div>{format_datetime(&sale_data.metadata.created_at)}</div>

                                                            <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Updated At:"</div>
                                                            <div>{format_datetime(&sale_data.metadata.updated_at)}</div>

                                                            <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Version:"</div>
                                                            <div>{sale_data.metadata.version}</div>
                                                        </div>
                                                    </div>
                                                </div>

                                                // UUID section at bottom
                                                <div style="background: #fafafa; padding: var(--space-xl); border-radius: var(--radius-md); border: 1px solid var(--color-border-lighter);">
                                                    <h4 style="margin: 0 0 var(--space-md) 0; color: var(--color-text-muted); font-size: var(--font-size-sm); font-weight: var(--font-weight-semibold);">"Technical IDs"</h4>
                                                    <div style="display: grid; grid-template-columns: 180px 1fr; gap: var(--space-md); align-items: center; font-size: var(--font-size-sm);">

                                                        <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Connection ID:"</div>
                                                        <div style="display: flex; align-items: center; gap: var(--space-md);">
                                                            <code style="font-size: var(--font-size-xs); color: var(--color-text-muted);" title=conn_id.clone()>{conn_id.clone()}</code>
                                                            <button
                                                                on:click=move |_| {
                                                                    let uuid_copy = conn_id.clone();
                                                                    wasm_bindgen_futures::spawn_local(async move {
                                                                        if let Some(window) = web_sys::window() {
                                                                            let nav = window.navigator().clipboard();
                                                                            let _ = nav.write_text(&uuid_copy);
                                                                        }
                                                                    });
                                                                }
                                                                style="padding: 2px var(--space-md); font-size: 11px; border: 1px solid var(--color-border-light); background: var(--color-bg-body); border-radius: var(--radius-sm); cursor: pointer;"
                                                                title="Copy to clipboard"
                                                            >
                                                                "📋"
                                                            </button>
                                                        </div>

                                                        <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Organization ID:"</div>
                                                        <div style="display: flex; align-items: center; gap: var(--space-md);">
                                                            <code style="font-size: var(--font-size-xs); color: var(--color-text-muted);" title=org_id.clone()>{org_id.clone()}</code>
                                                            <button
                                                                on:click=move |_| {
                                                                    let uuid_copy = org_id.clone();
                                                                    wasm_bindgen_futures::spawn_local(async move {
                                                                        if let Some(window) = web_sys::window() {
                                                                            let nav = window.navigator().clipboard();
                                                                            let _ = nav.write_text(&uuid_copy);
                                                                        }
                                                                    });
                                                                }
                                                                style="padding: 2px var(--space-md); font-size: 11px; border: 1px solid var(--color-border-light); background: var(--color-bg-body); border-radius: var(--radius-sm); cursor: pointer;"
                                                                title="Copy to clipboard"
                                                            >
                                                                "📋"
                                                            </button>
                                                        </div>

                                                        <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Marketplace ID:"</div>
                                                        <div style="display: flex; align-items: center; gap: var(--space-md);">
                                                            <code style="font-size: var(--font-size-xs); color: var(--color-text-muted);" title=mp_id.clone()>{mp_id.clone()}</code>
                                                            <button
                                                                on:click=move |_| {
                                                                    let uuid_copy = mp_id.clone();
                                                                    wasm_bindgen_futures::spawn_local(async move {
                                                                        if let Some(window) = web_sys::window() {
                                                                            let nav = window.navigator().clipboard();
                                                                            let _ = nav.write_text(&uuid_copy);
                                                                        }
                                                                    });
                                                                }
                                                                style="padding: 2px var(--space-md); font-size: 11px; border: 1px solid var(--color-border-light); background: var(--color-bg-body); border-radius: var(--radius-sm); cursor: pointer;"
                                                                title="Copy to clipboard"
                                                            >
                                                                "📋"
                                                            </button>
                                                        </div>

                                                        <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Marketplace Product ID:"</div>
                                                        <div style="display: flex; align-items: center; gap: var(--space-md);">
                                                            {if let Some(ref mp_ref) = sale_data.marketplace_product_ref {
                                                                let mp_ref_copy = mp_ref.clone();
                                                                view! {
                                                                    <>
                                                                        <code style="font-size: var(--font-size-xs); color: var(--color-text-muted);" title=mp_ref.clone()>{mp_ref.clone()}</code>
                                                                        <button
                                                                            on:click=move |_| {
                                                                                let uuid_copy = mp_ref_copy.clone();
                                                                                wasm_bindgen_futures::spawn_local(async move {
                                                                                    if let Some(window) = web_sys::window() {
                                                                                        let nav = window.navigator().clipboard();
                                                                                        let _ = nav.write_text(&uuid_copy);
                                                                                    }
                                                                                });
                                                                            }
                                                                            style="padding: 2px var(--space-md); font-size: 11px; border: 1px solid var(--color-border-light); background: var(--color-bg-body); border-radius: var(--radius-sm); cursor: pointer;"
                                                                            title="Copy to clipboard"
                                                                        >
                                                                            "📋"
                                                                        </button>
                                                                    </>
                                                                }.into_any()
                                                            } else {
                                                                view! { <span style="color: #999;">"—"</span> }.into_any()
                                                            }}
                                                        </div>

                                                        <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Nomenclature ID:"</div>
                                                        <div style="display: flex; align-items: center; gap: var(--space-md);">
                                                            {if let Some(ref nom_ref) = sale_data.nomenclature_ref {
                                                                let nom_ref_copy = nom_ref.clone();
                                                                view! {
                                                                    <>
                                                                        <code style="font-size: var(--font-size-xs); color: var(--color-text-muted);" title=nom_ref.clone()>{nom_ref.clone()}</code>
                                                                        <button
                                                                            on:click=move |_| {
                                                                                let uuid_copy = nom_ref_copy.clone();
                                                                                wasm_bindgen_futures::spawn_local(async move {
                                                                                    if let Some(window) = web_sys::window() {
                                                                                        let nav = window.navigator().clipboard();
                                                                                        let _ = nav.write_text(&uuid_copy);
                                                                                    }
                                                                                });
                                                                            }
                                                                            style="padding: 2px var(--space-md); font-size: 11px; border: 1px solid var(--color-border-light); background: var(--color-bg-body); border-radius: var(--radius-sm); cursor: pointer;"
                                                                            title="Copy to clipboard"
                                                                        >
                                                                            "📋"
                                                                        </button>
                                                                    </>
                                                                }.into_any()
                                                            } else {
                                                                view! { <span style="color: #999;">"—"</span> }.into_any()
                                                            }}
                                                        </div>
                                                    </div>
                                                </div>
                                            </div>
                                        }.into_any()
                                    },
                                    "line" => {
                                        let line = &sale_data.line;
                                        view! {
                                            <div class="line-info">
                                                <div style="margin-bottom: var(--space-lg);">
                                                    <div style="display: grid; grid-template-columns: 150px 1fr; gap: var(--space-md) var(--space-xl); align-items: center; margin-bottom: var(--space-lg); font-size: var(--font-size-sm);">
                                                        <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Line ID:"</div>
                                                        <div>{line.line_id.clone()}</div>

                                                        <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Артикул продавца:"</div>
                                                        <div style="font-weight: var(--font-weight-medium);">{line.supplier_article.clone()}</div>

                                                        <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"NM ID:"</div>
                                                        <div>{line.nm_id}</div>

                                                        <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Штрихкод:"</div>
                                                        <div>{line.barcode.clone()}</div>

                                                        <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Название:"</div>
                                                        <div style="font-weight: var(--font-weight-medium);">{line.name.clone()}</div>

                                                        <div style="font-weight: var(--font-weight-semibold); color: var(--color-text-secondary);">"Количество:"</div>
                                                        <div>
                                                            {format!("{:.0}", line.qty)}
                                                        </div>
                                                    </div>

                                                    <h3 style="margin: var(--space-md) 0 var(--space-xs) 0; font-size: var(--font-size-sm); color: var(--color-text-secondary); font-weight: var(--font-weight-semibold);">"Суммы и проценты"</h3>
                                                    <table style="width: 50%; border-collapse: collapse; font-family: var(--font-family-base); font-size: var(--font-size-sm);">
                                                        <thead>
                                                            <tr style="background: var(--color-hover-bg);">
                                                                <th style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: left; width: 40%;">"Наименование"</th>
                                                                <th style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: left; width: 25%;">"Поле"</th>
                                                                <th style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: right; width: 20%;">"Значение"</th>
                                                                <th style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: left; width: 15%;">"Ед."</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody>
                                                            <tr>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);">"Полная цена"</td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);"><code>"total_price"</code></td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: right;">{line.total_price.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);">"rub"</td>
                                                            </tr>
                                                            <tr>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);">"Процент скидки"</td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);"><code>"discount_percent"</code></td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: right;">{line.discount_percent.map(|d| format!("{:.1}", d)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);">"%"</td>
                                                            </tr>
                                                            <tr>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);">"Цена без скидок"</td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);"><code>"price_list"</code></td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: right;">{line.price_list.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);">"rub"</td>
                                                            </tr>
                                                            <tr>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);">"Сумма скидок"</td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);"><code>"discount_total"</code></td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: right;">{line.discount_total.map(|d| format!("{:.2}", d)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);">"rub"</td>
                                                            </tr>
                                                            <tr>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);">"Цена после скидок"</td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);"><code>"price_effective"</code></td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: right;">{line.price_effective.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);">"rub"</td>
                                                            </tr>
                                                            <tr>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);">"СПП"</td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);"><code>"spp"</code></td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: right;">{line.spp.map(|s| format!("{:.1}", s)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);">"%"</td>
                                                            </tr>
                                                            <tr>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);">"Итоговая цена"</td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);"><code>"finished_price"</code></td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: right;">{line.finished_price.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);">"rub"</td>
                                                            </tr>
                                                            <tr>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);">"Сумма платежа"</td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);"><code>"payment_sale_amount"</code></td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: right;">{line.payment_sale_amount.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);">"rub"</td>
                                                            </tr>
                                                            <tr style="background:rgb(138, 227, 254);">
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); font-weight: var(--font-weight-semibold);">"К выплате"</td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);"><code>"amount_line"</code></td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: right; font-weight: var(--font-weight-semibold);">{line.amount_line.map(|a| format!("{:.2}", a)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);">"rub"</td>
                                                            </tr>
                                                        </tbody>
                                                    </table>

                                                    // Блок Финансовые детали
                                                    {
                                                        let finance_reports = linked_finance_reports.get();
                                                        if !finance_reports.is_empty() {
                                                            let mut rows: Vec<(usize, String, String, String)> = Vec::new();

                                                            for (idx, report) in finance_reports.iter().enumerate() {
                                                                let row_num = idx + 1;

                                                                // rr_dt
                                                                rows.push((row_num, "Дата операции".to_string(), "rr_dt".to_string(), report.rr_dt.clone()));

                                                                // ppvz_vw
                                                                if let Some(val) = report.ppvz_vw {
                                                                    rows.push((row_num, "Вознаграждение Вайлдберриз (ВВ), без НДС".to_string(), "ppvz_vw".to_string(), format!("{:.2}", val)));
                                                                }

                                                                // ppvz_vw_nds
                                                                if let Some(val) = report.ppvz_vw_nds {
                                                                    rows.push((row_num, "НДС с вознаграждения Вайлдберриз".to_string(), "ppvz_vw_nds".to_string(), format!("{:.2}", val)));
                                                                }

                                                                // retail_amount
                                                                if let Some(val) = report.retail_amount {
                                                                    rows.push((row_num, "Вайлдберриз реализовал Товар (Пр)".to_string(), "retail_amount".to_string(), format!("{:.2}", val)));
                                                                }

                                                                // ppvz_for_pay
                                                                if let Some(val) = report.ppvz_for_pay {
                                                                    rows.push((row_num, "К перечислению продавцу за реализованный товар".to_string(), "ppvz_for_pay".to_string(), format!("{:.2}", val)));
                                                                }

                                                                // commission_percent
                                                                if let Some(val) = report.commission_percent {
                                                                    rows.push((row_num, "Размер кВВ, %".to_string(), "commission_percent".to_string(), format!("{:.2}", val)));
                                                                }

                                                                // retail_price
                                                                if let Some(val) = report.retail_price {
                                                                    rows.push((row_num, "Цена розничная".to_string(), "retail_price".to_string(), format!("{:.2}", val)));
                                                                }

                                                                // retail_price_withdisc_rub
                                                                if let Some(val) = report.retail_price_withdisc_rub {
                                                                    rows.push((row_num, "Цена розничная с учётом согласованной скидки".to_string(), "retail_price_withdisc_rub".to_string(), format!("{:.2}", val)));
                                                                }

                                                                // acquiring_fee
                                                                if let Some(val) = report.acquiring_fee {
                                                                    rows.push((row_num, "Эквайринг/Комиссии за организацию платежей".to_string(), "acquiring_fee".to_string(), format!("{:.2}", val)));
                                                                }
                                                            }

                                                            view! {
                                                                <div>
                                                                    <h3 style="margin: var(--space-md) 0 var(--space-xs) 0; font-size: var(--font-size-sm); color: var(--color-text-secondary); font-weight: var(--font-weight-semibold);">"Финансовые детали"</h3>
                                                                    <table style="width: 70%; border-collapse: collapse; font-family: var(--font-family-base); font-size: var(--font-size-sm);">
                                                                        <thead>
                                                                            <tr style="background: var(--color-hover-bg);">
                                                                                <th style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: center; width: 8%;">"#"</th>
                                                                                <th style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: left; width: 47%;">"Наименование"</th>
                                                                                <th style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: left; width: 25%;">"Поле"</th>
                                                                                <th style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: right; width: 20%;">"Значение"</th>
                                                                            </tr>
                                                                        </thead>
                                                                        <tbody>
                                                                            {rows.into_iter().map(|(num, name, field, value)| {
                                                                                view! {
                                                                                    <tr>
                                                                                        <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: center;">{num}</td>
                                                                                        <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);">{name}</td>
                                                                                        <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm);"><code>{field}</code></td>
                                                                                        <td style="border: 1px solid var(--color-border-light); padding: 3px var(--space-sm); text-align: right;">{value}</td>
                                                                                    </tr>
                                                                                }
                                                                            }).collect_view()}
                                                                        </tbody>
                                                                    </table>
                                                                </div>
                                                            }.into_any()
                                                        } else {
                                                            view! { <div></div> }.into_any()
                                                        }
                                                    }
                                                </div>
                                            </div>
                                        }.into_any()
                                    },
                                    "json" => view! {
                                        <div class="json-info">
                                            <div style="margin-bottom: var(--space-md); font-size: var(--font-size-sm);">
                                                <strong>"Raw JSON from WB API:"</strong>
                                            </div>
                                            {move || {
                                                if let Some(json) = raw_json_from_wb.get() {
                                                    view! {
                                                        <pre style="background: var(--color-hover-bg); padding: var(--space-xl); border-radius: var(--radius-sm); overflow-x: auto; font-size: var(--font-size-sm); line-height: var(--line-height-normal);">
                                                            {json}
                                                        </pre>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div style="padding: var(--space-4xl); text-align: center; color: #999; font-size: var(--font-size-sm);">
                                                            "Loading raw JSON from WB..."
                                                        </div>
                                                    }.into_any()
                                                }
                                            }}
                                        </div>
                                    }.into_any(),
                                    "links" => {
                                        if links_loading.get() {
                                            view! { <p>"Loading linked finance reports..."</p> }.into_any()
                                        } else if let Some(err) = links_error.get() {
                                            view! { <p style="color: red;">"Error loading links: " {err}</p> }.into_any()
                                        } else {
                                            let reports = linked_finance_reports.get();
                                            if reports.is_empty() {
                                                view! { <p>"No linked finance reports found for this SRID."</p> }.into_any()
                                            } else {
                                                let total_ppvz_vw: f64 = reports.iter().filter_map(|r| r.ppvz_vw).sum();
                                                let total_ppvz_vw_nds: f64 = reports.iter().filter_map(|r| r.ppvz_vw_nds).sum();
                                                let total_retail: f64 = reports.iter().filter_map(|r| r.retail_amount).sum();
                                                let total_ppvz_for_pay: f64 = reports.iter().filter_map(|r| r.ppvz_for_pay).sum();
                                                let total_acquiring: f64 = reports.iter().filter_map(|r| r.acquiring_fee).sum();

                                                view! {
                                                    <div>
                                                        <div style="padding: 8px 12px; margin-bottom: 8px; background: var(--secondary-bg-color); border: 1px solid var(--border-color); border-radius: 4px; font-weight: bold; display: flex; gap: 24px;">
                                                            <span>"Found: " {reports.len()} " records"</span>
                                                            <span>"PPVZ VW: " {format!("{:.2}", total_ppvz_vw)}</span>
                                                            <span>"PPVZ VW NDS: " {format!("{:.2}", total_ppvz_vw_nds)}</span>
                                                            <span>"Retail: " {format!("{:.2}", total_retail)}</span>
                                                            <span>"For Pay: " {format!("{:.2}", total_ppvz_for_pay)}</span>
                                                            <span>"Acquiring: " {format!("{:.2}", total_acquiring)}</span>
                                                        </div>

                                                        <div style="max-height: calc(100vh - 280px); overflow-y: auto; border: 1px solid var(--border-color); border-radius: 4px;">
                                                            <table style="width: 100%; border-collapse: collapse; font-size: var(--font-size-sm);">
                                                                <thead style="position: sticky; top: 0; z-index: 10; background: var(--secondary-bg-color);">
                                                                    <tr style="border-bottom: 2px solid var(--border-color);">
                                                                        <th style="padding: 8px; text-align: left; background: var(--secondary-bg-color);">"Date (rr_dt)"</th>
                                                                        <th style="padding: 8px; text-align: left; background: var(--secondary-bg-color);">"RRD ID"</th>
                                                                        <th style="padding: 8px; text-align: right; background: var(--secondary-bg-color);">"PPVZ VW"</th>
                                                                        <th style="padding: 8px; text-align: right; background: var(--secondary-bg-color);">"PPVZ VW NDS"</th>
                                                                        <th style="padding: 8px; text-align: right; background: var(--secondary-bg-color);">"Retail Amount"</th>
                                                                        <th style="padding: 8px; text-align: right; background: var(--secondary-bg-color);">"PPVZ For Pay"</th>
                                                                        <th style="padding: 8px; text-align: right; background: var(--secondary-bg-color);">"Commission %"</th>
                                                                        <th style="padding: 8px; text-align: right; background: var(--secondary-bg-color);">"Retail Price"</th>
                                                                        <th style="padding: 8px; text-align: right; background: var(--secondary-bg-color);">"Retail w/Disc"</th>
                                                                        <th style="padding: 8px; text-align: right; background: var(--secondary-bg-color);">"Acquiring Fee"</th>
                                                                    </tr>
                                                                </thead>
                                                                <tbody>
                                                                    <For
                                                                        each=move || reports.clone()
                                                                        key=|report| format!("{}_{}", report.rr_dt, report.rrd_id)
                                                                        children=move |report: WbFinanceReportDto| {
                                                                            let rr_dt = report.rr_dt.clone();
                                                                            let rrd_id = report.rrd_id;
                                                                            view! {
                                                                                <tr
                                                                                    style="border-bottom: 1px solid var(--border-color); cursor: pointer;"
                                                                                    on:click=move |_| {
                                                                                        set_selected_finance_report.set(Some((rr_dt.clone(), rrd_id)))
                                                                                    }
                                                                                >
                                                                                    <td style="padding: 6px 8px;">{report.rr_dt.clone()}</td>
                                                                                    <td style="padding: 6px 8px;">{report.rrd_id}</td>
                                                                                    <td style="padding: 6px 8px; text-align: right;">
                                                                                        {report.ppvz_vw.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "-".to_string())}
                                                                                    </td>
                                                                                    <td style="padding: 6px 8px; text-align: right;">
                                                                                        {report.ppvz_vw_nds.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "-".to_string())}
                                                                                    </td>
                                                                                    <td style="padding: 6px 8px; text-align: right;">
                                                                                        {report.retail_amount.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "-".to_string())}
                                                                                    </td>
                                                                                    <td style="padding: 6px 8px; text-align: right;">
                                                                                        {report.ppvz_for_pay.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "-".to_string())}
                                                                                    </td>
                                                                                    <td style="padding: 6px 8px; text-align: right;">
                                                                                        {report.commission_percent.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "-".to_string())}
                                                                                    </td>
                                                                                    <td style="padding: 6px 8px; text-align: right;">
                                                                                        {report.retail_price.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "-".to_string())}
                                                                                    </td>
                                                                                    <td style="padding: 6px 8px; text-align: right;">
                                                                                        {report.retail_price_withdisc_rub.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "-".to_string())}
                                                                                    </td>
                                                                                    <td style="padding: 6px 8px; text-align: right;">
                                                                                        {report.acquiring_fee.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "-".to_string())}
                                                                                    </td>
                                                                                </tr>
                                                                            }
                                                                        }
                                                                    />
                                                                </tbody>
                                                            </table>
                                                        </div>
                                                    </div>
                                                }.into_any()
                                            }
                                        }
                                    },
                                    "projections" => view! {
                                        <div class="projections-info">
                                            {move || {
                                                if projections_loading.get() {
                                                    view! {
                                                        <div style="padding: var(--space-lg); text-align: center; color: var(--color-text-tertiary); font-size: var(--font-size-sm);">
                                                            "Загрузка проекций..."
                                                        </div>
                                                    }.into_any()
                                                } else if let Some(proj_data) = projections.get() {
                                                    let p900_items = proj_data["p900_sales_register"].as_array().cloned().unwrap_or_default();
                                                    let p904_items = proj_data["p904_sales_data"].as_array().cloned().unwrap_or_default();

                                                    view! {
                                                        <div style="display: flex; flex-direction: column; gap: var(--space-sm);">
                                                            // P900 Sales Register
                                                            <div style="background: var(--color-bg-white); padding: var(--space-sm); border-radius: var(--radius-md); box-shadow: var(--shadow-sm);">
                                                                <h3 style="margin: 0 0 var(--space-sm) 0; color: var(--color-text-primary); font-size: var(--font-size-base); font-weight: var(--font-weight-semibold); border-bottom: 2px solid var(--color-warning); padding-bottom: var(--space-xs);">
                                                                    {format!("📊 Sales Register (p900) - {} записей", p900_items.len())}
                                                                </h3>
                                                                {if !p900_items.is_empty() {
                                                                    view! {
                                                                        <div style="overflow-x: auto;">
                                                                            <table style="width: 100%; border-collapse: collapse; font-size: var(--font-size-sm);">
                                                                                <thead>
                                                                                    <tr style="background: var(--color-bg-secondary);">
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: left; border: 1px solid var(--color-border);">"MP"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: left; border: 1px solid var(--color-border);">"SKU"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: left; border: 1px solid var(--color-border);">"Title"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Qty"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">"Amount"</th>
                                                                                    </tr>
                                                                                </thead>
                                                                                <tbody>
                                                                                    {p900_items.iter().map(|item| {
                                                                                        let mp = item["marketplace"].as_str().unwrap_or("—");
                                                                                        let sku = item["seller_sku"].as_str().unwrap_or("—");
                                                                                        let title = item["title"].as_str().unwrap_or("—");
                                                                                        let qty = item["qty"].as_f64().unwrap_or(0.0);
                                                                                        let amount = item["amount_line"].as_f64().unwrap_or(0.0);

                                                                                        view! {
                                                                                            <tr style="border-bottom: 1px solid var(--color-border-light);">
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); border: 1px solid var(--color-border);">{mp}</td>
                                                                                                <td class="field-value-mono" style="padding: var(--space-2xs) var(--space-xs); border: 1px solid var(--color-border);">{sku}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); border: 1px solid var(--color-border);">{title}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{qty}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border); font-weight: var(--font-weight-semibold);">{format!("{:.2}", amount)}</td>
                                                                                            </tr>
                                                                                        }
                                                                                    }).collect::<Vec<_>>()}
                                                                                </tbody>
                                                                            </table>
                                                                        </div>
                                                                    }.into_any()
                                                                } else {
                                                                    view! {
                                                                        <p style="text-align: center; padding: var(--space-sm); color: var(--color-text-tertiary); font-size: var(--font-size-sm);">"Нет записей"</p>
                                                                    }.into_any()
                                                                }}
                                                            </div>

                                                            // P904 Sales Data
                                                            <div style="background: var(--color-bg-white); padding: var(--space-sm); border-radius: var(--radius-md); box-shadow: var(--shadow-sm);">
                                                                <h3 style="margin: 0 0 var(--space-sm) 0; color: var(--color-text-primary); font-size: var(--font-size-base); font-weight: var(--font-weight-semibold); border-bottom: 2px solid var(--color-primary); padding-bottom: var(--space-xs);">
                                                                    {format!("📈 Sales Data (p904) - {} записей", p904_items.len())}
                                                                </h3>
                                                                {if !p904_items.is_empty() {
                                                                    view! {
                                                                        <div style="overflow-x: auto;">
                                                                            <table style="width: 100%; border-collapse: collapse; font-size: var(--font-size-xs);">
                                                                                <thead>
                                                                                    <tr style="background: var(--color-bg-secondary);">
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: left; border: 1px solid var(--color-border);">"Article"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);" title="total_price">"Price Full"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);" title="price_list">"Price List"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);" title="finished_price (если price_effective > 0)">"Cust In"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);" title="finished_price (если price_effective <= 0)">"Cust Out"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);" title="amount_line - price_effective">"Comm Out"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);" title="spp → coinvest_persent">"Coinv%"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);" title="commission_out / price_effective * 100">"Comm%"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);" title="finished_price * -1.9%">"Acq Out"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);" title="amount_line - finished_price (если > 0)">"Coinv In"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);" title="amount_line + acquiring_out + commission_out">"Total"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);" title="(cust_out + cust_in) - (acq_out + coinv_in + comm_out)">"Sell Out"</th>
                                                                                    </tr>
                                                                                </thead>
                                                                                <tbody>
                                                                                    {p904_items.iter().map(|item| {
                                                                                        let article = item["article"].as_str().unwrap_or("—");
                                                                                        let price_full = item["price_full"].as_f64().unwrap_or(0.0);
                                                                                        let price_list = item["price_list"].as_f64().unwrap_or(0.0);
                                                                                        let customer_in = item["customer_in"].as_f64().unwrap_or(0.0);
                                                                                        let customer_out = item["customer_out"].as_f64().unwrap_or(0.0);
                                                                                        let commission_out = item["commission_out"].as_f64().unwrap_or(0.0);
                                                                                        let coinvest_persent = item["coinvest_persent"].as_f64().unwrap_or(0.0);
                                                                                        let commission_percent = item["commission_percent"].as_f64().unwrap_or(0.0);
                                                                                        let acquiring_out = item["acquiring_out"].as_f64().unwrap_or(0.0);
                                                                                        let coinvest_in = item["coinvest_in"].as_f64().unwrap_or(0.0);
                                                                                        let total = item["total"].as_f64().unwrap_or(0.0);
                                                                                        let seller_out = item["seller_out"].as_f64().unwrap_or(0.0);

                                                                                        view! {
                                                                                            <tr style="border-bottom: 1px solid var(--color-border-light);">
                                                                                                <td class="field-value-mono" style="padding: var(--space-2xs) var(--space-xs); border: 1px solid var(--color-border);">{article}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", price_full)}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", price_list)}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border); color: #2e7d32; background: #e8f5e9;">{format!("{:.2}", customer_in)}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border); color: #c62828; background: #ffebee;">{format!("{:.2}", customer_out)}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border); color: #c62828;">{format!("{:.2}", commission_out)}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.1}%", coinvest_persent)}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}%", commission_percent)}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border); color: #c62828;">{format!("{:.2}", acquiring_out)}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border); color: #2e7d32;">{format!("{:.2}", coinvest_in)}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border); font-weight: var(--font-weight-semibold);">{format!("{:.2}", total)}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border); font-weight: var(--font-weight-bold); background: #e3f2fd;">{format!("{:.2}", seller_out)}</td>
                                                                                            </tr>
                                                                                        }
                                                                                    }).collect::<Vec<_>>()}
                                                                                </tbody>
                                                                            </table>
                                                                        </div>
                                                                    }.into_any()
                                                                } else {
                                                                    view! {
                                                                        <p style="text-align: center; padding: var(--space-sm); color: var(--color-text-tertiary); font-size: var(--font-size-sm);">"Нет записей"</p>
                                                                    }.into_any()
                                                                }}
                                                            </div>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div style="padding: var(--space-lg); text-align: center; color: var(--color-text-tertiary); font-size: var(--font-size-sm);">
                                                            "Нет данных проекций"
                                                        </div>
                                                    }.into_any()
                                                }
                                            }}
                                        </div>
                                    }.into_any(),
                                    _ => view! { <div style="font-size: var(--font-size-sm);">"Unknown tab"</div> }.into_any()
                                }
                                    }}
                                </div>
                            </div>
                        }.into_any()
                    } else {
                        view! { <div>"No data"</div> }.into_any()
                    }
                }}
            </div>
        </div>

        // Modals for linked aggregates
        {move || selected_marketplace_product_id.get().map(|mp_id| {
            let on_close_modal = {
                let set_selected = set_selected_marketplace_product_id.clone();
                move || set_selected.set(None)
            };
            view! {
                <div class="modal-overlay">
                    <div class="modal-content">
                        <MarketplaceProductDetails
                            id=Some(mp_id)
                            on_saved=Rc::new({
                                let on_close_modal = on_close_modal.clone();
                                move |_| on_close_modal()
                            })
                            on_cancel=Rc::new(move |_| on_close_modal())
                        />
                    </div>
                </div>
            }
        })}

        {move || selected_nomenclature_id.get().map(|nom_id| {
            let on_close_modal = {
                let set_selected = set_selected_nomenclature_id.clone();
                move || set_selected.set(None)
            };
            view! {
                <div class="modal-overlay">
                    <div class="modal-content">
                        <NomenclatureDetails
                            id=Some(nom_id)
                            on_saved=Callback::new(move |_| on_close_modal())
                            on_cancel=Callback::new(move |_| on_close_modal())
                        />
                    </div>
                </div>
            }
        })}

        // Modal for WbFinanceReportDetail when clicking on a linked finance report
        {move || selected_finance_report.get().map(|(rr_dt, rrd_id)| {
            view! {
                <div style="position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 2000;">
                    <div style="background: white; border-radius: 8px; box-shadow: 0 4px 16px rgba(0,0,0,0.2); width: 90%; max-width: 1200px; max-height: 90vh; overflow: hidden; display: flex; flex-direction: column;">
                        <WbFinanceReportDetail
                            rr_dt=rr_dt
                            rrd_id=rrd_id
                            on_close=move || set_selected_finance_report.set(None)
                        />
                    </div>
                </div>
            }
        })}
    }
}
