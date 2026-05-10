use leptos::prelude::*;

/// Глобальный контекст версий изменений по доменам.
/// Предоставляется в app.rs рядом с AppGlobalContext.
/// Добавить новый домен = одно поле здесь + одна строка в поллере.
#[derive(Clone, Copy)]
pub struct ChangeTokenContext {
    pub sys_tasks: RwSignal<u64>,
}

impl ChangeTokenContext {
    pub fn new() -> Self {
        Self {
            sys_tasks: RwSignal::new(0),
        }
    }
}
