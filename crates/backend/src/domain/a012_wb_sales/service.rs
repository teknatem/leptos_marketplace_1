use super::repository;
use anyhow::Result;
use contracts::domain::a012_wb_sales::aggregate::WbSales;
use contracts::domain::common::AggregateId;
use std::collections::{BTreeSet, HashMap};
use uuid::Uuid;

const ZERO_UUID: &str = "00000000-0000-0000-0000-000000000000";

#[derive(Default)]
pub struct PostingPreparationCache {
    connection_by_id:
        HashMap<String, Option<contracts::domain::a006_connection_mp::aggregate::ConnectionMP>>,
    marketplace_product_by_id: HashMap<
        String,
        Option<contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct>,
    >,
    acquiring_fee_rate_by_marketplace_id: HashMap<String, f64>,
    resolved_price_by_nom_and_date: HashMap<
        (String, String),
        Option<crate::projections::p906_nomenclature_prices::service::ResolvedPrice>,
    >,
    nomenclature_by_id:
        HashMap<String, Option<contracts::domain::a004_nomenclature::aggregate::Nomenclature>>,
    kit_variant_by_owner_ref:
        HashMap<String, Option<contracts::domain::a022_kit_variant::aggregate::KitVariant>>,
    direct_cost_by_nom_and_date: HashMap<(String, String), Option<f64>>,
    resolved_prod_unit_cost_by_nom_and_date: HashMap<(String, String), Option<f64>>,
}

impl PostingPreparationCache {
    async fn get_connection(
        &mut self,
        connection_id: &str,
    ) -> Result<Option<contracts::domain::a006_connection_mp::aggregate::ConnectionMP>> {
        if let Some(value) = self.connection_by_id.get(connection_id) {
            return Ok(value.clone());
        }

        let resolved = match Uuid::parse_str(connection_id) {
            Ok(id) => crate::domain::a006_connection_mp::service::get_by_id(id).await?,
            Err(_) => None,
        };
        self.connection_by_id
            .insert(connection_id.to_string(), resolved.clone());
        Ok(resolved)
    }

    async fn get_marketplace_product(
        &mut self,
        marketplace_product_ref: &str,
    ) -> Result<Option<contracts::domain::a007_marketplace_product::aggregate::MarketplaceProduct>>
    {
        if let Some(value) = self.marketplace_product_by_id.get(marketplace_product_ref) {
            return Ok(value.clone());
        }

        let resolved = match Uuid::parse_str(marketplace_product_ref) {
            Ok(id) => crate::domain::a007_marketplace_product::service::get_by_id(id).await?,
            Err(_) => None,
        };
        self.marketplace_product_by_id
            .insert(marketplace_product_ref.to_string(), resolved.clone());
        Ok(resolved)
    }

    async fn get_acquiring_fee_rate(&mut self, marketplace_id: &str) -> Result<f64> {
        if let Some(value) = self
            .acquiring_fee_rate_by_marketplace_id
            .get(marketplace_id)
        {
            return Ok(*value);
        }

        let rate = match Uuid::parse_str(marketplace_id) {
            Ok(id) => crate::domain::a005_marketplace::service::get_by_id(id)
                .await?
                .map(|marketplace| marketplace.acquiring_fee_pro)
                .unwrap_or(0.0),
            Err(_) => 0.0,
        };

        self.acquiring_fee_rate_by_marketplace_id
            .insert(marketplace_id.to_string(), rate);
        Ok(rate)
    }

    async fn resolve_price(
        &mut self,
        nomenclature_ref: &str,
        sale_date: &str,
    ) -> Result<Option<crate::projections::p906_nomenclature_prices::service::ResolvedPrice>> {
        let cache_key = (nomenclature_ref.to_string(), sale_date.to_string());
        if let Some(value) = self.resolved_price_by_nom_and_date.get(&cache_key) {
            return Ok(value.clone());
        }

        let resolved =
            crate::projections::p906_nomenclature_prices::service::resolve_price_for_nomenclature(
                nomenclature_ref,
                sale_date,
            )
            .await?;
        self.resolved_price_by_nom_and_date
            .insert(cache_key, resolved.clone());
        Ok(resolved)
    }

    async fn get_nomenclature(
        &mut self,
        nomenclature_ref: &str,
    ) -> Result<Option<contracts::domain::a004_nomenclature::aggregate::Nomenclature>> {
        if let Some(value) = self.nomenclature_by_id.get(nomenclature_ref) {
            return Ok(value.clone());
        }

        let resolved = match Uuid::parse_str(nomenclature_ref) {
            Ok(id) => crate::domain::a004_nomenclature::repository::get_by_id(id).await?,
            Err(_) => None,
        };
        self.nomenclature_by_id
            .insert(nomenclature_ref.to_string(), resolved.clone());
        Ok(resolved)
    }

    async fn get_kit_variant_by_owner_ref(
        &mut self,
        owner_ref: &str,
    ) -> Result<Option<contracts::domain::a022_kit_variant::aggregate::KitVariant>> {
        if let Some(value) = self.kit_variant_by_owner_ref.get(owner_ref) {
            return Ok(value.clone());
        }

        let resolved =
            crate::domain::a022_kit_variant::repository::get_main_by_owner_ref(owner_ref).await?;
        self.kit_variant_by_owner_ref
            .insert(owner_ref.to_string(), resolved.clone());
        Ok(resolved)
    }

    async fn resolve_direct_cost(
        &mut self,
        nomenclature_ref: &str,
        sale_date: &str,
    ) -> Result<Option<f64>> {
        let cache_key = (nomenclature_ref.to_string(), sale_date.to_string());
        if let Some(value) = self.direct_cost_by_nom_and_date.get(&cache_key) {
            return Ok(*value);
        }

        let resolved =
            crate::projections::p912_nomenclature_costs::service::resolve_latest_cost_before_date(
                nomenclature_ref,
                sale_date,
            )
            .await?
            .map(|record| record.cost)
            .filter(|cost| *cost > 0.0);
        self.direct_cost_by_nom_and_date.insert(cache_key, resolved);
        Ok(resolved)
    }

    async fn resolve_simple_prod_unit_cost(
        &mut self,
        nomenclature_ref: &str,
        sale_date: &str,
    ) -> Result<Option<f64>> {
        let cache_key = (nomenclature_ref.to_string(), sale_date.to_string());
        if let Some(value) = self.resolved_prod_unit_cost_by_nom_and_date.get(&cache_key) {
            return Ok(*value);
        }

        let mut resolved = self
            .resolve_direct_cost(nomenclature_ref, sale_date)
            .await?;
        if resolved.is_none() {
            let base_ref = self
                .get_nomenclature(nomenclature_ref)
                .await?
                .and_then(|nomenclature| nomenclature.base_nomenclature_ref)
                .filter(|base_ref| {
                    let trimmed = base_ref.trim();
                    !trimmed.is_empty() && trimmed != ZERO_UUID && trimmed != nomenclature_ref
                });
            if let Some(base_ref) = base_ref {
                resolved = self.resolve_direct_cost(&base_ref, sale_date).await?;
            }
        }

        self.resolved_prod_unit_cost_by_nom_and_date
            .insert(cache_key, resolved);
        Ok(resolved)
    }
}

fn set_if_changed<T: PartialEq>(slot: &mut Option<T>, next: Option<T>) -> bool {
    if *slot != next {
        *slot = next;
        true
    } else {
        false
    }
}

fn clear_fact_fields(document: &mut WbSales) -> bool {
    let mut changed = false;
    changed |= set_if_changed(&mut document.line.is_fact, Some(false));
    changed |= set_if_changed(&mut document.line.sell_out_fact, None);
    changed |= set_if_changed(&mut document.line.acquiring_fee_fact, None);
    changed |= set_if_changed(&mut document.line.other_fee_fact, None);
    changed |= set_if_changed(&mut document.line.supplier_payout_fact, None);
    changed |= set_if_changed(&mut document.line.profit_fact, None);
    changed |= set_if_changed(&mut document.line.commission_fact, None);
    changed
}

fn valid_ref(value: Option<String>, current_ref: &str) -> Option<String> {
    value.filter(|raw| {
        let trimmed = raw.trim();
        !trimmed.is_empty() && trimmed != ZERO_UUID && trimmed != current_ref
    })
}

pub async fn preload_prod_cost_context_for_documents(
    cache: &mut PostingPreparationCache,
    documents: &[WbSales],
) -> Result<()> {
    if documents.is_empty() {
        return Ok(());
    }

    let sale_date = documents[0].state.sale_dt.format("%Y-%m-%d").to_string();
    let document_nom_refs: BTreeSet<String> = documents
        .iter()
        .filter_map(|document| document.nomenclature_ref.clone())
        .filter(|value| !value.trim().is_empty())
        .collect();

    if document_nom_refs.is_empty() {
        return Ok(());
    }

    let document_nom_refs_vec = document_nom_refs.iter().cloned().collect::<Vec<_>>();
    for nomenclature in
        crate::domain::a004_nomenclature::repository::list_by_ids(&document_nom_refs_vec).await?
    {
        cache
            .nomenclature_by_id
            .insert(nomenclature.base.id.as_string(), Some(nomenclature));
    }

    let owner_refs = document_nom_refs
        .iter()
        .filter(|owner_ref| {
            cache
                .nomenclature_by_id
                .get(owner_ref.as_str())
                .and_then(|item| item.as_ref())
                .is_some_and(|nomenclature| nomenclature.is_assembly)
        })
        .cloned()
        .collect::<Vec<_>>();

    if !owner_refs.is_empty() {
        let variants =
            crate::domain::a022_kit_variant::repository::list_main_by_owner_refs(&owner_refs)
                .await?;
        for owner_ref in &owner_refs {
            cache
                .kit_variant_by_owner_ref
                .insert(owner_ref.clone(), variants.get(owner_ref).cloned());
        }
    }

    let component_refs: BTreeSet<String> = owner_refs
        .iter()
        .filter_map(|owner_ref| cache.kit_variant_by_owner_ref.get(owner_ref))
        .filter_map(|variant| variant.as_ref())
        .flat_map(|variant| variant.parse_goods().into_iter())
        .map(|item| item.nomenclature_ref)
        .filter(|value| !value.trim().is_empty())
        .collect();

    if !component_refs.is_empty() {
        let component_refs_vec = component_refs.iter().cloned().collect::<Vec<_>>();
        for nomenclature in
            crate::domain::a004_nomenclature::repository::list_by_ids(&component_refs_vec).await?
        {
            cache
                .nomenclature_by_id
                .insert(nomenclature.base.id.as_string(), Some(nomenclature));
        }
    }

    let mut refs_to_resolve = BTreeSet::new();
    for nomenclature_ref in document_nom_refs.iter().chain(component_refs.iter()) {
        refs_to_resolve.insert(nomenclature_ref.clone());
        if let Some(base_ref) = cache
            .nomenclature_by_id
            .get(nomenclature_ref)
            .and_then(|item| item.as_ref())
            .and_then(|nomenclature| {
                valid_ref(nomenclature.base_nomenclature_ref.clone(), nomenclature_ref)
            })
        {
            refs_to_resolve.insert(base_ref);
        }
    }

    if refs_to_resolve.is_empty() {
        return Ok(());
    }

    let direct_costs =
        crate::projections::p912_nomenclature_costs::service::resolve_latest_costs_before_date(
            &refs_to_resolve.iter().cloned().collect::<Vec<_>>(),
            &sale_date,
        )
        .await?;

    for nomenclature_ref in &refs_to_resolve {
        let cache_key = (nomenclature_ref.clone(), sale_date.clone());
        let direct_cost = direct_costs
            .get(nomenclature_ref)
            .map(|record| record.cost)
            .filter(|cost| *cost > 0.0);
        cache
            .direct_cost_by_nom_and_date
            .insert(cache_key.clone(), direct_cost);
        if let Some(cost) = direct_cost {
            cache
                .resolved_prod_unit_cost_by_nom_and_date
                .insert(cache_key, Some(cost));
        }
    }

    for nomenclature_ref in document_nom_refs.iter().chain(component_refs.iter()) {
        let cache_key = (nomenclature_ref.clone(), sale_date.clone());
        if cache
            .resolved_prod_unit_cost_by_nom_and_date
            .contains_key(&cache_key)
        {
            continue;
        }

        let direct_cost = cache
            .direct_cost_by_nom_and_date
            .get(&cache_key)
            .copied()
            .flatten();
        let resolved = direct_cost.or_else(|| {
            cache
                .nomenclature_by_id
                .get(nomenclature_ref)
                .and_then(|item| item.as_ref())
                .and_then(|nomenclature| {
                    valid_ref(nomenclature.base_nomenclature_ref.clone(), nomenclature_ref)
                })
                .and_then(|base_ref| {
                    cache
                        .direct_cost_by_nom_and_date
                        .get(&(base_ref, sale_date.clone()))
                        .copied()
                        .flatten()
                })
        });
        cache
            .resolved_prod_unit_cost_by_nom_and_date
            .insert(cache_key, resolved);
    }

    Ok(())
}

pub async fn resolve_prod_item_cost_total_cached(
    document: &WbSales,
    cache: &mut PostingPreparationCache,
) -> Result<Option<f64>> {
    let Some(nomenclature_ref) = document.nomenclature_ref.as_deref() else {
        return Ok(None);
    };
    if nomenclature_ref.trim().is_empty() {
        return Ok(None);
    }

    let sale_date = document.state.sale_dt.format("%Y-%m-%d").to_string();
    let Some(nomenclature) = cache.get_nomenclature(nomenclature_ref).await? else {
        tracing::warn!(
            "Skip prod item_cost for WB Sales {}: nomenclature not found {}",
            document.base.id.as_string(),
            nomenclature_ref
        );
        return Ok(None);
    };

    if !nomenclature.is_assembly {
        let resolved = cache
            .resolve_simple_prod_unit_cost(nomenclature_ref, &sale_date)
            .await?;
        if resolved.is_none() {
            tracing::warn!(
                "Skip prod item_cost for WB Sales {}: no p912 cost for nomenclature {} on {}",
                document.base.id.as_string(),
                nomenclature_ref,
                sale_date
            );
        }
        return Ok(resolved.map(|unit_cost| unit_cost * document.line.qty.abs()));
    }

    let Some(kit_variant) = cache.get_kit_variant_by_owner_ref(nomenclature_ref).await? else {
        tracing::warn!(
            "Skip prod item_cost for WB Sales {}: kit variant not found for owner_ref {}",
            document.base.id.as_string(),
            nomenclature_ref
        );
        return Ok(None);
    };

    let goods = kit_variant.parse_goods();
    if goods.is_empty() {
        tracing::warn!(
            "Skip prod item_cost for WB Sales {}: empty kit composition for owner_ref {}",
            document.base.id.as_string(),
            nomenclature_ref
        );
        return Ok(None);
    }

    let mut unit_cost = 0.0;
    let mut missing_components = Vec::new();
    for component in goods {
        match cache
            .resolve_simple_prod_unit_cost(&component.nomenclature_ref, &sale_date)
            .await?
        {
            Some(component_cost) => unit_cost += component_cost * component.quantity,
            None => missing_components.push(component.nomenclature_ref),
        }
    }

    if !missing_components.is_empty() {
        tracing::warn!(
            "Skip prod item_cost for WB Sales {}: missing p912 cost for kit {} components {:?} on {}",
            document.base.id.as_string(),
            nomenclature_ref,
            missing_components,
            sale_date
        );
        return Ok(None);
    }

    Ok(Some(unit_cost * document.line.qty.abs()))
}

pub async fn sync_organization_from_connection_cached(
    document: &mut WbSales,
    cache: &mut PostingPreparationCache,
) -> Result<bool> {
    let should_sync = document.header.organization_id.trim().is_empty()
        || match cache.get_connection(&document.header.connection_id).await? {
            Some(connection) => {
                let organization_ref = connection.organization_ref.trim().trim_matches('"');
                organization_ref != document.header.organization_id
            }
            None => false,
        };

    if !should_sync {
        return Ok(false);
    }

    let Some(connection) = cache.get_connection(&document.header.connection_id).await? else {
        tracing::warn!(
            "Skip organization sync for WB Sales {}: connection not found, connection_id={}",
            document.base.id.value(),
            document.header.connection_id
        );
        return Ok(false);
    };

    let organization_ref = connection.organization_ref.trim().trim_matches('"');
    let organization_uuid = match Uuid::parse_str(organization_ref) {
        Ok(uuid) => uuid,
        Err(_) => {
            tracing::warn!(
                "Skip organization sync for WB Sales {}: invalid organization_ref={}",
                document.base.id.value(),
                connection.organization_ref
            );
            return Ok(false);
        }
    };

    if crate::domain::a002_organization::service::get_by_id(organization_uuid)
        .await?
        .is_none()
    {
        tracing::warn!(
            "Skip organization sync for WB Sales {}: organization_ref not found={}",
            document.base.id.value(),
            connection.organization_ref
        );
        return Ok(false);
    }

    let resolved_org_id = organization_uuid.to_string();
    if document.header.organization_id != resolved_org_id {
        document.header.organization_id = resolved_org_id;
        return Ok(true);
    }

    Ok(false)
}

pub async fn auto_fill_references_cached(
    document: &mut WbSales,
    cache: &mut PostingPreparationCache,
) -> Result<bool> {
    let mut changed = false;

    if document.marketplace_product_ref.is_none() {
        if let Some(marketplace_sku) =
            crate::domain::a007_marketplace_product::service::wb_marketplace_sku(
                document.line.nm_id,
            )
        {
            let title = if document.line.name.trim().is_empty() {
                format!("Артикул: {}", document.line.supplier_article)
            } else {
                document.line.name.clone()
            };

            let mp_id = crate::domain::a007_marketplace_product::service::find_or_create_for_sale(
                crate::domain::a007_marketplace_product::service::FindOrCreateParams {
                    marketplace_ref: document.header.marketplace_id.clone(),
                    connection_mp_ref: document.header.connection_id.clone(),
                    marketplace_sku,
                    article: Some(document.line.supplier_article.clone()),
                    barcode: Some(document.line.barcode.clone()),
                    title,
                },
            )
            .await?;

            document.marketplace_product_ref = Some(mp_id.to_string());
            changed = true;
        }
    }

    if document.nomenclature_ref.is_none() {
        if let Some(mp_ref) = document.marketplace_product_ref.as_deref() {
            if let Some(mp) = cache.get_marketplace_product(mp_ref).await? {
                if let Some(nom_ref) = mp.nomenclature_ref {
                    document.nomenclature_ref = Some(nom_ref);
                    changed = true;
                }
            }
        }
    }

    Ok(changed)
}

pub async fn fill_dealer_price_resolved_cached(
    document: &mut WbSales,
    cache: &mut PostingPreparationCache,
) -> Result<bool> {
    let Some(nom_ref) = document.nomenclature_ref.as_deref() else {
        let mut changed = false;
        changed |= set_if_changed(&mut document.line.dealer_price_ut, None);
        changed |= set_if_changed(&mut document.line.cost_of_production, None);
        return Ok(changed);
    };

    if document.line.cost_of_production.unwrap_or(0.0) > 0.0
        && document.line.dealer_price_ut.unwrap_or(0.0) > 0.0
    {
        return Ok(false);
    }

    let sale_date = document.state.sale_dt.format("%Y-%m-%d").to_string();
    let resolved = cache.resolve_price(nom_ref, &sale_date).await?;

    if let Some(ref resolved_price) = resolved {
        tracing::info!(
            "Filled dealer_price_ut = {:?} for document {} (from {})",
            resolved_price.price,
            document.base.id.as_string(),
            resolved_price.describe(&sale_date)
        );
    } else {
        tracing::warn!(
            "Could not find dealer_price_ut for document {} (nomenclature: {})",
            document.base.id.as_string(),
            nom_ref
        );
    }

    let resolved_price = resolved.map(|value| value.price);
    let mut changed = false;

    if document.line.dealer_price_ut.unwrap_or(0.0) <= 0.0 {
        changed |= set_if_changed(&mut document.line.dealer_price_ut, resolved_price);
    }
    if document.line.cost_of_production.unwrap_or(0.0) <= 0.0 {
        changed |= set_if_changed(&mut document.line.cost_of_production, resolved_price);
    }

    Ok(changed)
}

pub async fn calculate_plan_fields_cached(
    document: &mut WbSales,
    cache: &mut PostingPreparationCache,
) -> Result<bool> {
    let acquiring_fee_pro = cache
        .get_acquiring_fee_rate(&document.header.marketplace_id)
        .await?;

    let finished_price = document.line.finished_price.unwrap_or(0.0);
    let amount_line = document.line.amount_line.unwrap_or(0.0);
    let cost_of_production = document.line.cost_of_production.unwrap_or(0.0);

    let acquiring_fee_plan = acquiring_fee_pro * finished_price / 100.0;
    let commission_plan = finished_price - amount_line;
    let other_fee_plan = 0.0;
    let supplier_payout_plan = amount_line - acquiring_fee_plan;
    let profit_plan =
        finished_price - acquiring_fee_plan - commission_plan - other_fee_plan - cost_of_production;

    let mut changed = clear_fact_fields(document);
    changed |= set_if_changed(&mut document.line.sell_out_plan, Some(finished_price));
    changed |= set_if_changed(
        &mut document.line.acquiring_fee_plan,
        Some(acquiring_fee_plan),
    );
    changed |= set_if_changed(&mut document.line.other_fee_plan, Some(other_fee_plan));
    changed |= set_if_changed(
        &mut document.line.supplier_payout_plan,
        Some(supplier_payout_plan),
    );
    changed |= set_if_changed(&mut document.line.commission_plan, Some(commission_plan));
    changed |= set_if_changed(&mut document.line.profit_plan, Some(profit_plan));
    Ok(changed)
}

pub async fn prepare_document_for_posting_cached(
    document: &mut WbSales,
    cache: &mut PostingPreparationCache,
) -> Result<bool> {
    let mut changed = false;
    changed |= sync_organization_from_connection_cached(document, cache).await?;
    changed |= auto_fill_references_cached(document, cache).await?;
    changed |= fill_dealer_price_resolved_cached(document, cache).await?;
    changed |= calculate_plan_fields_cached(document, cache).await?;
    Ok(changed)
}

pub async fn calculate_plan_fields(document: &mut WbSales) -> Result<()> {
    let mut cache = PostingPreparationCache::default();
    calculate_plan_fields_cached(document, &mut cache).await?;
    Ok(())
}

pub async fn auto_fill_references(document: &mut WbSales) -> Result<()> {
    let mut cache = PostingPreparationCache::default();
    auto_fill_references_cached(document, &mut cache).await?;
    Ok(())
}

pub async fn fill_dealer_price(document: &mut WbSales) -> Result<()> {
    fill_dealer_price_resolved(document).await
}

pub async fn fill_dealer_price_resolved(document: &mut WbSales) -> Result<()> {
    let mut cache = PostingPreparationCache::default();
    fill_dealer_price_resolved_cached(document, &mut cache).await?;
    Ok(())
}

pub async fn prepare_document_for_posting(document: &mut WbSales) -> Result<bool> {
    let mut cache = PostingPreparationCache::default();
    prepare_document_for_posting_cached(document, &mut cache).await
}

pub async fn store_document_with_raw(mut document: WbSales, raw_json: &str) -> Result<Uuid> {
    let raw_ref = crate::shared::data::raw_storage::save_raw_json(
        "WB",
        "WB_Sales",
        &document.header.document_no,
        raw_json,
        document.source_meta.fetched_at,
    )
    .await?;

    document.source_meta.raw_payload_ref = raw_ref;
    prepare_document_for_posting(&mut document).await?;

    document
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", e))?;
    document.before_write();

    let id = repository::upsert_document(&document).await?;

    if document.is_posted {
        if let Err(e) = super::posting::post_document(id).await {
            tracing::error!("Failed to post WB Sales document: {}", e);
        }
    } else {
        if let Err(e) = crate::projections::p900_mp_sales_register::service::delete_by_registrator(
            &id.to_string(),
        )
        .await
        {
            tracing::error!("Failed to delete projections for WB Sales document: {}", e);
        }
        if let Err(e) =
            crate::projections::p904_sales_data::repository::delete_by_registrator(&id.to_string())
                .await
        {
            tracing::error!(
                "Failed to delete P904 projections for WB Sales document: {}",
                e
            );
        }
    }

    Ok(id)
}

pub async fn get_by_id(id: Uuid) -> Result<Option<WbSales>> {
    repository::get_by_id(id).await
}

pub async fn get_by_document_no(document_no: &str) -> Result<Option<WbSales>> {
    repository::get_by_document_no(document_no).await
}

pub async fn get_by_sale_id(sale_id: &str) -> Result<Option<WbSales>> {
    repository::get_by_sale_id(sale_id).await
}

pub async fn list_all() -> Result<Vec<WbSales>> {
    repository::list_all().await
}

pub async fn list_by_sale_date_range(
    date_from: Option<chrono::NaiveDate>,
    date_to: Option<chrono::NaiveDate>,
) -> Result<Vec<WbSales>> {
    repository::list_by_sale_date_range(date_from, date_to).await
}

pub async fn delete(id: Uuid) -> Result<bool> {
    repository::soft_delete(id).await
}

pub async fn refresh_dealer_price(id: Uuid) -> Result<()> {
    let mut document = get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Document not found: {}", id))?;

    document.line.dealer_price_ut = None;
    document.line.cost_of_production = None;
    fill_dealer_price_resolved(&mut document).await?;
    calculate_plan_fields(&mut document).await?;
    repository::upsert_document(&document).await?;

    tracing::info!(
        "Refreshed dealer_price_ut for document {}: {:?}",
        id,
        document.line.dealer_price_ut
    );

    Ok(())
}
