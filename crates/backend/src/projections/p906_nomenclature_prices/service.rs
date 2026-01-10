use anyhow::Result;
use super::repository::{self, PriceWithNomenclature, NomenclaturePriceEntry};

/// Получить список цен с фильтрами
pub async fn list_with_filters(
    period: Option<String>,
    nomenclature_ref: Option<String>,
    q: Option<String>,
    sort_by: Option<String>,
    sort_desc: Option<bool>,
    limit: Option<u64>,
    offset: Option<u64>,
) -> Result<(Vec<PriceWithNomenclature>, i64)> {
    repository::list_with_filters(period, nomenclature_ref, q, sort_by, sort_desc, limit, offset).await
}

/// Получить уникальные периоды для фильтра
pub async fn get_unique_periods() -> Result<Vec<String>> {
    repository::get_unique_periods().await
}

/// Удалить записи по периоду (перед импортом)
pub async fn delete_by_period(period: &str) -> Result<u64> {
    repository::delete_by_period(period).await
}

/// Вставить или обновить запись цены
pub async fn upsert_price(entry: &NomenclaturePriceEntry) -> Result<()> {
    repository::upsert_entry(entry).await
}

