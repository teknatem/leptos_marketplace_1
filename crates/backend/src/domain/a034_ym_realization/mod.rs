pub mod posting;
pub mod repository;
pub mod representation;
pub mod service;

/// Минимальный набор полей документа, нужный для построения GL-проводок.
/// Позволяет тестировать `posting::build_general_ledger_entries` без БД.
#[derive(Debug, Clone)]
pub struct DocumentForPosting {
    pub id: String,
    pub document_date: String,
    pub connection_id: String,
    pub sales_revenue: f64,
    pub return_revenue: f64,
    pub sales_qty: f64,
    pub return_qty: f64,
}
