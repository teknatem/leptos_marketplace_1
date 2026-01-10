use crate::shared::modal_frame::ModalFrame;
use leptos::prelude::*;
use gloo_timers::future::TimeoutFuture;
use std::sync::Arc;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::KeyboardEvent;

#[derive(Clone)]
struct ModalEntry {
    id: u64,
    builder: Arc<dyn Fn(ModalHandle) -> AnyView + Send + Sync>,
    modal_style: Option<String>,
    modal_class: Option<String>,
    can_close: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
}

/// A handle returned by `ModalStackService::push`.
///
/// Can be cloned and used inside event handlers to close the modal.
#[derive(Clone)]
pub struct ModalHandle {
    id: u64,
    svc: ModalStackService,
}

impl ModalHandle {
    pub fn close(&self) {
        self.svc.close_deferred(self.id);
    }

    pub fn id(&self) -> u64 {
        self.id
    }
}

/// Centralized modal stack for rare multi-modal flows.
///
/// - Supports push/close/pop
/// - Escape closes only the topmost modal (handled by `ModalHost`)
#[derive(Clone, Copy)]
pub struct ModalStackService {
    stack: RwSignal<Vec<ModalEntry>>,
    next_id: RwSignal<u64>,
}

impl ModalStackService {
    pub fn new() -> Self {
        Self {
            stack: RwSignal::new(Vec::new()),
            next_id: RwSignal::new(1),
        }
    }

    fn defer(&self, f: impl FnOnce(ModalStackService) + 'static) {
        let svc = *self;
        spawn_local(async move {
            // Defer to next tick to avoid "closure invoked ... after being dropped" when
            // a modal is removed synchronously during the originating DOM event dispatch.
            TimeoutFuture::new(0).await;
            f(svc);
        });
    }

    pub fn is_open(&self) -> bool {
        !self.stack.get().is_empty()
    }

    pub fn len(&self) -> usize {
        self.stack.get().len()
    }

    /// Push a new modal onto the stack.
    ///
    /// `builder` receives a `ModalHandle` so the modal can close itself.
    pub fn push<F>(&self, builder: F) -> ModalHandle
    where
        F: Fn(ModalHandle) -> AnyView + Send + Sync + 'static,
    {
        let id = self.next_id.get();
        self.next_id.set(id + 1);

        let handle = ModalHandle { id, svc: *self };
        let builder = Arc::new(builder) as Arc<dyn Fn(ModalHandle) -> AnyView + Send + Sync>;

        self.stack.update(|s| {
            s.push(ModalEntry {
                id,
                builder,
                modal_style: None,
                modal_class: None,
                can_close: None,
            });
        });

        handle
    }

    /// Push a new modal with style/class overrides for the modal surface.
    pub fn push_with_frame<F>(
        &self,
        modal_style: Option<String>,
        modal_class: Option<String>,
        builder: F,
    ) -> ModalHandle
    where
        F: Fn(ModalHandle) -> AnyView + Send + Sync + 'static,
    {
        let id = self.next_id.get();
        self.next_id.set(id + 1);

        let handle = ModalHandle { id, svc: *self };
        let builder = Arc::new(builder) as Arc<dyn Fn(ModalHandle) -> AnyView + Send + Sync>;

        self.stack.update(|s| {
            s.push(ModalEntry {
                id,
                builder,
                modal_style,
                modal_class,
                can_close: None,
            });
        });

        handle
    }

    /// Push a new modal with style/class overrides AND a close guard.
    ///
    /// If `can_close` returns false, overlay-click and Escape will NOT close the modal.
    pub fn push_with_frame_guard<F>(
        &self,
        modal_style: Option<String>,
        modal_class: Option<String>,
        can_close: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
        builder: F,
    ) -> ModalHandle
    where
        F: Fn(ModalHandle) -> AnyView + Send + Sync + 'static,
    {
        let id = self.next_id.get();
        self.next_id.set(id + 1);

        let handle = ModalHandle { id, svc: *self };
        let builder = Arc::new(builder) as Arc<dyn Fn(ModalHandle) -> AnyView + Send + Sync>;

        self.stack.update(|s| {
            s.push(ModalEntry {
                id,
                builder,
                modal_style,
                modal_class,
                can_close,
            });
        });

        handle
    }

    pub fn close(&self, id: u64) {
        self.stack.update(|s| {
            s.retain(|e| e.id != id);
        });
    }

    pub fn close_deferred(&self, id: u64) {
        self.defer(move |svc| svc.close(id));
    }

    pub fn pop(&self) {
        self.stack.update(|s| {
            s.pop();
        });
    }

    pub fn pop_deferred(&self) {
        self.defer(|svc| svc.pop());
    }

    pub fn clear(&self) {
        self.stack.set(Vec::new());
    }

    pub fn clear_deferred(&self) {
        self.defer(|svc| svc.clear());
    }
}

/// Renders the modal stack at the application root.
///
/// Must be mounted exactly once.
#[component]
pub fn ModalHost() -> impl IntoView {
    let svc = use_context::<ModalStackService>()
        .expect("ModalStackService not provided in context (provide it in app root)");

    // Global Escape handler: closes only the topmost modal.
    Effect::new(move |_| {
        let svc = svc;

        let closure = Closure::wrap(Box::new(move |event: web_sys::Event| {
            if let Some(keyboard_event) = event.dyn_ref::<KeyboardEvent>() {
                if keyboard_event.key() == "Escape" && svc.is_open() {
                    let can_close = svc
                        .stack
                        .get_untracked()
                        .last()
                        .and_then(|e| e.can_close.clone())
                        .map(|f| f())
                        .unwrap_or(true);
                    if can_close {
                        svc.pop_deferred();
                    }
                }
            }
        }) as Box<dyn FnMut(_)>);

        if let Some(window) = web_sys::window() {
            let _ = window
                .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref());
            // ModalHost is mounted once for the whole app lifetime; keep closure alive.
            closure.forget();
        }
    });

    view! {
        <Show when=move || svc.is_open()>
            <For
                each=move || {
                    svc.stack
                        .get()
                        .into_iter()
                        .enumerate()
                        .collect::<Vec<(usize, ModalEntry)>>()
                }
                key=|(_, entry)| entry.id
                children=move |(idx, entry)| {
                    // z-index based on current stack order
                    let z_index = 1000 + idx as i32;
                    let on_close = {
                        let svc = svc;
                        let id = entry.id;
                        let can_close = entry.can_close.clone();
                        Callback::new(move |_| {
                            let allowed = can_close.as_ref().map(|f| f()).unwrap_or(true);
                            if allowed {
                                svc.close_deferred(id);
                            }
                        })
                    };

                    let handle = ModalHandle { id: entry.id, svc };
                    let view = (entry.builder)(handle);
                    let modal_style = entry.modal_style.clone().unwrap_or_default();
                    let modal_class = entry.modal_class.clone().unwrap_or_default();

                    view! {
                        <ModalFrame
                            z_index=z_index
                            on_close=on_close
                            modal_style=modal_style
                            modal_class=modal_class
                        >
                            {view}
                        </ModalFrame>
                    }
                }
            />
        </Show>
    }
}
