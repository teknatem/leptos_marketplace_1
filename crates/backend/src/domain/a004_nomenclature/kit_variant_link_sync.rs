use anyhow::Result;
use contracts::domain::common::AggregateId;
use sea_orm::TransactionTrait;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct KitVariantLinkSyncStats {
    pub linked: usize,
    pub cleared: usize,
    pub unchanged: usize,
    pub ambiguous_owner_refs: usize,
}

pub async fn sync_links() -> Result<KitVariantLinkSyncStats> {
    let nomenclature_items = super::repository::list_all().await?;
    let kit_variants =
        crate::domain::a022_kit_variant::repository::list_active_with_owner_refs().await?;

    let mut canonical_by_owner_ref: HashMap<String, String> = HashMap::new();
    let mut ambiguous_owner_refs = HashSet::new();

    for variant in kit_variants {
        let Some(owner_ref) = variant.owner_ref.clone() else {
            continue;
        };

        match canonical_by_owner_ref.entry(owner_ref.clone()) {
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(variant.base.id.as_string());
            }
            std::collections::hash_map::Entry::Occupied(_) => {
                ambiguous_owner_refs.insert(owner_ref);
            }
        }
    }

    let db = crate::shared::data::db::get_connection();
    let txn = db.begin().await?;
    let mut stats = KitVariantLinkSyncStats {
        ambiguous_owner_refs: ambiguous_owner_refs.len(),
        ..Default::default()
    };

    for mut nomenclature in nomenclature_items
        .into_iter()
        .filter(|item| !item.base.metadata.is_deleted)
    {
        let expected_ref = canonical_by_owner_ref
            .get(&nomenclature.base.id.as_string())
            .cloned();

        if nomenclature.kit_variant_ref == expected_ref {
            stats.unchanged += 1;
            continue;
        }

        if expected_ref.is_some() {
            stats.linked += 1;
        } else if nomenclature.kit_variant_ref.is_some() {
            stats.cleared += 1;
        } else {
            stats.unchanged += 1;
            continue;
        }

        nomenclature.kit_variant_ref = expected_ref;
        nomenclature.before_write();
        super::repository::update_txn(&txn, &nomenclature).await?;
    }

    txn.commit().await?;

    if !ambiguous_owner_refs.is_empty() {
        tracing::warn!(
            "A004 kit_variant link sync found {} owner_ref conflicts in a022_kit_variant; latest record was used",
            ambiguous_owner_refs.len()
        );
    }

    tracing::info!(
        "A004 kit_variant link sync completed: linked={}, cleared={}, unchanged={}, ambiguous_owner_refs={}",
        stats.linked,
        stats.cleared,
        stats.unchanged,
        stats.ambiguous_owner_refs
    );

    Ok(stats)
}
