use gloo_net::http::Request;
use leptos::logging::log;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Details components for linked aggregates
use crate::domain::a004_nomenclature::ui::details::NomenclatureDetails;
use crate::domain::a007_marketplace_product::ui::details::MarketplaceProductDetails;
use crate::layout::global_context::AppGlobalContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NomenclatureInfo {
    pub description: String,
    pub article: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketplaceProductInfo {
    pub description: String,
    pub article: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderDetailDto {
    pub id: String,
    pub code: String,
    pub description: String,
    pub header: HeaderDto,
    pub lines: Vec<LineDto>,
    pub state: StateDto,
    pub source_meta: SourceMetaDto,
    pub metadata: MetadataDto,
    /// Флаг ошибки (ненулевой при отсутствии сопоставления номенклатуры в строках)
    #[serde(default)]
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderDto {
    pub document_no: String,
    pub connection_id: String,
    pub organization_id: String,
    pub marketplace_id: String,
    pub campaign_id: String,
    pub total_amount: Option<f64>,
    pub currency: Option<String>,
    #[serde(default)]
    pub items_total: Option<f64>,
    #[serde(default)]
    pub delivery_total: Option<f64>,
    #[serde(default)]
    pub subsidies_json: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineDto {
    pub line_id: String,
    pub shop_sku: String,
    pub offer_id: String,
    pub name: String,
    pub qty: f64,
    pub price_list: Option<f64>,
    pub discount_total: Option<f64>,
    pub price_effective: Option<f64>,
    pub amount_line: Option<f64>,
    pub currency_code: Option<String>,
    #[serde(default)]
    pub buyer_price: Option<f64>,
    #[serde(default)]
    pub subsidies_json: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    /// Плановая цена (пока константа = 0)
    #[serde(default)]
    pub price_plan: Option<f64>,
    /// Ссылка на товар маркетплейса (a007_marketplace_product)
    #[serde(default)]
    pub marketplace_product_ref: Option<String>,
    /// Ссылка на номенклатуру 1С (a004_nomenclature)
    #[serde(default)]
    pub nomenclature_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDto {
    pub status_raw: String,
    pub substatus_raw: Option<String>,
    pub status_norm: String,
    pub status_changed_at: Option<String>,
    pub updated_at_source: Option<String>,
    pub creation_date: Option<String>,
    pub delivery_date: Option<String>,
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

#[component]
pub fn YmOrderDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let (order, set_order) = signal::<Option<YmOrderDetailDto>>(None);
    let (raw_json_from_ym, set_raw_json_from_ym) = signal::<Option<String>>(None);
    let (loading, set_loading) = signal(true);
    let (error, set_error) = signal::<Option<String>>(None);
    let (active_tab, set_active_tab) = signal("general");
    let (posting, set_posting) = signal(false);

    // Информация о номенклатуре для каждой строки (ключ - line_id)
    let (nomenclatures_info, set_nomenclatures_info) =
        signal::<HashMap<String, NomenclatureInfo>>(HashMap::new());
    let (selected_nomenclature_id, set_selected_nomenclature_id) = signal::<Option<String>>(None);
    let (selected_marketplace_product_id, set_selected_marketplace_product_id) =
        signal::<Option<String>>(None);

    // Projections signals
    let (projections, set_projections) = signal::<Option<serde_json::Value>>(None);
    let (projections_loading, set_projections_loading) = signal(false);

    let is_posted = Memo::new(move |_| order.get().map(|o| o.metadata.is_posted).unwrap_or(false));

    // Сохраняем id для использования в разных местах
    let id_stored = StoredValue::new(id.clone());

    // Получаем tabs_store для обновления названия вкладки
    let tabs_store = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext должен быть доступен");

    // Функция для загрузки информации о номенклатуре для всех строк
    let load_nomenclatures = move |lines: Vec<LineDto>| {
        for line in lines {
            if let Some(nom_ref) = line.nomenclature_ref.clone() {
                let line_id = line.line_id.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let url = format!("http://localhost:3000/api/nomenclature/{}", nom_ref);
                    if let Ok(response) = Request::get(&url).send().await {
                        if response.status() == 200 {
                            if let Ok(text) = response.text().await {
                                if let Ok(nom) = serde_json::from_str::<serde_json::Value>(&text) {
                                    let info = NomenclatureInfo {
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
                                    };
                                    set_nomenclatures_info.update(|map| {
                                        map.insert(line_id, info);
                                    });
                                }
                            }
                        }
                    }
                });
            }
        }
    };

    // Загрузить детальные данные
    Effect::new(move || {
        let id = id_stored.get_value();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            set_error.set(None);

            let url = format!("http://localhost:3000/api/a013/ym-order/{}", id);

            match Request::get(&url).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status == 200 {
                        match response.text().await {
                            Ok(text) => {
                                // Парсим структуру
                                match serde_json::from_str::<YmOrderDetailDto>(&text) {
                                    Ok(data) => {
                                        // Загружаем raw JSON от YM
                                        let raw_payload_ref =
                                            data.source_meta.raw_payload_ref.clone();

                                        // Загружаем информацию о номенклатуре для всех строк
                                        load_nomenclatures(data.lines.clone());

                                        // Обновляем заголовок вкладки
                                        let doc_no = data.header.document_no.clone();
                                        tabs_store.update_tab_title(
                                            &format!("a013_ym_order_detail_{}", id),
                                            &format!("YM Order: {}", doc_no),
                                        );

                                        set_order.set(Some(data));
                                        set_loading.set(false);

                                        // Асинхронная загрузка raw JSON
                                        wasm_bindgen_futures::spawn_local(async move {
                                            let raw_url = format!(
                                                "http://localhost:3000/api/a013/raw/{}",
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
                                                                    set_raw_json_from_ym
                                                                        .set(Some(formatted));
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    log!(
                                                        "Failed to load raw JSON from YM: {:?}",
                                                        e
                                                    );
                                                }
                                            }
                                        });

                                        // Асинхронная загрузка проекций
                                        wasm_bindgen_futures::spawn_local(async move {
                                            set_projections_loading.set(true);
                                            let projections_url = format!(
                                                "http://localhost:3000/api/a013/ym-order/{}/projections",
                                                id
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
                                        log!("Failed to parse order: {:?}", e);
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
                    log!("Failed to fetch order: {:?}", e);
                    set_error.set(Some(format!("Failed to fetch: {}", e)));
                    set_loading.set(false);
                }
            }
        });
    });

    // Обработчики для Post/Unpost
    let handle_post = move |_| {
        let id = id_stored.get_value();
        set_posting.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            let url = format!("http://localhost:3000/api/a013/ym-order/{}/post", id);
            match Request::post(&url).send().await {
                Ok(resp) => {
                    if resp.status() == 200 {
                        log!("Document posted successfully");
                        // Перезагрузить документ
                        set_loading.set(true);
                        let reload_url = format!("http://localhost:3000/api/a013/ym-order/{}", id);
                        if let Ok(response) = Request::get(&reload_url).send().await {
                            if response.status() == 200 {
                                if let Ok(text) = response.text().await {
                                    if let Ok(data) =
                                        serde_json::from_str::<YmOrderDetailDto>(&text)
                                    {
                                        set_order.set(Some(data));
                                    }
                                }
                            }
                        }
                        // Перезагрузить проекции
                        let projections_url =
                            format!("http://localhost:3000/api/a013/ym-order/{}/projections", id);
                        if let Ok(resp) = Request::get(&projections_url).send().await {
                            if resp.status() == 200 {
                                if let Ok(text) = resp.text().await {
                                    if let Ok(proj_data) =
                                        serde_json::from_str::<serde_json::Value>(&text)
                                    {
                                        set_projections.set(Some(proj_data));
                                    }
                                }
                            }
                        }
                        set_loading.set(false);
                    } else {
                        log!("Failed to post document: HTTP {}", resp.status());
                    }
                }
                Err(e) => {
                    log!("Failed to post document: {:?}", e);
                }
            }
            set_posting.set(false);
        });
    };

    let handle_unpost = move |_| {
        let id = id_stored.get_value();
        set_posting.set(true);
        wasm_bindgen_futures::spawn_local(async move {
            let url = format!("http://localhost:3000/api/a013/ym-order/{}/unpost", id);
            match Request::post(&url).send().await {
                Ok(resp) => {
                    if resp.status() == 200 {
                        log!("Document unposted successfully");
                        // Перезагрузить документ
                        set_loading.set(true);
                        let reload_url = format!("http://localhost:3000/api/a013/ym-order/{}", id);
                        if let Ok(response) = Request::get(&reload_url).send().await {
                            if response.status() == 200 {
                                if let Ok(text) = response.text().await {
                                    if let Ok(data) =
                                        serde_json::from_str::<YmOrderDetailDto>(&text)
                                    {
                                        set_order.set(Some(data));
                                    }
                                }
                            }
                        }
                        // Перезагрузить проекции (после unpost должны быть пустые)
                        let projections_url =
                            format!("http://localhost:3000/api/a013/ym-order/{}/projections", id);
                        if let Ok(resp) = Request::get(&projections_url).send().await {
                            if resp.status() == 200 {
                                if let Ok(text) = resp.text().await {
                                    if let Ok(proj_data) =
                                        serde_json::from_str::<serde_json::Value>(&text)
                                    {
                                        set_projections.set(Some(proj_data));
                                    }
                                }
                            }
                        }
                        set_loading.set(false);
                    } else {
                        log!("Failed to unpost document: HTTP {}", resp.status());
                    }
                }
                Err(e) => {
                    log!("Failed to unpost document: {:?}", e);
                }
            }
            set_posting.set(false);
        });
    };

    view! {
        <div class="detail-form">
            <div class="detail-form-header">
                <div class="detail-form-header-left">
                    <h2>
                        {move || {
                            order.get()
                                .map(|d| format!("YM Order #{}", d.header.document_no))
                                .unwrap_or_else(|| "Yandex Market Order Details".to_string())
                        }}
                    </h2>
                    <Show when=move || order.get().is_some()>
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
                    <Show when=move || order.get().is_some()>
                        <Show
                            when=move || !is_posted.get()
                            fallback=move || {
                                view! {
                                    <button
                                        class="button button--warning"
                                        on:click=handle_unpost
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
                                        class="button button--primary"
                                        on:click=handle_post
                                        prop:disabled=move || posting.get()
                                    >
                                        {move || if posting.get() { "Проведение..." } else { "✓ Провести" }}
                                    </button>
                                }
                            }
                        </Show>
                    </Show>
                    <button
                        class="button button--secondary"
                        on:click=move |_| on_close.run(())
                    >
                        "Закрыть"
                    </button>
                </div>
            </div>

            <div class="detail-form-content">
                {move || {
                    if loading.get() {
                        view! {
                            <div class="loading-placeholder">
                                <div class="loading-spinner"></div>
                                <p>"Загрузка данных..."</p>
                            </div>
                        }.into_any()
                    } else if let Some(err) = error.get() {
                        view! {
                            <div class="error-message">
                                <strong>"Ошибка: "</strong>{err}
                            </div>
                        }.into_any()
                    } else if let Some(order_data) = order.get() {
                        view! {
                            <div>
                                // Вкладки
                                <div class="detail-tabs">
                                    <button
                                        class=move || if active_tab.get() == "general" { "detail-tab active" } else { "detail-tab" }
                                        on:click=move |_| set_active_tab.set("general")
                                    >
                                        "📋 Общее"
                                    </button>
                                    <button
                                        class=move || if active_tab.get() == "lines" { "detail-tab active" } else { "detail-tab" }
                                        on:click=move |_| set_active_tab.set("lines")
                                    >
                                        "📦 Строки"
                                    </button>
                                    <button
                                        class=move || if active_tab.get() == "campaign" { "detail-tab active" } else { "detail-tab" }
                                        on:click=move |_| set_active_tab.set("campaign")
                                    >
                                        "🏢 Кампания"
                                    </button>
                                    <button
                                        class=move || if active_tab.get() == "json" { "detail-tab active" } else { "detail-tab" }
                                        on:click=move |_| set_active_tab.set("json")
                                    >
                                        "📄 Raw JSON"
                                    </button>
                                    <button
                                        class=move || if active_tab.get() == "projections" { "detail-tab active" } else { "detail-tab" }
                                        on:click=move |_| set_active_tab.set("projections")
                                    >
                                        {move || {
                                            let count = projections.get().as_ref().map(|p| {
                                                let p900_len = p["p900_sales_register"].as_array().map(|a| a.len()).unwrap_or(0);
                                                let p904_len = p["p904_sales_data"].as_array().map(|a| a.len()).unwrap_or(0);
                                                p900_len + p904_len
                                            }).unwrap_or(0);
                                            format!("📊 Проекции ({})", count)
                                        }}
                                    </button>
                                </div>

                                // Контент вкладок
                                <div class="detail-tab-content">
                                    {move || {
                                let tab = active_tab.get();
                                match tab.as_ref() {
                                    "general" => {
                                        // Helper для рендеринга UUID с кнопкой копирования
                                        let conn_id = order_data.header.connection_id.clone();
                                        let org_id = order_data.header.organization_id.clone();
                                        let mp_id = order_data.header.marketplace_id.clone();

                                        view! {
                                            <div class="detail-fields">
                                                <div class="field-row">
                                                    <div class="field-label">"Order №:"</div>
                                                    <div class="field-value">{order_data.header.document_no.clone()}</div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Код:"</div>
                                                    <div class="field-value">{order_data.code.clone()}</div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Описание:"</div>
                                                    <div class="field-value">{order_data.description.clone()}</div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Статус:"</div>
                                                    <div class="field-value">
                                                        <span class="badge badge-success">
                                                            {order_data.state.status_norm.clone()}
                                                        </span>
                                                        {if order_data.is_error {
                                                            view! {
                                                                <span class="badge badge-error" title="Есть строки без сопоставленной номенклатуры">
                                                                    "⚠️ Ошибка сопоставления"
                                                                </span>
                                                            }.into_any()
                                                        } else {
                                                            view! { <span></span> }.into_any()
                                                        }}
                                                    </div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Исходный статус:"</div>
                                                    <div class="field-value">
                                                        <span class="badge badge-info">
                                                            {order_data.state.status_raw.clone()}
                                                        </span>
                                                    </div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Substatus:"</div>
                                                    <div class="field-value">
                                                        {order_data.state.substatus_raw.clone().unwrap_or("—".to_string())}
                                                    </div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Дата изменения статуса:"</div>
                                                    <div class="field-value">{order_data.state.status_changed_at.clone().unwrap_or("—".to_string())}</div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Обновлено в источнике:"</div>
                                                    <div class="field-value">{order_data.state.updated_at_source.clone().unwrap_or("—".to_string())}</div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Дата создания:"</div>
                                                    <div class="field-value field-value-highlight-primary">{order_data.state.creation_date.clone().unwrap_or("—".to_string())}</div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Дата доставки:"</div>
                                                    <div class="field-value field-value-highlight-success">{order_data.state.delivery_date.clone().unwrap_or("—".to_string())}</div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Connection ID:"</div>
                                                    <div class="field-value field-value-uuid">
                                                        <span class="uuid-short" title=conn_id.clone()>{format!("{}...", conn_id.chars().take(8).collect::<String>())}</span>
                                                        <button
                                                            class="button button--ghost button--small"
                                                            on:click=move |_| {
                                                                let uuid_copy = conn_id.clone();
                                                                wasm_bindgen_futures::spawn_local(async move {
                                                                    if let Some(window) = web_sys::window() {
                                                                        let nav = window.navigator().clipboard();
                                                                        let _ = nav.write_text(&uuid_copy);
                                                                    }
                                                                });
                                                            }
                                                            title="Копировать"
                                                        >
                                                            "📋"
                                                        </button>
                                                    </div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Organization ID:"</div>
                                                    <div class="field-value field-value-uuid">
                                                        <span class="uuid-short" title=org_id.clone()>{format!("{}...", org_id.chars().take(8).collect::<String>())}</span>
                                                        <button
                                                            class="button button--ghost button--small"
                                                            on:click=move |_| {
                                                                let uuid_copy = org_id.clone();
                                                                wasm_bindgen_futures::spawn_local(async move {
                                                                    if let Some(window) = web_sys::window() {
                                                                        let nav = window.navigator().clipboard();
                                                                        let _ = nav.write_text(&uuid_copy);
                                                                    }
                                                                });
                                                            }
                                                            title="Копировать"
                                                        >
                                                            "📋"
                                                        </button>
                                                    </div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Marketplace ID:"</div>
                                                    <div class="field-value field-value-uuid">
                                                        <span class="uuid-short" title=mp_id.clone()>{format!("{}...", mp_id.chars().take(8).collect::<String>())}</span>
                                                        <button
                                                            class="button button--ghost button--small"
                                                            on:click=move |_| {
                                                                let uuid_copy = mp_id.clone();
                                                                wasm_bindgen_futures::spawn_local(async move {
                                                                    if let Some(window) = web_sys::window() {
                                                                        let nav = window.navigator().clipboard();
                                                                        let _ = nav.write_text(&uuid_copy);
                                                                    }
                                                                });
                                                            }
                                                            title="Копировать"
                                                        >
                                                            "📋"
                                                        </button>
                                                    </div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Создан:"</div>
                                                    <div class="field-value">{order_data.metadata.created_at.clone()}</div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Обновлен:"</div>
                                                    <div class="field-value">{order_data.metadata.updated_at.clone()}</div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Версия:"</div>
                                                    <div class="field-value">{order_data.metadata.version}</div>
                                                </div>
                                            </div>
                                        }.into_any()
                                    },
                                    "lines" => {
                                        let lines = &order_data.lines;
                                        let total_qty: f64 = lines.iter().map(|l| l.qty).sum();
                                        let total_amount: f64 = lines.iter().filter_map(|l| l.amount_line).sum();
                                        let lines_without_nomenclature = lines.iter().filter(|l| l.nomenclature_ref.is_none()).count();

                                        view! {
                                            <div class="detail-lines">
                                                <div class="lines-summary">
                                                    <div>
                                                        <strong>"Итого: "</strong>
                                                        {format!("{} строк, {} шт., {:.2} руб.",
                                                            lines.len(),
                                                            total_qty,
                                                            total_amount
                                                        )}
                                                    </div>
                                                    {if lines_without_nomenclature > 0 {
                                                        view! {
                                                            <span class="badge badge-error">
                                                                {"⚠️ Без номенклатуры: "}{lines_without_nomenclature}
                                                            </span>
                                                        }.into_any()
                                                    } else {
                                                        view! {
                                                            <span class="badge badge-success">
                                                                "✓ Все строки сопоставлены"
                                                            </span>
                                                        }.into_any()
                                                    }}
                                                </div>

                                                <table class="table__data">
                                                    <thead>
                                                        <tr>
                                                            <th>"Shop SKU"</th>
                                                            <th>"Название"</th>
                                                            <th class="text-right">"Кол-во"</th>
                                                            <th class="text-right">"Цена"</th>
                                                            <th class="text-right">"Сумма"</th>
                                                            <th>"Номенклатура"</th>
                                                            <th class="text-center">"Статус"</th>
                                                        </tr>
                                                    </thead>
                                                    <tbody>
                                                        {lines.iter().map(|line| {
                                                            let has_nomenclature = line.nomenclature_ref.is_some();
                                                            let row_class = if !has_nomenclature {
                                                                "row-warning"
                                                            } else {
                                                                ""
                                                            };
                                                            let line_id = line.line_id.clone();
                                                            let nom_ref = line.nomenclature_ref.clone();

                                                            view! {
                                                                <tr class={row_class}>
                                                                    <td>
                                                                        <code>{line.shop_sku.clone()}</code>
                                                                    </td>
                                                                    <td class="cell-truncate" title={line.name.clone()}>
                                                                        {line.name.clone()}
                                                                    </td>
                                                                    <td class="text-right">
                                                                        <strong>{format!("{:.0}", line.qty)}</strong>
                                                                    </td>
                                                                    <td class="text-right">
                                                                        {line.price_effective.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}
                                                                    </td>
                                                                    <td class="text-right" style="font-weight: bold; color: #2e7d32;">
                                                                        {line.amount_line.map(|a| format!("{:.2}", a)).unwrap_or("—".to_string())}
                                                                    </td>
                                                                    <td>
                                                                        {move || {
                                                                            let nom_info_map = nomenclatures_info.get();
                                                                            if let Some(info) = nom_info_map.get(&line_id) {
                                                                                let nom_ref_for_click = nom_ref.clone().unwrap_or_default();
                                                                                view! {
                                                                                    <div
                                                                                        style="color: #1976d2; cursor: pointer; text-decoration: underline;"
                                                                                        on:click=move |_| {
                                                                                            set_selected_nomenclature_id.set(Some(nom_ref_for_click.clone()));
                                                                                        }
                                                                                    >
                                                                                        {info.description.clone()}
                                                                                    </div>
                                                                                }.into_any()
                                                                            } else if has_nomenclature {
                                                                                view! {
                                                                                    <span style="color: #757575;">"Загрузка..."</span>
                                                                                }.into_any()
                                                                            } else {
                                                                                view! {
                                                                                    <span class="text-warning">"⚠️ Не задано"</span>
                                                                                }.into_any()
                                                                            }
                                                                        }}
                                                                    </td>
                                                                    <td class="text-center">
                                                                        {
                                                                            let status_str = line.status.clone().unwrap_or("—".to_string());
                                                                            let badge_class = match status_str.as_str() {
                                                                                "DELIVERED" => "badge badge-success",
                                                                                "CANCELLED" => "badge badge-error",
                                                                                "PROCESSING" => "badge badge-info",
                                                                                "—" => "",
                                                                                _ => "badge badge-secondary",
                                                                            };
                                                                            if badge_class.is_empty() {
                                                                                view! { <span>"—"</span> }.into_any()
                                                                            } else {
                                                                                view! { <span class={badge_class}>{status_str}</span> }.into_any()
                                                                            }
                                                                        }
                                                                    </td>
                                                                </tr>
                                                            }
                                                        }).collect_view()}
                                                        <tr class="totals-row">
                                                            <td colspan="2" class="text-right"><strong>"Итого:"</strong></td>
                                                            <td class="text-right"><strong>{format!("{:.0}", total_qty)}</strong></td>
                                                            <td></td>
                                                            <td class="text-right" style="color: #2e7d32;"><strong>{format!("{:.2}", total_amount)}</strong></td>
                                                            <td colspan="2"></td>
                                                        </tr>
                                                    </tbody>
                                                </table>
                                            </div>
                                        }.into_any()
                                    },
                                    "campaign" => {
                                        let campaign_id = order_data.header.campaign_id.clone();
                                        let subsidies_display = order_data.header.subsidies_json.clone()
                                            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                                            .map(|v| serde_json::to_string_pretty(&v).unwrap_or("—".to_string()))
                                            .unwrap_or("—".to_string());

                                        view! {
                                            <div class="detail-fields">
                                                <div class="field-row">
                                                    <div class="field-label">"Campaign ID:"</div>
                                                    <div class="field-value field-value-uuid">
                                                        <span class="uuid-short" title=campaign_id.clone()>{campaign_id.clone()}</span>
                                                        <button
                                                            class="button button--ghost button--small"
                                                            on:click=move |_| {
                                                                let id_copy = campaign_id.clone();
                                                                wasm_bindgen_futures::spawn_local(async move {
                                                                    if let Some(window) = web_sys::window() {
                                                                        let nav = window.navigator().clipboard();
                                                                        let _ = nav.write_text(&id_copy);
                                                                    }
                                                                });
                                                            }
                                                            title="Copy to clipboard"
                                                        >
                                                            "📋"
                                                        </button>
                                                    </div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Маркетплейс:"</div>
                                                    <div class="field-value">
                                                        <span class="badge badge-warning">
                                                            "Yandex Market"
                                                        </span>
                                                    </div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Загружено:"</div>
                                                    <div class="field-value">{order_data.source_meta.fetched_at.clone()}</div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Версия документа:"</div>
                                                    <div class="field-value">{order_data.source_meta.document_version}</div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Сумма заказа (API):"</div>
                                                    <div class="field-value field-value-highlight-success" style="font-size: 16px;">
                                                        {order_data.header.total_amount.map(|a| format!("{:.2}", a)).unwrap_or("—".to_string())}
                                                        {order_data.header.currency.as_ref().map(|c| format!(" {}", c)).unwrap_or_default()}
                                                    </div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Сумма товаров:"</div>
                                                    <div class="field-value field-value-highlight-primary">
                                                        {order_data.header.items_total.map(|a| format!("{:.2}", a)).unwrap_or("—".to_string())}
                                                        {order_data.header.currency.as_ref().map(|c| format!(" {}", c)).unwrap_or_default()}
                                                    </div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Доставка:"</div>
                                                    <div class="field-value" style="color: #7b1fa2;">
                                                        {order_data.header.delivery_total.map(|a| format!("{:.2}", a)).unwrap_or("—".to_string())}
                                                        {order_data.header.currency.as_ref().map(|c| format!(" {}", c)).unwrap_or_default()}
                                                    </div>
                                                </div>

                                                <div class="field-row">
                                                    <div class="field-label">"Субсидии от Маркета:"</div>
                                                    <div class="field-value">
                                                        <pre class="json-preview">{subsidies_display}</pre>
                                                    </div>
                                                </div>
                                            </div>
                                        }.into_any()
                                    },
                                    "json" => view! {
                                        <div class="detail-raw-json">
                                            <div class="raw-json-header">
                                                <strong>"Raw JSON от Yandex Market API:"</strong>
                                            </div>
                                            {move || {
                                                if let Some(json) = raw_json_from_ym.get() {
                                                    view! {
                                                        <pre class="raw-json-content">
                                                            {json}
                                                        </pre>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div class="loading-placeholder">
                                                            "Загрузка Raw JSON от Yandex Market..."
                                                        </div>
                                                    }.into_any()
                                                }
                                            }}
                                        </div>
                                    }.into_any(),
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
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);" title="price_list">"Price List"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);" title="customer_in (buyer_price/amount_line)">"Cust In"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);" title="customer_out">"Cust Out"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);" title="commission_out">"Comm Out"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);" title="acquiring_out">"Acq Out"</th>
                                                                                        <th style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);" title="total">"Total"</th>
                                                                                    </tr>
                                                                                </thead>
                                                                                <tbody>
                                                                                    {p904_items.iter().map(|item| {
                                                                                        let article = item["article"].as_str().unwrap_or("—");
                                                                                        let price_list = item["price_list"].as_f64().unwrap_or(0.0);
                                                                                        let customer_in = item["customer_in"].as_f64().unwrap_or(0.0);
                                                                                        let customer_out = item["customer_out"].as_f64().unwrap_or(0.0);
                                                                                        let commission_out = item["commission_out"].as_f64().unwrap_or(0.0);
                                                                                        let acquiring_out = item["acquiring_out"].as_f64().unwrap_or(0.0);
                                                                                        let total = item["total"].as_f64().unwrap_or(0.0);

                                                                                        view! {
                                                                                            <tr style="border-bottom: 1px solid var(--color-border-light);">
                                                                                                <td class="field-value-mono" style="padding: var(--space-2xs) var(--space-xs); border: 1px solid var(--color-border);">{article}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", price_list)}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border); color: var(--color-success); font-weight: var(--font-weight-semibold);">{format!("{:.2}", customer_in)}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", customer_out)}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", commission_out)}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border);">{format!("{:.2}", acquiring_out)}</td>
                                                                                                <td style="padding: var(--space-2xs) var(--space-xs); text-align: right; border: 1px solid var(--color-border); font-weight: var(--font-weight-bold);">{format!("{:.2}", total)}</td>
                                                                                            </tr>
                                                                                        }
                                                                                    }).collect::<Vec<_>>()}
                                                                                </tbody>
                                                                            </table>
                                                                        </div>
                                                                    }.into_any()
                                                                } else {
                                                                    view! {
                                                                        <p style="text-align: center; padding: var(--space-sm); color: var(--color-text-tertiary); font-size: var(--font-size-sm);">"Нет записей (проекции создаются только для документов со статусом DELIVERED)"</p>
                                                                    }.into_any()
                                                                }}
                                                            </div>
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div style="padding: var(--space-lg); text-align: center; color: var(--color-text-tertiary); font-size: var(--font-size-sm);">
                                                            "Проекции не загружены"
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

            // Modals for linked aggregates
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
                                on_saved=std::rc::Rc::new({
                                    let on_close_modal = on_close_modal.clone();
                                    move |_| on_close_modal()
                                })
                                on_cancel=std::rc::Rc::new(move |_| on_close_modal())
                            />
                        </div>
                    </div>
                }
            })}
        </div>
    }
}
