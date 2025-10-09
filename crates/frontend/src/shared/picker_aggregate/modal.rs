use leptos::prelude::*;

/// Сервис для централизованного управления модальными окнами
#[derive(Clone, Copy)]
pub struct ModalService {
    is_visible: RwSignal<bool>,
}

impl ModalService {
    pub fn new() -> Self {
        Self {
            is_visible: RwSignal::new(false),
        }
    }

    /// Показать модальное окно
    pub fn show(&self) {
        self.is_visible.set(true);
    }

    /// Скрыть модальное окно
    pub fn hide(&self) {
        self.is_visible.set(false);
    }

    /// Проверить, открыто ли модальное окно
    pub fn is_open(&self) -> bool {
        self.is_visible.get()
    }
}

/// Компонент модального окна
/// Использование:
/// ```rust
/// let modal = use_context::<ModalService>().unwrap();
/// modal.show();
///
/// view! {
///     <Modal>
///         <MyComponent />
///     </Modal>
/// }
/// ```
#[component]
pub fn Modal(children: ChildrenFn) -> impl IntoView {
    let modal = use_context::<ModalService>().expect("ModalService not provided in context");

    view! {
        {move || {
            if modal.is_visible.get() {
                view! {
                    <div
                        class="modal-overlay"
                        on:click=move |_| modal.hide()
                    >
                        <div
                            class="modal-content"
                            on:click=|e| e.stop_propagation()
                        >
                            {children()}
                        </div>
                    </div>
                }.into_any()
            } else {
                view! { <></> }.into_any()
            }
        }}
    }
}

/// Устаревший компонент для обратной совместимости
#[component]
pub fn ModalRenderer() -> impl IntoView {
    view! { <></> }
}
