use leptos::prelude::*;
use wasm_bindgen::JsCast;

const THEME_STORAGE_KEY: &str = "app_theme";
const DEFAULT_THEME: &str = "dark";

/// Get saved theme from localStorage
fn get_saved_theme() -> String {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(theme)) = storage.get_item(THEME_STORAGE_KEY) {
                return theme;
            }
        }
    }
    DEFAULT_THEME.to_string()
}

/// Save theme to localStorage
fn save_theme(theme: &str) {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(THEME_STORAGE_KEY, theme);
        }
    }
}

/// Apply theme to the document
fn apply_theme(theme: &str) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            // Set data-theme attribute on body for CSS selectors
            if let Some(body) = document.body() {
                let _ = body.set_attribute("data-theme", theme);
            }

            // Update theme stylesheet link
            if let Some(link) = document.get_element_by_id("theme-stylesheet") {
                if let Ok(link_element) = link.dyn_into::<web_sys::HtmlLinkElement>() {
                    let _ =
                        link_element.set_href(&format!("static/themes/{}/{}.css", theme, theme));
                }
            }
        }
    }
}

/// ThemeSelect component for switching themes
#[component]
pub fn ThemeSelect() -> impl IntoView {
    // Load saved theme on mount
    let saved_theme = get_saved_theme();
    let current_theme = RwSignal::new(saved_theme.clone());
    let is_open = RwSignal::new(false);

    // Apply saved theme on mount
    Effect::new(move |_| {
        apply_theme(&saved_theme);
    });

    let change_theme = move |theme: String| {
        // Update theme stylesheet
        apply_theme(&theme);

        // Save theme to localStorage
        save_theme(&theme);

        // Update current theme signal
        current_theme.set(theme);
        is_open.set(false);
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
