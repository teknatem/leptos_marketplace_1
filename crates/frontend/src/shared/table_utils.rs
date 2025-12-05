//! Утилиты для таблиц: изменение ширины колонок с сохранением в localStorage.
//!
//! # Использование
//!
//! ```rust
//! use crate::shared::table_utils::init_column_resize;
//!
//! // В компоненте списка
//! Effect::new(move |_| {
//!     init_column_resize("my-table-id", "my_feature_column_widths");
//! });
//! ```
//!
//! В HTML таблицы:
//! ```html
//! <table id="my-table-id">
//!     <thead>
//!         <tr>
//!             <th class="resizable">Колонка 1</th>
//!             <th class="resizable">Колонка 2</th>
//!         </tr>
//!     </thead>
//! </table>
//! ```

use leptos::task::spawn_local;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{HtmlElement, MouseEvent as WebMouseEvent};

/// Проверяет, было ли только что изменение ширины колонки.
/// Используется для блокировки клика сортировки сразу после resize.
pub fn was_just_resizing() -> bool {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.body())
        .map(|b| b.get_attribute("data-was-resizing").as_deref() == Some("true"))
        .unwrap_or(false)
}

/// Очищает флаг resize.
pub fn clear_resize_flag() {
    if let Some(body) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.body())
    {
        let _ = body.remove_attribute("data-was-resizing");
    }
}

/// Сохраняет ширины колонок в localStorage.
///
/// # Аргументы
/// * `table_id` - ID таблицы в DOM
/// * `storage_key` - Ключ для localStorage (должен быть уникальным для каждого списка)
pub fn save_column_widths(table_id: &str, storage_key: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(storage) = window.local_storage().ok().flatten() else {
        return;
    };
    let Some(table) = document.get_element_by_id(table_id) else {
        return;
    };

    let headers = table.query_selector_all("th.resizable").ok();
    let Some(headers) = headers else { return };

    let mut widths: Vec<i32> = Vec::new();
    for i in 0..headers.length() {
        if let Some(th) = headers.get(i) {
            if let Ok(th) = th.dyn_into::<HtmlElement>() {
                widths.push(th.offset_width());
            }
        }
    }

    if let Ok(json) = serde_json::to_string(&widths) {
        let _ = storage.set_item(storage_key, &json);
    }
}

/// Восстанавливает ширины колонок из localStorage.
///
/// # Аргументы
/// * `table_id` - ID таблицы в DOM
/// * `storage_key` - Ключ для localStorage
pub fn restore_column_widths(table_id: &str, storage_key: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(storage) = window.local_storage().ok().flatten() else {
        return;
    };
    let Some(table) = document.get_element_by_id(table_id) else {
        return;
    };

    let Some(json) = storage.get_item(storage_key).ok().flatten() else {
        return;
    };
    let Ok(widths): Result<Vec<i32>, _> = serde_json::from_str(&json) else {
        return;
    };

    let headers = table.query_selector_all("th.resizable").ok();
    let Some(headers) = headers else { return };

    for (i, width) in widths.iter().enumerate() {
        if let Some(th) = headers.get(i as u32) {
            if let Ok(th) = th.dyn_into::<HtmlElement>() {
                let _ = th.style().set_property("width", &format!("{}px", width));
                let _ = th
                    .style()
                    .set_property("min-width", &format!("{}px", width));
            }
        }
    }
}

/// Инициализирует изменение ширины для всех колонок с классом "resizable".
///
/// Добавляет resize-handle к каждому заголовку и обрабатывает события мыши.
/// Ширины сохраняются в localStorage и восстанавливаются при следующем открытии.
///
/// # Аргументы
/// * `table_id` - ID таблицы в DOM
/// * `storage_key` - Ключ для localStorage (например, "a012_wb_sales_column_widths")
///
/// # Пример
/// ```rust
/// Effect::new(move |_| {
///     init_column_resize("wb-sales-table", "a012_wb_sales_column_widths");
/// });
/// ```
pub fn init_column_resize(table_id: &str, storage_key: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    let Some(document) = window.document() else {
        return;
    };
    let Some(table) = document.get_element_by_id(table_id) else {
        return;
    };

    // First restore saved widths
    restore_column_widths(table_id, storage_key);

    let headers = table.query_selector_all("th.resizable").ok();
    let Some(headers) = headers else { return };

    let table_id_owned = table_id.to_string();
    let storage_key_owned = storage_key.to_string();

    for i in 0..headers.length() {
        let Some(th) = headers.get(i) else { continue };
        let Ok(th) = th.dyn_into::<HtmlElement>() else {
            continue;
        };

        // Skip if already has resize handle
        if th.query_selector(".resize-handle").ok().flatten().is_some() {
            continue;
        }

        // Create resize handle
        let Ok(handle) = document.create_element("div") else {
            continue;
        };
        handle.set_class_name("resize-handle");

        // State for this column
        let resizing = Rc::new(RefCell::new(false));
        let did_resize = Rc::new(RefCell::new(false));
        let start_x = Rc::new(RefCell::new(0i32));
        let start_width = Rc::new(RefCell::new(0i32));
        let th_ref = Rc::new(RefCell::new(th.clone()));
        let table_id_for_save = table_id_owned.clone();
        let storage_key_for_save = storage_key_owned.clone();

        // Mousedown on handle
        let resizing_md = resizing.clone();
        let did_resize_md = did_resize.clone();
        let start_x_md = start_x.clone();
        let start_width_md = start_width.clone();
        let th_md = th_ref.clone();

        let mousedown = Closure::wrap(Box::new(move |e: WebMouseEvent| {
            e.prevent_default();
            e.stop_propagation();
            *resizing_md.borrow_mut() = true;
            *did_resize_md.borrow_mut() = false;
            *start_x_md.borrow_mut() = e.client_x();
            *start_width_md.borrow_mut() = th_md.borrow().offset_width();

            if let Some(body) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.body())
            {
                let _ = body.class_list().add_1("resizing-column");
            }
        }) as Box<dyn FnMut(WebMouseEvent)>);

        let _ = handle
            .add_event_listener_with_callback("mousedown", mousedown.as_ref().unchecked_ref());
        mousedown.forget();

        // Mousemove on document
        let resizing_mm = resizing.clone();
        let did_resize_mm = did_resize.clone();
        let start_x_mm = start_x.clone();
        let start_width_mm = start_width.clone();
        let th_mm = th_ref.clone();

        let mousemove = Closure::wrap(Box::new(move |e: WebMouseEvent| {
            if !*resizing_mm.borrow() {
                return;
            }
            *did_resize_mm.borrow_mut() = true;
            let diff = e.client_x() - *start_x_mm.borrow();
            let new_width = (*start_width_mm.borrow() + diff).max(40);
            let _ = th_mm
                .borrow()
                .style()
                .set_property("width", &format!("{}px", new_width));
            let _ = th_mm
                .borrow()
                .style()
                .set_property("min-width", &format!("{}px", new_width));
        }) as Box<dyn FnMut(WebMouseEvent)>);

        let _ = document
            .add_event_listener_with_callback("mousemove", mousemove.as_ref().unchecked_ref());
        mousemove.forget();

        // Mouseup on document
        let resizing_mu = resizing.clone();
        let did_resize_mu = did_resize.clone();
        let table_id_mu = table_id_for_save.clone();
        let storage_key_mu = storage_key_for_save.clone();

        let mouseup = Closure::wrap(Box::new(move |_: WebMouseEvent| {
            if !*resizing_mu.borrow() {
                return;
            }
            let was_resizing = *did_resize_mu.borrow();
            *resizing_mu.borrow_mut() = false;
            *did_resize_mu.borrow_mut() = false;

            if let Some(body) = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.body())
            {
                let _ = body.class_list().remove_1("resizing-column");
                if was_resizing {
                    // Save column widths to localStorage
                    save_column_widths(&table_id_mu, &storage_key_mu);
                    let _ = body.set_attribute("data-was-resizing", "true");
                    spawn_local(async {
                        gloo_timers::future::TimeoutFuture::new(50).await;
                        clear_resize_flag();
                    });
                }
            }
        }) as Box<dyn FnMut(WebMouseEvent)>);

        let _ =
            document.add_event_listener_with_callback("mouseup", mouseup.as_ref().unchecked_ref());
        mouseup.forget();

        let _ = th.append_child(&handle);
    }
}

