use leptos::prelude::*;

#[derive(Clone, Debug)]
pub struct MarketplaceProductListState {
    // Фильтры
    pub marketplace_ref: Option<String>,
    pub search: String,

    // Сортировка
    pub sort_field: String,
    pub sort_ascending: bool,

    // Множественный выбор
    pub selected_ids: Vec<String>,

    // Флаг загрузки
    pub is_loaded: bool,

    // Серверная пагинация
    pub page: usize,
    pub page_size: usize,
    pub total_count: usize,
    pub total_pages: usize,
}

impl Default for MarketplaceProductListState {
    fn default() -> Self {
        Self {
            marketplace_ref: None,
            search: String::new(),
            sort_field: "code".to_string(),
            sort_ascending: true,
            selected_ids: Vec::new(),
            is_loaded: false,
            // Пагинация
            page: 0,
            page_size: 50,
            total_count: 0,
            total_pages: 0,
        }
    }
}

pub fn create_state() -> RwSignal<MarketplaceProductListState> {
    RwSignal::new(MarketplaceProductListState::default())
}
