use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::rc::Rc;

// Details components for linked aggregates
use crate::domain::a004_nomenclature::ui::details::NomenclatureDetails;
use crate::domain::a007_marketplace_product::ui::details::MarketplaceProductDetails;

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
    let (nomenclature_info, set_nomenclature_info) = signal::<Option<NomenclatureInfo>>(None);
    let (selected_marketplace_product_id, set_selected_marketplace_product_id) =
        signal::<Option<String>>(None);
    let (selected_nomenclature_id, set_selected_nomenclature_id) = signal::<Option<String>>(None);

    let is_posted = Memo::new(move |_| sale.get().map(|s| s.metadata.is_posted).unwrap_or(false));

    // Сохраняем id в StoredValue для использования в обработчиках
    let stored_id = StoredValue::new(id.clone());

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
        <div class="sale-detail" style="padding: 20px; height: 100%; display: flex; flex-direction: column;">
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px; flex-shrink: 0; border-bottom: 2px solid #e0e0e0; padding-bottom: 15px;">
                <div style="display: flex; align-items: center; gap: 15px;">
                    <h2 style="margin: 0;">"Wildberries Sales Details"</h2>
                    <Show when=move || sale.get().is_some()>
                        {move || {
                            let posted = is_posted.get();
                            view! {
                                <div style=move || format!(
                                    "display: flex; align-items: center; gap: 8px; padding: 6px 12px; border-radius: 20px; font-size: 13px; font-weight: 600; {}",
                                    if posted {
                                        "background: #e8f5e9; color: #2e7d32; border: 2px solid #4CAF50;"
                                    } else {
                                        "background: #fff3e0; color: #e65100; border: 2px solid #FF9800;"
                                    }
                                )>
                                    <span style="font-size: 16px;">{if posted { "✓" } else { "○" }}</span>
                                    <span>{if posted { "Проведен" } else { "Не проведен" }}</span>
                                </div>
                            }
                        }}
                    </Show>
                </div>
                <div style="display: flex; gap: 8px;">
                    <Show when=move || sale.get().is_some()>
                        <Show
                            when=move || !is_posted.get()
                            fallback=move || {
                                view! {
                                    <button
                                        on:click=move |_| {
                                            let doc_id = stored_id.get_value();
                                            set_posting.set(true);
                                            wasm_bindgen_futures::spawn_local(async move {
                                                let url = format!("http://localhost:3000/api/a012/wb-sales/{}/unpost", doc_id);
                                                match Request::post(&url).send().await {
                                                    Ok(response) => {
                                                        if response.status() == 200 {
                                                            log!("Document unposted successfully");
                                                            // Перезагрузить только данные документа
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
                                        disabled=move || posting.get()
                                        style="padding: 8px 16px; background: #FF9800; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 600;"
                                    >
                                        {move || if posting.get() { "Unposting..." } else { "✗ Unpost" }}
                                    </button>
                                }
                            }
                        >
                            {
                                view! {
                                    <button
                                        on:click=move |_| {
                                            let doc_id = stored_id.get_value();
                                            set_posting.set(true);
                                            wasm_bindgen_futures::spawn_local(async move {
                                                let url = format!("http://localhost:3000/api/a012/wb-sales/{}/post", doc_id);
                                                match Request::post(&url).send().await {
                                                    Ok(response) => {
                                                        if response.status() == 200 {
                                                            log!("Document posted successfully");
                                                            // Перезагрузить только данные документа
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
                                        disabled=move || posting.get()
                                        style="padding: 8px 16px; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 600;"
                                    >
                                        {move || if posting.get() { "Posting..." } else { "✓ Post" }}
                                    </button>
                                }
                            }
                        </Show>
                    </Show>
                    <button
                        on:click=move |_| on_close.run(())
                        style="padding: 8px 16px; background: #f44336; color: white; border: none; border-radius: 4px; cursor: pointer;"
                    >
                        "✕ Close"
                    </button>
                </div>
            </div>

            <div style="flex: 1; overflow-y: auto; min-height: 0;">
                {move || {
                    if loading.get() {
                        view! {
                            <div style="text-align: center; padding: 40px;">
                                <p>"Loading..."</p>
                            </div>
                        }.into_any()
                    } else if let Some(err) = error.get() {
                        view! {
                            <div style="padding: 20px; background: #ffebee; border: 1px solid #ffcdd2; border-radius: 4px; color: #c62828;">
                                <strong>"Error: "</strong>{err}
                            </div>
                        }.into_any()
                    } else if let Some(sale_data) = sale.get() {
                        view! {
                            <div style="height: 100%; display: flex; flex-direction: column;">
                                // Вкладки
                                <div class="tabs" style="border-bottom: 2px solid #ddd; margin-bottom: 20px; flex-shrink: 0; background: white; position: sticky; top: 0; z-index: 10;">
                                    <button
                                        on:click=move |_| set_active_tab.set("general")
                                        style=move || format!(
                                            "padding: 10px 20px; border: none; border-radius: 4px 4px 0 0; cursor: pointer; margin-right: 5px; font-weight: 500; {}",
                                            if active_tab.get() == "general" {
                                                "background: #2196F3; color: white; border-bottom: 2px solid #2196F3;"
                                            } else {
                                                "background: #f5f5f5; color: #666;"
                                            }
                                        )
                                    >
                                        "📋 General"
                                    </button>
                                    <button
                                        on:click=move |_| set_active_tab.set("line")
                                        style=move || format!(
                                            "padding: 10px 20px; border: none; border-radius: 4px 4px 0 0; cursor: pointer; margin-right: 5px; font-weight: 500; {}",
                                            if active_tab.get() == "line" {
                                                "background: #2196F3; color: white; border-bottom: 2px solid #2196F3;"
                                            } else {
                                                "background: #f5f5f5; color: #666;"
                                            }
                                        )
                                    >
                                        "📦 Line Details"
                                    </button>
                                    <button
                                        on:click=move |_| set_active_tab.set("json")
                                        style=move || format!(
                                            "padding: 10px 20px; border: none; border-radius: 4px 4px 0 0; cursor: pointer; font-weight: 500; {}",
                                            if active_tab.get() == "json" {
                                                "background: #2196F3; color: white; border-bottom: 2px solid #2196F3;"
                                            } else {
                                                "background: #f5f5f5; color: #666;"
                                            }
                                        )
                                    >
                                        "📄 Raw JSON"
                                    </button>
                                </div>

                                // Контент вкладок
                                <div style="flex: 1; overflow-y: auto; padding: 10px 0;">
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
                                                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 15px; margin-bottom: 20px;">
                                                    // Блок: Товар маркетплейса
                                                    <div style="padding: 10px 15px; background: #e3f2fd; border-radius: 6px; border-left: 3px solid #2196F3;">
                                                        <div style="font-weight: 600; color: #1976d2; font-size: 13px; margin-bottom: 8px;">"📦 Товар маркетплейса"</div>
                                                        {move || if let Some(ref mp_info) = marketplace_product_info.get() {
                                                            view! {
                                                                <div style="font-size: 13px; line-height: 1.6;">
                                                                    <div
                                                                        style="color: #0d47a1; font-weight: 600; cursor: pointer; text-decoration: underline;"
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
                                                                    <div style="color: #666; margin-top: 4px;">
                                                                        <span style="font-weight: 600;">"Артикул: "</span>
                                                                        <code style="background: #bbdefb; padding: 2px 6px; border-radius: 3px; font-size: 12px;">{mp_info.article.clone()}</code>
                                                                    </div>
                                                                </div>
                                                            }.into_any()
                                                        } else {
                                                            view! {
                                                                <div style="color: #999; font-style: italic; font-size: 13px;">"Не привязан"</div>
                                                            }.into_any()
                                                        }}
                                                    </div>

                                                    // Блок: Номенклатура 1С
                                                    <div style="padding: 10px 15px; background: #e8f5e9; border-radius: 6px; border-left: 3px solid #4CAF50;">
                                                        <div style="font-weight: 600; color: #2e7d32; font-size: 13px; margin-bottom: 8px;">"📋 Номенклатура 1С"</div>
                                                        {move || if let Some(ref nom_info) = nomenclature_info.get() {
                                                            view! {
                                                                <div style="font-size: 13px; line-height: 1.6;">
                                                                    <div
                                                                        style="color: #1b5e20; font-weight: 600; cursor: pointer; text-decoration: underline;"
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
                                                                    <div style="color: #666; margin-top: 4px;">
                                                                        <span style="font-weight: 600;">"Артикул: "</span>
                                                                        <code style="background: #c8e6c9; padding: 2px 6px; border-radius: 3px; font-size: 12px;">{nom_info.article.clone()}</code>
                                                                    </div>
                                                                </div>
                                                            }.into_any()
                                                        } else {
                                                            view! {
                                                                <div style="color: #999; font-style: italic; font-size: 13px;">"Не привязана"</div>
                                                            }.into_any()
                                                        }}
                                                    </div>
                                                </div>

                                                // 2 колонки для основных данных
                                                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 20px; margin-bottom: 20px;">
                                                    // Левая колонка
                                                    <div style="background: white; padding: 15px; border-radius: 8px; border: 1px solid #e0e0e0;">
                                                        <div style="display: grid; grid-template-columns: 140px 1fr; gap: 12px; align-items: start;">
                                                            <div style="font-weight: 600; color: #555;">"Document №:"</div>
                                                            <div>{sale_data.header.document_no.clone()}</div>

                                                            <div style="font-weight: 600; color: #555;">"Code:"</div>
                                                            <div>{sale_data.code.clone()}</div>

                                                            <div style="font-weight: 600; color: #555;">"Description:"</div>
                                                            <div>{sale_data.description.clone()}</div>

                                                            <div style="font-weight: 600; color: #555;">"Event Type:"</div>
                                                            <div>
                                                                <span style="padding: 2px 8px; background: #e3f2fd; color: #1976d2; border-radius: 3px; font-weight: 500;">
                                                                    {sale_data.state.event_type.clone()}
                                                                </span>
                                                            </div>

                                                            <div style="font-weight: 600; color: #555;">"Status:"</div>
                                                            <div>
                                                                <span style="padding: 2px 8px; background: #e8f5e9; color: #2e7d32; border-radius: 3px; font-weight: 500;">
                                                                    {sale_data.state.status_norm.clone()}
                                                                </span>
                                                            </div>

                                                            <div style="font-weight: 600; color: #555;">"Sale Date:"</div>
                                                            <div>{format_datetime(&sale_data.state.sale_dt)}</div>

                                                            <div style="font-weight: 600; color: #555;">"Last Change:"</div>
                                                            <div>{sale_data.state.last_change_dt.as_ref().map(|d| format_datetime(d)).unwrap_or("—".to_string())}</div>
                                                        </div>
                                                    </div>

                                                    // Правая колонка
                                                    <div style="background: white; padding: 15px; border-radius: 8px; border: 1px solid #e0e0e0;">
                                                        <div style="display: grid; grid-template-columns: 140px 1fr; gap: 12px; align-items: start;">
                                                            <div style="font-weight: 600; color: #555;">"Warehouse Name:"</div>
                                                            <div>{sale_data.warehouse.warehouse_name.clone().unwrap_or("—".to_string())}</div>

                                                            <div style="font-weight: 600; color: #555;">"Warehouse Type:"</div>
                                                            <div>{sale_data.warehouse.warehouse_type.clone().unwrap_or("—".to_string())}</div>

                                                            <div style="font-weight: 600; color: #555;">"Is Supply:"</div>
                                                            <div>
                                                                {match sale_data.state.is_supply {
                                                                    Some(true) => view! {
                                                                        <span style="padding: 2px 8px; background: #e3f2fd; color: #1976d2; border-radius: 3px; font-weight: 500;">
                                                                            "Yes"
                                                                        </span>
                                                                    }.into_any(),
                                                                    Some(false) => view! {
                                                                        <span style="padding: 2px 8px; background: #ffebee; color: #c62828; border-radius: 3px; font-weight: 500;">
                                                                            "No"
                                                                        </span>
                                                                    }.into_any(),
                                                                    None => view! { <span>"—"</span> }.into_any(),
                                                                }}
                                                            </div>

                                                            <div style="font-weight: 600; color: #555;">"Is Realization:"</div>
                                                            <div>
                                                                {match sale_data.state.is_realization {
                                                                    Some(true) => view! {
                                                                        <span style="padding: 2px 8px; background: #e3f2fd; color: #1976d2; border-radius: 3px; font-weight: 500;">
                                                                            "Yes"
                                                                        </span>
                                                                    }.into_any(),
                                                                    Some(false) => view! {
                                                                        <span style="padding: 2px 8px; background: #ffebee; color: #c62828; border-radius: 3px; font-weight: 500;">
                                                                            "No"
                                                                        </span>
                                                                    }.into_any(),
                                                                    None => view! { <span>"—"</span> }.into_any(),
                                                                }}
                                                            </div>

                                                            <div style="font-weight: 600; color: #555;">"Created At:"</div>
                                                            <div>{format_datetime(&sale_data.metadata.created_at)}</div>

                                                            <div style="font-weight: 600; color: #555;">"Updated At:"</div>
                                                            <div>{format_datetime(&sale_data.metadata.updated_at)}</div>

                                                            <div style="font-weight: 600; color: #555;">"Version:"</div>
                                                            <div>{sale_data.metadata.version}</div>
                                                        </div>
                                                    </div>
                                                </div>

                                                // UUID section at bottom
                                                <div style="background: #fafafa; padding: 15px; border-radius: 8px; border: 1px solid #e0e0e0;">
                                                    <h4 style="margin: 0 0 12px 0; color: #666; font-size: 14px; font-weight: 600;">"Technical IDs"</h4>
                                                    <div style="display: grid; grid-template-columns: 180px 1fr; gap: 12px; align-items: center; font-size: 13px;">

                                                        <div style="font-weight: 600; color: #555;">"Connection ID:"</div>
                                                        <div style="display: flex; align-items: center; gap: 8px;">
                                                            <code style="font-size: 12px; color: #666;" title=conn_id.clone()>{conn_id.clone()}</code>
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
                                                                style="padding: 2px 6px; font-size: 11px; border: 1px solid #ddd; background: white; border-radius: 3px; cursor: pointer;"
                                                                title="Copy to clipboard"
                                                            >
                                                                "📋"
                                                            </button>
                                                        </div>

                                                        <div style="font-weight: 600; color: #555;">"Organization ID:"</div>
                                                        <div style="display: flex; align-items: center; gap: 8px;">
                                                            <code style="font-size: 12px; color: #666;" title=org_id.clone()>{org_id.clone()}</code>
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
                                                                style="padding: 2px 6px; font-size: 11px; border: 1px solid #ddd; background: white; border-radius: 3px; cursor: pointer;"
                                                                title="Copy to clipboard"
                                                            >
                                                                "📋"
                                                            </button>
                                                        </div>

                                                        <div style="font-weight: 600; color: #555;">"Marketplace ID:"</div>
                                                        <div style="display: flex; align-items: center; gap: 8px;">
                                                            <code style="font-size: 12px; color: #666;" title=mp_id.clone()>{mp_id.clone()}</code>
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
                                                                style="padding: 2px 6px; font-size: 11px; border: 1px solid #ddd; background: white; border-radius: 3px; cursor: pointer;"
                                                                title="Copy to clipboard"
                                                            >
                                                                "📋"
                                                            </button>
                                                        </div>

                                                        <div style="font-weight: 600; color: #555;">"Marketplace Product ID:"</div>
                                                        <div style="display: flex; align-items: center; gap: 8px;">
                                                            {if let Some(ref mp_ref) = sale_data.marketplace_product_ref {
                                                                let mp_ref_copy = mp_ref.clone();
                                                                view! {
                                                                    <>
                                                                        <code style="font-size: 12px; color: #666;" title=mp_ref.clone()>{mp_ref.clone()}</code>
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
                                                                            style="padding: 2px 6px; font-size: 11px; border: 1px solid #ddd; background: white; border-radius: 3px; cursor: pointer;"
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

                                                        <div style="font-weight: 600; color: #555;">"Nomenclature ID:"</div>
                                                        <div style="display: flex; align-items: center; gap: 8px;">
                                                            {if let Some(ref nom_ref) = sale_data.nomenclature_ref {
                                                                let nom_ref_copy = nom_ref.clone();
                                                                view! {
                                                                    <>
                                                                        <code style="font-size: 12px; color: #666;" title=nom_ref.clone()>{nom_ref.clone()}</code>
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
                                                                            style="padding: 2px 6px; font-size: 11px; border: 1px solid #ddd; background: white; border-radius: 3px; cursor: pointer;"
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
                                                <div style="margin-bottom: 20px;">
                                                    <div style="display: grid; grid-template-columns: 200px 1fr; gap: 10px 20px; align-items: center; margin-bottom: 20px;">
                                                        <div style="font-weight: 600; color: #555;">"Line ID:"</div>
                                                        <div style="font-family: var(--font-family-base); font-size: 14px;">{line.line_id.clone()}</div>

                                                        <div style="font-weight: 600; color: #555;">"Артикул продавца:"</div>
                                                        <div style="font-family: var(--font-family-base); font-size: 14px; font-weight: 500;">{line.supplier_article.clone()}</div>

                                                        <div style="font-weight: 600; color: #555;">"NM ID:"</div>
                                                        <div style="font-family: var(--font-family-base); font-size: 14px;">{line.nm_id}</div>

                                                        <div style="font-weight: 600; color: #555;">"Штрихкод:"</div>
                                                        <div style="font-family: var(--font-family-base); font-size: 14px;">{line.barcode.clone()}</div>

                                                        <div style="font-weight: 600; color: #555;">"Название:"</div>
                                                        <div style="font-family: var(--font-family-base); font-size: 14px; font-weight: 500;">{line.name.clone()}</div>

                                                        <div style="font-weight: 600; color: #555;">"Количество:"</div>
                                                        <div style="font-family: var(--font-family-base); font-size: 14px;">
                                                            {format!("{:.0}", line.qty)}
                                                        </div>
                                                    </div>

                                                    <h3 style="margin: 20px 0 10px 0; font-size: 16px; color: #555;">"Суммы и проценты"</h3>
                                                    <table style="width: 50%; border-collapse: collapse; font-family: var(--font-family-base); font-size: 14px;">
                                                        <thead>
                                                            <tr style="background: #f5f5f5;">
                                                                <th style="border: 1px solid #ddd; padding: 8px; text-align: left; width: 40%;">"Наименование"</th>
                                                                <th style="border: 1px solid #ddd; padding: 8px; text-align: left; width: 25%;">"Поле"</th>
                                                                <th style="border: 1px solid #ddd; padding: 8px; text-align: right; width: 20%;">"Значение"</th>
                                                                <th style="border: 1px solid #ddd; padding: 8px; text-align: left; width: 15%;">"Ед."</th>
                                                            </tr>
                                                        </thead>
                                                        <tbody>
                                                            <tr>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">"Цена без скидок"</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;"><code style="font-size: 0.9em;">"price_list"</code></td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{line.price_list.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">"rub"</td>
                                                            </tr>
                                                            <tr>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">"Сумма скидок"</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;"><code style="font-size: 0.9em;">"discount_total"</code></td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{line.discount_total.map(|d| format!("{:.2}", d)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">"rub"</td>
                                                            </tr>
                                                            <tr>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">"Цена после скидок"</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;"><code style="font-size: 0.9em;">"price_effective"</code></td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{line.price_effective.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">"rub"</td>
                                                            </tr>
                                                            <tr style="background:rgb(138, 227, 254);">
                                                                <td style="border: 1px solid #ddd; padding: 8px; font-weight: 600;">"К выплате"</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;"><code style="font-size: 0.9em;">"amount_line"</code></td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right; font-weight: 600;">{line.amount_line.map(|a| format!("{:.2}", a)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">"rub"</td>
                                                            </tr>
                                                            <tr>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">"Полная цена"</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;"><code style="font-size: 0.9em;">"total_price"</code></td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{line.total_price.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">"rub"</td>
                                                            </tr>
                                                            <tr>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">"Сумма платежа"</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;"><code style="font-size: 0.9em;">"payment_sale_amount"</code></td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{line.payment_sale_amount.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">"rub"</td>
                                                            </tr>
                                                            <tr>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">"Итоговая цена"</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;"><code style="font-size: 0.9em;">"finished_price"</code></td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{line.finished_price.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">"rub"</td>
                                                            </tr>
                                                            <tr>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">"Процент скидки"</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;"><code style="font-size: 0.9em;">"discount_percent"</code></td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{line.discount_percent.map(|d| format!("{:.1}", d)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">"%"</td>
                                                            </tr>
                                                            <tr>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">"СПП"</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;"><code style="font-size: 0.9em;">"spp"</code></td>
                                                                <td style="border: 1px solid #ddd; padding: 8px; text-align: right;">{line.spp.map(|s| format!("{:.1}", s)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid #ddd; padding: 8px;">"%"</td>
                                                            </tr>
                                                        </tbody>
                                                    </table>
                                                </div>
                                            </div>
                                        }.into_any()
                                    },
                                    "json" => view! {
                                        <div class="json-info">
                                            <div style="margin-bottom: 10px;">
                                                <strong>"Raw JSON from WB API:"</strong>
                                            </div>
                                            {move || {
                                                if let Some(json) = raw_json_from_wb.get() {
                                                    view! {
                                                        <pre style="background: #f5f5f5; padding: 15px; border-radius: 4px; overflow-x: auto; font-size: 0.85em;">
                                                            {json}
                                                        </pre>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div style="padding: 20px; text-align: center; color: #999;">
                                                            "Loading raw JSON from WB..."
                                                        </div>
                                                    }.into_any()
                                                }
                                            }}
                                        </div>
                                    }.into_any(),
                                        _ => view! { <div>"Unknown tab"</div> }.into_any()
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
    }
}
