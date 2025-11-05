use leptos::prelude::*;
use leptos::prelude::ElementChild;
use leptos::logging::log;
use gloo_net::http::Request;

#[component]
pub fn WbSalesList() -> impl IntoView {
    let (loading, set_loading) = signal(false);
    let (count, set_count) = signal(0);

    let load_sales = move || {
        let set_count = set_count.clone();
        let set_loading = set_loading.clone();
        wasm_bindgen_futures::spawn_local(async move {
            set_loading.set(true);
            let url = "http://localhost:3000/api/a012/wb-sales";
            match Request::get(url).send().await {
                Ok(response) => {
                    match response.text().await {
                        Ok(text) => match serde_json::from_str::<Vec<serde_json::Value>>(&text) {
                            Ok(data) => set_count.set(data.len()),
                            Err(e) => log!("Failed to parse response: {:?}", e),
                        },
                        Err(e) => log!("Failed to read response: {:?}", e),
                    }
                    set_loading.set(false);
                }
                Err(e) => {
                    log!("Error: {:?}", e);
                    set_loading.set(false);
                }
            }
        });
    };

    view! {
        <div>
            <h2>"Wildberries Sales (A012)"</h2>
            <button on:click=move |_| { load_sales(); } style="padding: 8px 16px; margin: 20px 0; background: #4CAF50; color: white; border: none; border-radius: 4px; cursor: pointer;">
                "Load Sales"
            </button>
            {move || if loading.get() {
                let txt = "Loading...".to_string();
                view! { <div>
                    <p>{txt}</p>
                    <div></div>
                </div> }.into_view()
            } else {
                let txt = format!("Total: {} records", count.get());
                view! { <div>
                    <p>{txt}</p>
                    <div></div>
                </div> }.into_view()
            }}
        </div>
    }
}

