//! Небольшая текстовая ссылка-подсказка с поповером. По клику разворачивает
//! панель произвольного содержимого над ссылкой; клик вне панели / Esc —
//! закрывают. Задумана как компактная замена вспомогательным кнопкам.

use leptos::prelude::*;
use leptos::task::spawn_local;

/// Ссылка `label`, открывающая поповер с `children`.
#[component]
pub fn HintLink(
    #[prop(into)] label: String,
    #[prop(optional, default = 340)] width_px: u32,
    children: Children,
) -> impl IntoView {
    let open = RwSignal::new(false);
    let open_below = RwSignal::new(false);
    let anchor_ref = NodeRef::<leptos::html::Span>::new();
    let panel_ref = NodeRef::<leptos::html::Div>::new();

    let escape = window_event_listener(leptos::ev::keydown, move |ev| {
        if ev.key() == "Escape" && open.get_untracked() {
            open.set(false);
        }
    });
    on_cleanup(move || escape.remove());

    view! {
        <span
            node_ref=anchor_ref
            style="position: relative; display: inline-flex; align-items: center;"
        >
            <a
                href="#"
                role="button"
                on:click=move |ev: leptos::ev::MouseEvent| {
                    ev.prevent_default();
                    let will_open = !open.get_untracked();
                    open.set(will_open);
                    if will_open {
                        spawn_local(async move {
                            gloo_timers::future::TimeoutFuture::new(0).await;
                            let (Some(anchor), Some(panel), Some(window)) = (
                                anchor_ref.get(),
                                panel_ref.get(),
                                web_sys::window(),
                            ) else {
                                return;
                            };
                            let anchor_rect = anchor.get_bounding_client_rect();
                            let panel_height = panel.get_bounding_client_rect().height();
                            let viewport_height = window
                                .inner_height()
                                .ok()
                                .and_then(|value| value.as_f64())
                                .unwrap_or(768.0);
                            let space_above = anchor_rect.top();
                            let space_below = viewport_height - anchor_rect.bottom();
                            open_below.set(
                                space_above < panel_height + 8.0 && space_below > space_above,
                            );
                        });
                    }
                }
                style="font-size:12px;color:var(--colorBrandForeground1,#0f6cbd);cursor:pointer;\
                       text-decoration:underline;white-space:nowrap;"
            >
                {label}
            </a>
            // Прозрачный слой-ловушка для клика вне панели.
            {move || open.get().then(|| view! {
                <div
                    on:click=move |_| open.set(false)
                    style="position:fixed;inset:0;z-index:4000;background:transparent;"
                ></div>
            })}
            // Панель всегда в DOM (только скрыта) — чтобы children строились один раз.
            <div node_ref=panel_ref style=move || format!(
                "position:absolute;{}right:0;z-index:4001;width:{width_px}px;max-width:80vw;\
                 max-height:60vh;overflow:auto;background:var(--colorNeutralBackground1);\
                 border:1px solid var(--colorNeutralStroke2);border-radius:8px;\
                 box-shadow:0 8px 24px rgba(0,0,0,0.18);padding:12px;font-size:13px;\
                 color:var(--colorNeutralForeground1);display:{};",
                if open_below.get() {
                    "top:calc(100% + 8px);bottom:auto;"
                } else {
                    "bottom:calc(100% + 8px);top:auto;"
                },
                if open.get() { "block" } else { "none" },
            )>
                {children()}
            </div>
        </span>
    }
}
