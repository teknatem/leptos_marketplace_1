use leptos::prelude::*;

/// Глобальный контекст версий изменений по доменам.
/// Предоставляется в app.rs рядом с AppGlobalContext.
/// Добавить новый домен = одно поле здесь + одна строка в поллере.
#[derive(Clone, Copy)]
pub struct ChangeTokenContext {
    pub sys_tasks: RwSignal<u64>,
    pub a027_wb_documents: RwSignal<u64>,
    pub a015_wb_orders: RwSignal<u64>,
    pub a012_wb_sales: RwSignal<u64>,
    pub a013_ym_order: RwSignal<u64>,
    pub plugins: RwSignal<u64>,
}

impl ChangeTokenContext {
    pub fn new() -> Self {
        Self {
            sys_tasks: RwSignal::new(0),
            a027_wb_documents: RwSignal::new(0),
            a015_wb_orders: RwSignal::new(0),
            a012_wb_sales: RwSignal::new(0),
            a013_ym_order: RwSignal::new(0),
            plugins: RwSignal::new(0),
        }
    }
}
