use leptos::prelude::*;
use wasm_bindgen::JsCast;

/// ThemeSelect component for switching themes
#[component]
pub fn ThemeSelect() -> impl IntoView {
    let current_theme = RwSignal::new("dark".to_string());
    let is_open = RwSignal::new(false);

    let change_theme = move |theme: String| {
        // Update theme stylesheet
        if let Some(window) = web_sys::window() {
            if let Some(document) = window.document() {
                if let Some(link) = document.get_element_by_id("theme-stylesheet") {
                    if let Ok(link_element) = link.dyn_into::<web_sys::HtmlLinkElement>() {
                        let _ = link_element
                            .set_href(&format!("static/themes/{}/{}.css", theme, theme));
                        current_theme.set(theme.clone());
                        is_open.set(false);
                    }
                }
            }
        }
    };

    let toggle_dropdown = move |_| {
        is_open.update(|v| *v = !*v);
    };

    view! {
        <div class="theme-select-wrapper">
            <button
                class="button button--ghost button--smallall"
                on:click=toggle_dropdown
            >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"></path>
                </svg>
                "Тема"
            </button>

            <Show when=move || is_open.get()>
                <div class="theme-dropdown">
                    <For
                        each=move || vec![
                            ("dark", "Темная"),
                            ("light", "Светлая"),
                            ("forest", "Лесная"),
                        ]
                        key=|(id, _)| *id
                        children=move |(theme_id, theme_name)| {
                            let theme_id_str = theme_id.to_string();
                            let is_active = move || current_theme.get() == theme_id;
                            let theme_id_clone = theme_id_str.clone();

                            view! {
                                <button
                                    class=move || {
                                        if is_active() {
                                            "theme-dropdown__item theme-dropdown__item--active"
                                        } else {
                                            "theme-dropdown__item"
                                        }
                                    }
                                    on:click=move |_| change_theme(theme_id_clone.clone())
                                >
                                    {theme_name}
                                </button>
                            }
                        }
                    />
                </div>
            </Show>
        </div>
    }
}
