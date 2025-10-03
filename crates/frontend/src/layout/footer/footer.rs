use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Copy, Debug, PartialEq)]
enum ServerStatus {
    Online,
    Offline,
    Checking,
}

impl ServerStatus {
    fn display_text(&self) -> &'static str {
        match self {
            ServerStatus::Online => "Server: Online",
            ServerStatus::Offline => "Server: Offline",
            ServerStatus::Checking => "Server: Checking...",
        }
    }

    fn css_class(&self) -> &'static str {
        match self {
            ServerStatus::Online => "status-online",
            ServerStatus::Offline => "status-offline",
            ServerStatus::Checking => "status-checking",
        }
    }
}

#[component]
pub fn Footer() -> impl IntoView {
    let status = RwSignal::new(ServerStatus::Checking);

    // Простая функция проверки сервера
    let check_server = move || {
        status.set(ServerStatus::Checking);

        spawn_local(async move {
            let result = ping_server().await;
            status.set(if result {
                ServerStatus::Online
            } else {
                ServerStatus::Offline
            });
        });
    };

    // Запускаем проверку при монтировании
    Effect::new(move |_| {
        check_server();
    });

    view! {
        <footer data-zone="footer" class="status-bar">
            <span class=move || status.get().css_class()>
                {move || status.get().display_text()}
            </span>
        </footer>
    }
}

async fn ping_server() -> bool {
    let window = match web_sys::window() {
        Some(w) => w,
        None => return false,
    };

    let request = match web_sys::Request::new_with_str("http://127.0.0.1:3000/health") {
        Ok(r) => r,
        Err(_) => return false,
    };

    let _ = request.headers().set("Accept", "application/json");

    let promise = window.fetch_with_request(&request);
    let response = match wasm_bindgen_futures::JsFuture::from(promise).await {
        Ok(r) => r,
        Err(_) => return false,
    };

    let response: web_sys::Response = match response.dyn_into() {
        Ok(r) => r,
        Err(_) => return false,
    };

    response.ok()
}
