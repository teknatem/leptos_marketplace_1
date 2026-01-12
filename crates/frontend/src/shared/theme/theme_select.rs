use crate::app::ThawThemeContext;
use leptos::prelude::*;
use thaw::Theme;
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
                    link_element.set_disabled(false);
                    let _ = link_element.set_href(&format!("static/themes/{}/{}.css", theme, theme));
                }
            }
        }
    }
}

/// Set CSS variable for Thaw ConfigProvider
fn set_thaw_background(value: &str) {
    if let Some(window) = web_sys::window() {
        if let Some(document) = window.document() {
            // Find .thaw-config-provider element
            if let Some(element) = document
                .query_selector(".thaw-config-provider")
                .ok()
                .flatten()
            {
                if let Some(html_element) = element.dyn_ref::<web_sys::HtmlElement>() {
                    // NOTE: This is intentionally a programmatic override for the forest theme.
                    // We use remove_property to restore Thaw defaults when leaving forest.
                    if value.is_empty() {
                        let _ = html_element
                            .style()
                            .remove_property("--colorNeutralBackground1");
                    } else {
                        let _ = html_element
                            .style()
                            .set_property("--colorNeutralBackground1", value);
                    }
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

    // Get Thaw theme context
    let thaw_theme_ctx = leptos::context::use_context::<ThawThemeContext>();

    // Apply saved theme on mount (including Thaw theme)
    Effect::new(move |_| {
        apply_theme(&saved_theme);
        if let Some(ctx) = thaw_theme_ctx {
            // Sync Thaw theme with app theme
            let thaw_theme = match saved_theme.as_str() {
                "light" => Theme::light(),
                "dark" => Theme::dark(),
                "forest" => Theme::dark(), // forest uses dark Thaw theme
                _ => Theme::dark(),
            };
            ctx.0.set(thaw_theme);

            // Forest theme: make Thaw surfaces transparent so background image is visible.
            if saved_theme == "forest" {
                set_thaw_background("transparent");
            } else {
                set_thaw_background("");
            }
        }
    });

    let change_theme = move |theme: String| {
        // Update theme stylesheet
        apply_theme(&theme);

        // Save theme to localStorage
        save_theme(&theme);

        // Update Thaw theme
        if let Some(ctx) = thaw_theme_ctx {
            let thaw_theme = match theme.as_str() {
                "light" => Theme::light(),
                "dark" => Theme::dark(),
                "forest" => Theme::dark(), // forest uses dark Thaw theme
                _ => Theme::dark(),
            };
            ctx.0.set(thaw_theme);

            // Forest theme: make Thaw surfaces transparent so background image is visible.
            if theme == "forest" {
                set_thaw_background("transparent");
            } else {
                set_thaw_background("");
            }
        }

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
                class="top-header__icon-button"
                on:click=toggle_dropdown
                title="Выбор темы"
            >
                <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <circle cx="13.5" cy="6.5" r=".5" fill="currentColor"/>
                    <circle cx="17.5" cy="10.5" r=".5" fill="currentColor"/>
                    <circle cx="8.5" cy="7.5" r=".5" fill="currentColor"/>
                    <circle cx="6.5" cy="12.5" r=".5" fill="currentColor"/>
                    <path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z"/>
                </svg>
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
