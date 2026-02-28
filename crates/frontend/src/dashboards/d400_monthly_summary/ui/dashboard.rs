use crate::dashboards::d400_monthly_summary::api;
use chrono::{Datelike, Utc};
use contracts::dashboards::d400_monthly_summary::MonthlySummaryResponse;
use js_sys::{Array, Function, Reflect};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Serialize;
use serde_wasm_bindgen::Serializer;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::HtmlIFrameElement;

/// Monthly Summary Dashboard component
#[component]
pub fn MonthlySummaryDashboard() -> impl IntoView {
    // Current date for default month selection
    let now = Utc::now().date_naive();
    let (selected_year, set_selected_year) = signal(now.year());
    let (selected_month, set_selected_month) = signal(now.month());
    let (available_periods, set_available_periods) = signal(Vec::<String>::new());
    let initial_period_set = StoredValue::new(false);

    // Data state
    let (data, set_data) = signal(None::<MonthlySummaryResponse>);
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);

    // Iframe state (HtmlIFrameElement is not Send+Sync, store locally)
    let iframe_element = StoredValue::new_local(None::<HtmlIFrameElement>);
    let (iframe_loaded, set_iframe_loaded) = signal(false);

    // Load available periods on mount
    Effect::new(move |_| {
        spawn_local(async move {
            match api::get_available_periods().await {
                Ok(periods) => {
                    if !periods.is_empty() && !initial_period_set.get_value() {
                        if let Some((year, month)) = parse_period(&periods[0]) {
                            set_selected_year.set(year);
                            set_selected_month.set(month);
                            initial_period_set.set_value(true);
                        }
                    }
                    set_available_periods.set(periods);
                }
                Err(err) => {
                    log::error!("Failed to load D400 periods: {}", err);
                }
            }
        });
    });

    // Load data when period changes
    Effect::new(move |_| {
        let year = selected_year.get();
        let month = selected_month.get();
        set_loading.set(true);
        set_error.set(None);

        spawn_local(async move {
            match api::get_monthly_summary(year, month).await {
                Ok(response) => {
                    set_data.set(Some(response));
                    set_loading.set(false);
                }
                Err(e) => {
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    });

    // Handle period change requests from iframe
    Effect::new(move |_| {
        let window = match web_sys::window() {
            Some(window) => window,
            None => return,
        };

        let handler = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
            let data = event.data();
            let Ok(msg_type) = Reflect::get(&data, &JsValue::from_str("type")) else {
                return;
            };

            if msg_type.as_string().as_deref() != Some("d400_period_change") {
                return;
            }

            let Ok(period_value) = Reflect::get(&data, &JsValue::from_str("period")) else {
                return;
            };
            let Some(period) = period_value.as_string() else {
                return;
            };

            if let Some((year, month)) = parse_period(&period) {
                set_selected_year.set(year);
                set_selected_month.set(month);
            }
        }) as Box<dyn FnMut(_)>);

        let _ =
            window.add_event_listener_with_callback("message", handler.as_ref().unchecked_ref());
        handler.forget();
    });

    // Render dashboard into iframe when data or iframe changes
    Effect::new(move |_| {
        let current_data = data.get();
        let is_loaded = iframe_loaded.get();

        let Some(current_data) = current_data else {
            return;
        };
        if !is_loaded {
            return;
        };
        let Some(iframe) = iframe_element.get_value() else {
            return;
        };

        let periods = available_periods.get();
        if let Err(err) = render_dashboard_in_iframe(&iframe, &current_data, &periods) {
            log::error!("Failed to render D400 iframe dashboard: {:?}", err);
        }
    });

    view! {
        <div id="d400_monthly_summary--dashboard" data-page-category="legacy" class="d400-dashboard">
            {move || {
                if loading.get() {
                    view! {
                        <div class="d400-loading">
                            <span>"Загрузка данных..."</span>
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            {move || {
                if let Some(err) = error.get() {
                    view! {
                        <div class="d400-error">
                            <strong>"⚠ Ошибка: "</strong>
                            {err}
                        </div>
                    }.into_any()
                } else {
                    view! { <></> }.into_any()
                }
            }}

            <iframe
                src="assets/dashboards/d400/dashboard.html"
                style="width: 100%; height: 900px; border: none;"
                on:load=move |ev| {
                    let iframe = ev
                        .target()
                        .and_then(|t| t.dyn_into::<HtmlIFrameElement>().ok());
                    iframe_element.set_value(iframe);
                    set_iframe_loaded.set(true);
                }
            ></iframe>
        </div>
    }
}

fn render_dashboard_in_iframe(
    iframe: &HtmlIFrameElement,
    data: &MonthlySummaryResponse,
    available_periods: &[String],
) -> Result<(), JsValue> {
    let window = iframe
        .content_window()
        .ok_or_else(|| JsValue::from_str("Iframe window not available"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("Iframe document not available"))?;
    let container = document
        .get_element_by_id("bolt-root")
        .ok_or_else(|| JsValue::from_str("bolt-root element not found"))?;

    let render_value = Reflect::get(&window, &JsValue::from_str("render"))?;
    if !render_value.is_function() {
        return Err(JsValue::from_str("render is not a function"));
    }
    let render_fn: Function = render_value.dyn_into()?;
    let data_value = data
        .serialize(&Serializer::json_compatible())
        .map_err(|err| JsValue::from_str(&err.to_string()))?;

    let options = js_sys::Object::new();
    let periods_array = Array::new();
    for period in available_periods {
        periods_array.push(&JsValue::from_str(period));
    }

    let on_period_change = Function::new_with_args(
        "period",
        "window.parent.postMessage({type:'d400_period_change', period: period}, '*');",
    );

    Reflect::set(
        &options,
        &JsValue::from_str("availablePeriods"),
        &periods_array,
    )?;
    Reflect::set(
        &options,
        &JsValue::from_str("onPeriodChange"),
        &on_period_change,
    )?;

    render_fn.call3(&window, &container.into(), &data_value, &options)?;
    Ok(())
}

fn parse_period(period: &str) -> Option<(i32, u32)> {
    let mut parts = period.split('-');
    let year = parts.next()?.parse::<i32>().ok()?;
    let month = parts.next()?.parse::<u32>().ok()?;
    if (1..=12).contains(&month) {
        Some((year, month))
    } else {
        None
    }
}
