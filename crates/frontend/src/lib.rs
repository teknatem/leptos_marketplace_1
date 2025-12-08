#![allow(
    clippy::unit_arg,
    clippy::module_inception,
    clippy::clone_on_copy,
    clippy::useless_asref,
    clippy::option_as_ref_deref,
    clippy::cmp_owned,
    clippy::unnecessary_map_or,
    clippy::vec_init_then_push,
    clippy::needless_borrow,
    clippy::let_unit_value,
    clippy::bind_instead_of_map,
    clippy::ptr_arg,
    clippy::unused_enumerate_index,
    clippy::new_without_default,
    clippy::redundant_closure,
    clippy::single_match,
    clippy::manual_div_ceil,
    clippy::useless_format,
    clippy::unused_unit,
    clippy::empty_line_after_doc_comments,
    clippy::redundant_pattern_matching,
    clippy::unwrap_or_default,
    clippy::if_same_then_else
)]

pub mod app;
pub mod dashboards;
pub mod domain;
pub mod layout;
pub mod projections;
pub mod routes;
pub mod shared;
pub mod system;
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
