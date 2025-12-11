//! Theme management module for the application.
//!
//! Provides a context-based theme system with support for dark, light, and forest themes.
//! Theme preference is persisted in localStorage.

use leptos::prelude::*;
use web_sys::window;

/// Available themes in the application.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
    Forest,
}

impl Theme {
    /// Returns the theme name as a string (used for CSS class and localStorage).
    pub fn as_str(&self) -> &'static str {
        match self {
            Theme::Dark => "dark",
            Theme::Light => "light",
            Theme::Forest => "forest",
        }
    }

    /// Returns the display name for the UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            Theme::Dark => "Тёмная",
            Theme::Light => "Светлая",
            Theme::Forest => "Лесная",
        }
    }

    /// Returns the CSS file path for this theme.
    pub fn css_path(&self) -> &'static str {
        match self {
            Theme::Dark => "/static/themes/dark/dark.css",
            Theme::Light => "/static/themes/light/light.css",
            Theme::Forest => "/static/themes/forest/forest.css",
        }
    }

    /// Parse theme from string.
    pub fn from_str(s: &str) -> Self {
        match s {
            "light" => Theme::Light,
            "forest" => Theme::Forest,
            _ => Theme::Dark,
        }
    }

    /// Returns all available themes.
    pub fn all() -> [Theme; 3] {
        [Theme::Dark, Theme::Light, Theme::Forest]
    }
}

const THEME_STORAGE_KEY: &str = "app-theme";

/// Load theme from localStorage.
fn load_theme_from_storage() -> Theme {
    window()
        .and_then(|w| w.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(THEME_STORAGE_KEY).ok().flatten())
        .map(|s| Theme::from_str(&s))
        .unwrap_or_default()
}

/// Save theme to localStorage.
fn save_theme_to_storage(theme: Theme) {
    if let Some(storage) = window().and_then(|w| w.local_storage().ok().flatten()) {
        let _ = storage.set_item(THEME_STORAGE_KEY, theme.as_str());
    }
}

/// Apply theme by loading the theme CSS file.
fn apply_theme_css(theme: Theme) {
    let document = match window().and_then(|w| w.document()) {
        Some(doc) => doc,
        None => return,
    };

    let head = match document.head() {
        Some(h) => h,
        None => return,
    };

    // Remove existing theme stylesheet
    if let Ok(existing) = document.query_selector("#theme-stylesheet") {
        if let Some(elem) = existing {
            let _ = elem.remove();
        }
    }

    // Create new link element for theme CSS
    if let Ok(link) = document.create_element("link") {
        let _ = link.set_attribute("id", "theme-stylesheet");
        let _ = link.set_attribute("rel", "stylesheet");
        let _ = link.set_attribute("href", theme.css_path());
        let _ = head.append_child(&link);
    }

    // Also set data-theme attribute on body for additional styling hooks
    if let Some(body) = document.body() {
        let _ = body.set_attribute("data-theme", theme.as_str());
    }
}

/// Theme context type.
#[derive(Clone, Copy)]
pub struct ThemeContext {
    /// Current theme signal.
    pub theme: RwSignal<Theme>,
}

impl ThemeContext {
    /// Set the theme and persist to storage.
    pub fn set_theme(&self, theme: Theme) {
        self.theme.set(theme);
        save_theme_to_storage(theme);
        apply_theme_css(theme);
    }

    /// Get the current theme.
    pub fn get_theme(&self) -> Theme {
        self.theme.get()
    }

    /// Cycle to the next theme.
    pub fn cycle_theme(&self) {
        let current = self.theme.get();
        let next = match current {
            Theme::Dark => Theme::Light,
            Theme::Light => Theme::Forest,
            Theme::Forest => Theme::Dark,
        };
        self.set_theme(next);
    }
}

/// Provides theme context to children components.
#[component]
pub fn ThemeProvider(children: Children) -> impl IntoView {
    // Load theme from storage on initial render
    let initial_theme = load_theme_from_storage();
    let theme = RwSignal::new(initial_theme);

    // Apply initial theme CSS
    apply_theme_css(initial_theme);

    let context = ThemeContext { theme };
    provide_context(context);

    children()
}

/// Hook to use the theme context.
pub fn use_theme() -> ThemeContext {
    use_context::<ThemeContext>().expect("ThemeContext not found. Wrap your app with ThemeProvider.")
}

/// Theme selector dropdown component.
#[component]
pub fn ThemeSelector() -> impl IntoView {
    let ctx = use_theme();
    let (dropdown_open, set_dropdown_open) = signal(false);

    let toggle_dropdown = move |_| {
        set_dropdown_open.update(|open| *open = !*open);
    };

    let select_theme = move |theme: Theme| {
        ctx.set_theme(theme);
        set_dropdown_open.set(false);
    };

    // Close dropdown when clicking outside
    Effect::new(move |_| {
        if dropdown_open.get() {
            use wasm_bindgen::prelude::*;
            use wasm_bindgen::JsCast;
            
            let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
                set_dropdown_open.set(false);
            }) as Box<dyn FnMut(_)>);
            
            if let Some(window) = window() {
                let _ = window.add_event_listener_with_callback(
                    "click",
                    closure.as_ref().unchecked_ref()
                );
                closure.forget(); // Keep the closure alive
            }
        }
    });

    view! {
        <div class="theme-selector" style="position: relative;">
            <button
                class="top-header-icon-btn"
                on:click=move |ev| {
                    ev.stop_propagation();
                    toggle_dropdown(ev);
                }
                title="Выбор темы"
            >
                {crate::shared::icons::icon("palette")}
            </button>

            <Show when=move || dropdown_open.get()>
                <div class="theme-dropdown" on:click=move |ev| ev.stop_propagation()>
                    {Theme::all().into_iter().map(|theme| {
                        let is_active = move || ctx.theme.get() == theme;
                        view! {
                            <button
                                class=move || if is_active() { "theme-dropdown-item active" } else { "theme-dropdown-item" }
                                on:click=move |_| select_theme(theme)
                            >
                                {theme.display_name()}
                            </button>
                        }
                    }).collect_view()}
                </div>
            </Show>
        </div>
    }
}

