use crate::shared::modal_frame::ModalFrame;
use leptos::prelude::*;
use thaw::{Button, ButtonAppearance};

/// Preview/confirmation shell for pasted screenshots.
/// Cropping and annotations can later be added inside the central canvas
/// without changing the confirmation flow.
#[component]
pub fn ScreenshotEditor(
    preview_url: String,
    is_uploading: RwSignal<bool>,
    error: RwSignal<Option<String>>,
    on_cancel: UnsyncCallback<()>,
    on_confirm: UnsyncCallback<()>,
) -> impl IntoView {
    let escape_handle = window_event_listener(leptos::ev::keydown, move |ev| {
        if ev.key() == "Escape" && !is_uploading.get_untracked() {
            ev.prevent_default();
            on_cancel.run(());
        }
    });
    on_cleanup(move || escape_handle.remove());

    view! {
        <ModalFrame
            on_close=Callback::new(|_| {})
            close_on_overlay=false
            z_index=3000
            modal_style="width: min(96vw, 1500px); height: 94vh; max-width: none; padding: 0; overflow: hidden; display: flex; flex-direction: column;".to_string()
        >
            <header style="flex: 0 0 auto; display: flex; flex-direction: column; gap: 8px; padding: 12px 18px; border-bottom: 1px solid var(--colorNeutralStroke2);">
                <div style="display: flex; align-items: center; justify-content: space-between; gap: 16px;">
                    <h2 style="font-size: 18px; margin: 0;">
                        "Редактор скриншота / добавление нового изображения"
                    </h2>
                    <div style="display: flex; flex: 0 0 auto; gap: 8px;">
                        <Button
                            appearance=ButtonAppearance::Secondary
                            disabled=is_uploading
                            on_click=move |_| on_cancel.run(())
                        >
                            "Отмена"
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Primary
                            disabled=is_uploading
                            on_click=move |_| on_confirm.run(())
                        >
                            {move || if is_uploading.get() { "Добавление…" } else { "ОК" }}
                        </Button>
                    </div>
                </div>
                {move || error.get().map(|message| view! {
                    <div role="alert" style="color: var(--colorPaletteRedForeground1); font-size: 13px;">
                        {message}
                    </div>
                })}
            </header>
            <main style="flex: 1 1 auto; min-height: 0; display: flex; align-items: center; justify-content: center; padding: 16px; background: var(--colorNeutralBackground3);">
                <img
                    src=preview_url
                    alt="Предварительный просмотр скриншота"
                    style="display: block; width: 100%; height: 100%; object-fit: contain;"
                />
            </main>
        </ModalFrame>
    }
}
