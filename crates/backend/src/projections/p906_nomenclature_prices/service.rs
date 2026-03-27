use super::repository::{self, NomenclaturePriceEntry, PriceWithNomenclature};
use anyhow::Result;
use uuid::Uuid;

const ZERO_UUID: &str = "00000000-0000-0000-0000-000000000000";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedPriceSource {
    NomenclatureOnDate,
    BaseNomenclatureOnDate,
    NomenclatureLastNonzero,
    BaseNomenclatureLastNonzero,
}

#[derive(Debug, Clone)]
pub struct ResolvedPrice {
    pub price: f64,
    pub source: ResolvedPriceSource,
    pub source_ref: String,
}

impl ResolvedPrice {
    pub fn describe(&self, target_date: &str) -> String {
        match self.source {
            ResolvedPriceSource::NomenclatureOnDate => {
                format!("nomenclature {} on date {}", self.source_ref, target_date)
            }
            ResolvedPriceSource::BaseNomenclatureOnDate => {
                format!(
                    "base_nomenclature {} on date {}",
                    self.source_ref, target_date
                )
            }
            ResolvedPriceSource::NomenclatureLastNonzero => {
                format!("nomenclature {} (last nonzero price)", self.source_ref)
            }
            ResolvedPriceSource::BaseNomenclatureLastNonzero => {
                format!("base_nomenclature {} (last nonzero price)", self.source_ref)
            }
        }
    }
}

async fn get_valid_base_nomenclature_ref(nomenclature_ref: &str) -> Result<Option<String>> {
    let Ok(nom_uuid) = Uuid::parse_str(nomenclature_ref) else {
        return Ok(None);
    };

    let Some(nomenclature) = crate::domain::a004_nomenclature::service::get_by_id(nom_uuid).await?
    else {
        return Ok(None);
    };

    Ok(nomenclature.base_nomenclature_ref.filter(|base_ref| {
        !base_ref.is_empty() && base_ref != ZERO_UUID && base_ref != nomenclature_ref
    }))
}

async fn get_positive_price_for_date(
    nomenclature_ref: &str,
    target_date: &str,
) -> Result<Option<f64>> {
    Ok(
        repository::get_price_for_date(nomenclature_ref, target_date)
            .await?
            .filter(|price| *price > 0.0),
    )
}

pub async fn resolve_price_for_nomenclature(
    nomenclature_ref: &str,
    target_date: &str,
) -> Result<Option<ResolvedPrice>> {
    if nomenclature_ref.trim().is_empty() {
        return Ok(None);
    }

    if let Some(price) = get_positive_price_for_date(nomenclature_ref, target_date).await? {
        return Ok(Some(ResolvedPrice {
            price,
            source: ResolvedPriceSource::NomenclatureOnDate,
            source_ref: nomenclature_ref.to_string(),
        }));
    }

    let base_ref = get_valid_base_nomenclature_ref(nomenclature_ref).await?;

    if let Some(base_ref) = base_ref.as_deref() {
        if let Some(price) = get_positive_price_for_date(base_ref, target_date).await? {
            return Ok(Some(ResolvedPrice {
                price,
                source: ResolvedPriceSource::BaseNomenclatureOnDate,
                source_ref: base_ref.to_string(),
            }));
        }
    }

    if let Some(price) = repository::get_last_nonzero_price(nomenclature_ref).await? {
        return Ok(Some(ResolvedPrice {
            price,
            source: ResolvedPriceSource::NomenclatureLastNonzero,
            source_ref: nomenclature_ref.to_string(),
        }));
    }

    if let Some(base_ref) = base_ref.as_deref() {
        if let Some(price) = repository::get_last_nonzero_price(base_ref).await? {
            return Ok(Some(ResolvedPrice {
                price,
                source: ResolvedPriceSource::BaseNomenclatureLastNonzero,
                source_ref: base_ref.to_string(),
            }));
        }
    }

    Ok(None)
}

pub async fn resolve_price_for_optional_nomenclature(
    nomenclature_ref: Option<&str>,
    target_date: &str,
) -> Result<Option<ResolvedPrice>> {
    let Some(nomenclature_ref) = nomenclature_ref else {
        return Ok(None);
    };

    resolve_price_for_nomenclature(nomenclature_ref, target_date).await
}

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
    repository::list_with_filters(
        period,
        nomenclature_ref,
        q,
        sort_by,
        sort_desc,
        limit,
        offset,
    )
    .await
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
