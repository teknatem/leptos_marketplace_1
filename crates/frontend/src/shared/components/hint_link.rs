//! Небольшая текстовая ссылка-подсказка с поповером. По клику разворачивает
//! панель произвольного содержимого над ссылкой; клик вне панели / Esc —
//! закрывают. Задумана как компактная замена вспомогательным кнопкам.

use leptos::prelude::*;

/// Ссылка `label`, открывающая поповер с `children`.
#[component]
pub fn HintLink(#[prop(into)] label: String, children: Children) -> impl IntoView {
    let open = RwSignal::new(false);

    let escape = window_event_listener(leptos::ev::keydown, move |ev| {
        if ev.key() == "Escape" && open.get_untracked() {
            open.set(false);
        }
    });
    on_cleanup(move || escape.remove());

    view! {
        <span style="position: relative; display: inline-flex; align-items: center;">
            <a
                href="#"
                role="button"
                on:click=move |ev: leptos::ev::MouseEvent| {
                    ev.prevent_default();
                    open.update(|o| *o = !*o);
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
            <div style=move || format!(
                "position:absolute;bottom:calc(100% + 8px);right:0;z-index:4001;width:340px;max-width:80vw;\
                 max-height:60vh;overflow:auto;background:var(--colorNeutralBackground1);\
                 border:1px solid var(--colorNeutralStroke2);border-radius:8px;\
                 box-shadow:0 8px 24px rgba(0,0,0,0.18);padding:12px;font-size:13px;\
                 color:var(--colorNeutralForeground1);display:{};",
                if open.get() { "block" } else { "none" },
            )>
                {children()}
            </div>
        </span>
    }
}
