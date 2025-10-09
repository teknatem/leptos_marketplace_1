pub mod center;
pub mod footer;
pub mod global_context;
pub mod header;
pub mod left;
pub mod right;

use crate::shared::picker_aggregate::ModalRenderer;
use leptos::prelude::*;

#[component]
pub fn Shell<L, C, R>(left: L, center: C, right: R) -> impl IntoView
where
    L: Fn() -> AnyView + 'static + Send,
    C: Fn() -> AnyView + 'static + Send,
    R: Fn() -> AnyView + 'static + Send,
{
    view! {
        <header::Header />
        <div class="main-content">
            <left::Left>
                {left()}
            </left::Left>
            <center::Center>
                {center()}
            </center::Center>
            <right::Right>
                {right()}
            </right::Right>
        </div>
        <footer::Footer />
        <ModalRenderer />
    }
}
