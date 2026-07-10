//! Сервис представлений агрегатов.
//!
//! Возвращает человекочитаемое представление (наименование + дата + id/номер)
//! по паре (тип регистратора, id). Логика владения — в модуле каждого агрегата
//! (`<module>::representation::represent_many`), здесь только тонкий диспетчер и
//! общий конструктор представления.
//!
//! Используется для детализации GL по регистратору и резолва ссылок (`refs.rs`).

use std::collections::HashMap;

use contracts::general_ledger::AggregateRepresentation;

/// Размер чанка id под лимит переменных SQLite.
pub const ID_CHUNK: usize = 500;

/// Нормализует дату до `YYYY-MM-DD` (обрезает время), пустую → None.
pub fn norm_date(raw: Option<String>) -> Option<String> {
    raw.map(|d| d.chars().take(10).collect::<String>())
        .map(|d| d.trim().to_string())
        .filter(|d| !d.is_empty())
}

/// Собирает представление: title = название типа (из метаданных агрегата),
/// date нормализуется до YYYY-MM-DD, doc_id обрезается (пустой → None).
/// Итоговая подпись формируется в [`to_label`] как «title · date · #doc_id».
pub fn build(
    type_name: &str,
    date: Option<String>,
    doc_id: Option<String>,
) -> AggregateRepresentation {
    AggregateRepresentation {
        title: type_name.trim().to_string(),
        date: norm_date(date),
        doc_id: doc_id
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
    }
}

/// Прогоняет батч-запрос `f` по чанкам id (под лимит переменных SQLite) и
/// объединяет результаты. Используется провайдерами модулей.
pub async fn chunked<F, Fut>(ids: &[String], f: F) -> HashMap<String, AggregateRepresentation>
where
    F: Fn(Vec<String>) -> Fut,
    Fut: std::future::Future<Output = HashMap<String, AggregateRepresentation>>,
{
    if ids.len() <= ID_CHUNK {
        return f(ids.to_vec()).await;
    }
    let mut out = HashMap::new();
    for chunk in ids.chunks(ID_CHUNK) {
        out.extend(f(chunk.to_vec()).await);
    }
    out
}

/// Строковая форма представления для `refs.rs`: «title · date · #doc_id».
pub fn to_label(rep: &AggregateRepresentation) -> String {
    let mut parts = vec![rep.title.clone()];
    if let Some(date) = rep.date.as_ref().filter(|d| !d.is_empty()) {
        parts.push(date.clone());
    }
    if let Some(doc) = rep.doc_id.as_ref().filter(|d| !d.is_empty()) {
        parts.push(format!("#{doc}"));
    }
    parts.join(" · ")
}

/// Батч-резолв представлений для набора id одного типа регистратора.
///
/// Для неизвестного типа или ненайденных id возвращает пустую/частичную карту —
/// вызывающая сторона делает фолбэк (UI — на синтетику).
pub async fn resolve_many(kind: &str, ids: &[String]) -> HashMap<String, AggregateRepresentation> {
    if ids.is_empty() {
        return HashMap::new();
    }
    match kind {
        "a012_wb_sales" => crate::domain::a012_wb_sales::representation::represent_many(ids).await,
        "a013_ym_order" => crate::domain::a013_ym_order::representation::represent_many(ids).await,
        "a014_ozon_transactions" => {
            crate::domain::a014_ozon_transactions::representation::represent_many(ids).await
        }
        "a015_wb_orders" => {
            crate::domain::a015_wb_orders::representation::represent_many(ids).await
        }
        "a016_ym_returns" => {
            crate::domain::a016_ym_returns::representation::represent_many(ids).await
        }
        "a021_production_output" => {
            crate::domain::a021_production_output::representation::represent_many(ids).await
        }
        "a022_kit_variant" => {
            crate::domain::a022_kit_variant::representation::represent_many(ids).await
        }
        "a023_purchase_of_goods" => {
            crate::domain::a023_purchase_of_goods::representation::represent_many(ids).await
        }
        "a026_wb_advert_daily" => {
            crate::domain::a026_wb_advert_daily::representation::represent_many(ids).await
        }
        "a034_ym_realization" => {
            crate::domain::a034_ym_realization::representation::represent_many(ids).await
        }
        "a036_wb_sales_funnel_daily" => {
            crate::domain::a036_wb_sales_funnel_daily::representation::represent_many(ids).await
        }
        "a037_wb_product_snapshot" => {
            crate::domain::a037_wb_product_snapshot::representation::represent_many(ids).await
        }
        "a028_missing_cost_registry" => {
            crate::domain::a028_missing_cost_registry::representation::represent_many(ids).await
        }
        "p903_wb_finance_report" => {
            crate::projections::p903_wb_finance_report::representation::represent_many(ids).await
        }
        "p907_ym_payment_report" => {
            crate::projections::p907_ym_payment_report::representation::represent_many(ids).await
        }
        _ => HashMap::new(),
    }
}

/// Резолв представления одного объекта. None — тип неизвестен или объект не найден.
pub async fn resolve(kind: &str, id: &str) -> Option<AggregateRepresentation> {
    let ids = [id.to_string()];
    resolve_many(kind, &ids).await.remove(id)
}
