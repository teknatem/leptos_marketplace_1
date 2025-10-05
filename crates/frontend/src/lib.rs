pub mod app;
pub mod domain;
pub mod layout;
pub mod routes;
pub mod shared;
pub mod usecases;

use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub fn hydrate() {
    // initializes logging using the `log` crate
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();

    leptos::mount::mount_to_body(app::App);
}

#[wasm_bindgen(start)]
pub fn start() {
    hydrate();
}
