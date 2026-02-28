use super::{
    get_dom_snapshot,
    tree_view::TreeView,
    validator::{validate_pages, Severity, ValidationReport},
    DomNode,
};
use crate::shared::icons::icon;
use leptos::prelude::*;
use wasm_bindgen::JsCast;

// ── Tab enum ─────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
enum InspectorTab {
    Tree,
    Validation,
}

// ── Component ─────────────────────────────────────────────────────────────────

#[component]
pub fn DomValidatorPage() -> impl IntoView {
    let (tree, set_tree) = signal::<Option<DomNode>>(get_dom_snapshot());
    let (report, set_report) = signal::<Option<ValidationReport>>(None);
    let (active_tab, set_active_tab) = signal(InspectorTab::Tree);

    Effect::new(move |_| {
        set_tree.set(get_dom_snapshot());
    });

    let refresh = move |_| {
        if let Some(new_tree) = super::tree_builder::build_dom_tree() {
            super::set_dom_snapshot(&new_tree);
            set_tree.set(Some(new_tree));
        }
    };

    let run_validation = move |_| {
        let result = validate_pages();
        set_report.set(Some(result));
        set_active_tab.set(InspectorTab::Validation);
    };

    let export_to_json = move |_| {
        if let Some(tree_data) = tree.get() {
            if let Ok(json) = serde_json::to_string_pretty(&tree_data) {
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        if let Ok(elem) = document.create_element("a") {
                            if let Ok(element) = elem.dyn_into::<web_sys::HtmlAnchorElement>() {
                                let blob_parts = js_sys::Array::new();
                                blob_parts.push(&wasm_bindgen::JsValue::from_str(&json));
                                if let Ok(blob) =
                                    web_sys::Blob::new_with_str_sequence(&blob_parts)
                                {
                                    if let Ok(url) =
                                        web_sys::Url::create_object_url_with_blob(&blob)
                                    {
                                        element.set_href(&url);
                                        element.set_download("dom_snapshot.json");
                                        let _ = element.click();
                                        let _ = web_sys::Url::revoke_object_url(&url);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    view! {
        <div class="page" id="dom_inspector--system" data-page-category="system">
            <div class="page__header">
                <div class="page__header-left">
                    <h1 class="page__title">"DOM Inspector"</h1>
                </div>
                <div class="page__header-right">
                    <button class="button button--secondary" on:click=refresh>
                        {icon("refresh-cw")}
                        "Обновить снимок"
                    </button>
                    <button
                        class="button button--primary"
                        on:click=run_validation
                    >
                        {icon("check-circle")}
                        "Проверить стандарт"
                    </button>
                    <button
                        class="button button--secondary"
                        on:click=export_to_json
                        disabled=move || tree.get().is_none()
                    >
                        {icon("download")}
                        "Экспорт JSON"
                    </button>
                </div>
            </div>

            // Tab bar
            <div class="page__tabs">
                <button
                    class="page__tab"
                    class:page__tab--active=move || active_tab.get() == InspectorTab::Tree
                    on:click=move |_| set_active_tab.set(InspectorTab::Tree)
                >
                    "DOM Tree"
                </button>
                <button
                    class="page__tab"
                    class:page__tab--active=move || active_tab.get() == InspectorTab::Validation
                    on:click=move |_| set_active_tab.set(InspectorTab::Validation)
                >
                    "Проверка стандарта"
                    {move || report.get().map(|r| {
                        let errs = r.error_count();
                        let warns = r.warning_count();
                        if errs > 0 {
                            view! { <span class="dom-validator-badge dom-validator-badge--error">{errs}</span> }.into_any()
                        } else if warns > 0 {
                            view! { <span class="dom-validator-badge dom-validator-badge--warning">{warns}</span> }.into_any()
                        } else {
                            view! { <span class="dom-validator-badge dom-validator-badge--ok">"✓"</span> }.into_any()
                        }
                    })}
                </button>
            </div>

            <div class="page__content">
                {move || match active_tab.get() {
                    InspectorTab::Tree => view! {
                        <div class="dom-validator-content">
                            {move || match tree.get() {
                                Some(node) => view! { <TreeView node=node /> }.into_any(),
                                None => view! {
                                    <div class="dom-validator-placeholder">
                                        <p>"Снимок DOM не найден. Нажмите «Обновить снимок»."</p>
                                    </div>
                                }.into_any()
                            }}
                        </div>
                    }.into_any(),
                    InspectorTab::Validation => view! {
                        <div class="dom-validator-content">
                            {move || match report.get() {
                                None => view! {
                                    <div class="dom-validator-placeholder">
                                        <p>"Нажмите «Проверить стандарт» для запуска проверки."</p>
                                    </div>
                                }.into_any(),
                                Some(r) => view! { <ValidationReportView report=r /> }.into_any()
                            }}
                        </div>
                    }.into_any(),
                }}
            </div>
        </div>
    }
}

// ── Validation report view ────────────────────────────────────────────────────

#[component]
fn ValidationReportView(report: ValidationReport) -> impl IntoView {
    let errors = report.error_count();
    let warnings = report.warning_count();
    let summary_class = if errors > 0 {
        "dom-validator-summary dom-validator-summary--error"
    } else if warnings > 0 {
        "dom-validator-summary dom-validator-summary--warning"
    } else {
        "dom-validator-summary dom-validator-summary--ok"
    };

    view! {
        <div class="dom-validator-report">
            // Summary row
            <div class=summary_class>
                <span class="dom-validator-summary__stat">
                    {format!("Табов: {}", report.total_tabs)}
                </span>
                <span class="dom-validator-summary__stat dom-validator-summary__stat--ok">
                    {format!("✓ OK: {}", report.ok_count)}
                </span>
                <span class="dom-validator-summary__stat dom-validator-summary__stat--legacy">
                    {format!("⏳ Legacy: {}", report.legacy_count)}
                </span>
                {(errors > 0).then(|| view! {
                    <span class="dom-validator-summary__stat dom-validator-summary__stat--error">
                        {format!("✗ Ошибки: {}", errors)}
                    </span>
                })}
                {(warnings > 0).then(|| view! {
                    <span class="dom-validator-summary__stat dom-validator-summary__stat--warning">
                        {format!("⚠ Предупреждения: {}", warnings)}
                    </span>
                })}
            </div>

            // Issue list
            {if report.issues.is_empty() {
                view! {
                    <div class="dom-validator-placeholder">
                        <p>"Все страницы соответствуют стандарту."</p>
                    </div>
                }.into_any()
            } else {
                view! {
                    <table class="dom-validator-table">
                        <thead>
                            <tr>
                                <th>"Уровень"</th>
                                <th>"Таб"</th>
                                <th>"Сообщение"</th>
                            </tr>
                        </thead>
                        <tbody>
                            {report.issues.iter().map(|issue| {
                                let (row_class, badge_class, label) = match issue.severity {
                                    Severity::Error => ("dom-validator-row--error", "dom-validator-badge--error", "Ошибка"),
                                    Severity::Warning => ("dom-validator-row--warning", "dom-validator-badge--warning", "Предупреждение"),
                                    Severity::Info => ("dom-validator-row--info", "dom-validator-badge--info", "Инфо"),
                                };
                                view! {
                                    <tr class=row_class>
                                        <td>
                                            <span class=format!("dom-validator-badge {}", badge_class)>
                                                {label}
                                            </span>
                                        </td>
                                        <td class="dom-validator-tab-key">{issue.tab_key.clone()}</td>
                                        <td>{issue.message.clone()}</td>
                                    </tr>
                                }
                            }).collect_view()}
                        </tbody>
                    </table>
                }.into_any()
            }}
        </div>
    }
}
