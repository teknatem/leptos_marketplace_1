/// Универсальные утилиты для работы со списками (поиск, сортировка, UI компоненты)
use leptos::prelude::*;
use leptos::ev::MouseEvent;
use std::cmp::Ordering;
use wasm_bindgen::JsCast;

/// Trait для типов данных, поддерживающих поиск
pub trait Searchable {
    /// Проверяет, соответствует ли объект поисковому запросу
    fn matches_filter(&self, filter: &str) -> bool;

    /// Возвращает значение указанного поля для подсветки
    fn get_field_value(&self, field: &str) -> Option<String>;
}

/// Trait для типов данных, поддерживающих сортировку
pub trait Sortable {
    /// Сравнивает два объекта по указанному полю
    fn compare_by_field(&self, other: &Self, field: &str) -> Ordering;
}

/// Подсветка совпадений в тексте (case-insensitive)
pub fn highlight_matches(text: &str, filter: &str) -> AnyView {
    if filter.trim().is_empty() || filter.trim().len() < 3 {
        return view! { <span>{text}</span> }.into_any();
    }

    let filter_lower = filter.to_lowercase();
    let text_lower = text.to_lowercase();

    // Если нет совпадений, возвращаем текст как есть
    if !text_lower.contains(&filter_lower) {
        return view! { <span>{text}</span> }.into_any();
    }

    // Находим все совпадения
    let mut parts: Vec<AnyView> = Vec::new();
    let mut last_pos = 0;

    while let Some(pos) = text_lower[last_pos..].find(&filter_lower) {
        let actual_pos = last_pos + pos;

        // Добавляем текст до совпадения
        if actual_pos > last_pos {
            parts.push(view! { <span>{&text[last_pos..actual_pos]}</span> }.into_any());
        }

        // Добавляем подсвеченное совпадение
        let match_end = actual_pos + filter_lower.len();
        parts.push(view! {
            <span style="background-color: #ff9800; color: white; padding: 1px 2px; border-radius: 2px; font-weight: 500;">
                {&text[actual_pos..match_end]}
            </span>
        }.into_any());

        last_pos = match_end;
    }

    // Добавляем оставшийся текст
    if last_pos < text.len() {
        parts.push(view! { <span>{&text[last_pos..]}</span> }.into_any());
    }

    view! { <>{parts}</> }.into_any()
}

/// Сортирует список по указанному полю
pub fn sort_list<T: Sortable>(items: &mut Vec<T>, field: &str, ascending: bool) {
    items.sort_by(|a, b| {
        let cmp = a.compare_by_field(b, field);
        if ascending { cmp } else { cmp.reverse() }
    });
}

/// Фильтрует список по поисковому запросу
pub fn filter_list<T: Searchable + Clone>(items: Vec<T>, filter: &str) -> Vec<T> {
    if filter.trim().is_empty() || filter.trim().len() < 3 {
        return items;
    }

    items.into_iter()
        .filter(|item| item.matches_filter(filter))
        .collect()
}

/// Компонент поиска с debounce и кнопкой очистки
#[component]
pub fn SearchInput(
    /// Текущее значение фильтра (для отображения)
    #[prop(into)]
    value: Signal<String>,
    /// Callback для обновления значения фильтра
    #[prop(into)]
    on_change: Callback<String>,
    /// Placeholder текст
    #[prop(optional, into)]
    placeholder: String,
) -> impl IntoView {
    let placeholder = if placeholder.is_empty() {
        "Поиск (мин. 3 символа)...".to_string()
    } else {
        placeholder
    };

    // Локальное состояние для input (до debounce)
    let (input_value, set_input_value) = signal(String::new());

    // Debounce механизм
    let debounce_timeout = leptos::prelude::StoredValue::new(None::<i32>);

    let handle_input_change = move |new_value: String| {
        set_input_value.set(new_value.clone());

        // Отменяем предыдущий таймер если есть
        if let Some(timeout_id) = debounce_timeout.get_value() {
            web_sys::window()
                .and_then(|w| Some(w.clear_timeout_with_handle(timeout_id)));
        }

        // Создаем новый таймер
        let window = web_sys::window().expect("no window");
        let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
            on_change.run(new_value.clone());
        }) as Box<dyn Fn()>);

        let timeout_id = window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                closure.as_ref().unchecked_ref::<js_sys::Function>(),
                300, // 300ms задержка
            )
            .expect("setTimeout failed");

        closure.forget();
        debounce_timeout.set_value(Some(timeout_id));
    };

    let is_filter_active = move || {
        let text = value.get();
        !text.trim().is_empty() && text.trim().len() >= 3
    };

    let clear_filter = move |_| {
        set_input_value.set(String::new());
        on_change.run(String::new());
    };

    view! {
        <div style="position: relative; display: inline-flex; align-items: center;">
            <input
                type="text"
                placeholder={placeholder}
                style=move || format!(
                    "width: 250px; padding: 6px 32px 6px 10px; border: 1px solid #ddd; border-radius: 4px; font-size: 15px; background: {};",
                    if is_filter_active() { "#fffbea" } else { "white" }
                )
                prop:value=move || input_value.get()
                on:input=move |ev| {
                    let val = event_target_value(&ev);
                    handle_input_change(val);
                }
            />
            {move || if !input_value.get().is_empty() {
                view! {
                    <button
                        style="position: absolute; right: 6px; background: none; border: none; cursor: pointer; padding: 4px; display: inline-flex; align-items: center; color: #666; line-height: 1;"
                        on:click=clear_filter
                        title="Очистить"
                    >
                        {crate::shared::icons::icon("x")}
                    </button>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }}
        </div>
    }
}

/// Получить индикатор сортировки для заголовка
pub fn get_sort_indicator(current_field: &str, field: &str, ascending: bool) -> &'static str {
    if current_field == field {
        if ascending { " ▲" } else { " ▼" }
    } else {
        " ⇅"
    }
}

/// Создать обработчик переключения сортировки
pub fn create_sort_toggle(
    field: &'static str,
    sort_field: Signal<String>,
    set_sort_field: WriteSignal<String>,
    set_sort_ascending: WriteSignal<bool>,
) -> impl Fn(MouseEvent) + 'static {
    move |_| {
        if sort_field.get() == field {
            set_sort_ascending.update(|v| *v = !*v);
        } else {
            set_sort_field.set(field.to_string());
            set_sort_ascending.set(true);
        }
    }
}
