use super::repository;
use contracts::domain::a004_nomenclature::aggregate::{Nomenclature, NomenclatureDto};
use uuid::Uuid;

fn normalize_validation_error(message: &str) -> String {
    match message {
        "Р¤РѕСЂРјР°С‚ РЅРµ РґРѕР»Р¶РµРЅ РїСЂРµРІС‹С€Р°С‚СЊ 20 СЃРёРјРІРѕР»РѕРІ" => {
            "РџРѕР»Рµ В«Р¤РѕСЂРјР°С‚В» РІ Р±Р»РѕРєРµ РёР·РјРµСЂРµРЅРёР№ РЅРµ РґРѕР»Р¶РЅРѕ РїСЂРµРІС‹С€Р°С‚СЊ 20 СЃРёРјРІРѕР»РѕРІ".to_string()
        }
        "Р Р°Р·РјРµСЂ РЅРµ РґРѕР»Р¶РµРЅ РїСЂРµРІС‹С€Р°С‚СЊ 20 СЃРёРјРІРѕР»РѕРІ" => {
            "РџРѕР»Рµ В«Р Р°Р·РјРµСЂВ» РІ Р±Р»РѕРєРµ РёР·РјРµСЂРµРЅРёР№ РЅРµ РґРѕР»Р¶РЅРѕ РїСЂРµРІС‹С€Р°С‚СЊ 20 СЃРёРјРІРѕР»РѕРІ".to_string()
        }
        "РљР°С‚РµРіРѕСЂРёСЏ РЅРµ РґРѕР»Р¶РЅР° РїСЂРµРІС‹С€Р°С‚СЊ 40 СЃРёРјРІРѕР»РѕРІ" => {
            "РџРѕР»Рµ В«РљР°С‚РµРіРѕСЂРёСЏВ» РІ Р±Р»РѕРєРµ РёР·РјРµСЂРµРЅРёР№ РЅРµ РґРѕР»Р¶РЅРѕ РїСЂРµРІС‹С€Р°С‚СЊ 40 СЃРёРјРІРѕР»РѕРІ".to_string()
        }
        "Р›РёРЅРµР№РєР° РЅРµ РґРѕР»Р¶РЅР° РїСЂРµРІС‹С€Р°С‚СЊ 40 СЃРёРјРІРѕР»РѕРІ" => {
            "РџРѕР»Рµ В«Р›РёРЅРµР№РєР°В» РІ Р±Р»РѕРєРµ РёР·РјРµСЂРµРЅРёР№ РЅРµ РґРѕР»Р¶РЅРѕ РїСЂРµРІС‹С€Р°С‚СЊ 40 СЃРёРјРІРѕР»РѕРІ".to_string()
        }
        "РњРѕРґРµР»СЊ РЅРµ РґРѕР»Р¶РЅР° РїСЂРµРІС‹С€Р°С‚СЊ 80 СЃРёРјРІРѕР»РѕРІ" => {
            "РџРѕР»Рµ В«РњРѕРґРµР»СЊВ» РІ Р±Р»РѕРєРµ РёР·РјРµСЂРµРЅРёР№ РЅРµ РґРѕР»Р¶РЅРѕ РїСЂРµРІС‹С€Р°С‚СЊ 80 СЃРёРјРІРѕР»РѕРІ".to_string()
        }
        "Р Р°РєРѕРІРёРЅР° РЅРµ РґРѕР»Р¶РЅР° РїСЂРµРІС‹С€Р°С‚СЊ 40 СЃРёРјРІРѕР»РѕРІ" => {
            "РџРѕР»Рµ В«Р Р°РєРѕРІРёРЅР°В» РІ Р±Р»РѕРєРµ РёР·РјРµСЂРµРЅРёР№ РЅРµ РґРѕР»Р¶РЅРѕ РїСЂРµРІС‹С€Р°С‚СЊ 40 СЃРёРјРІРѕР»РѕРІ".to_string()
        }
        _ => message.to_string(),
    }
}

pub async fn create(dto: NomenclatureDto) -> anyhow::Result<Uuid> {
    let code = dto
        .code
        .clone()
        .unwrap_or_else(|| format!("NOM-{}", Uuid::new_v4()));
    let mut aggregate = Nomenclature::new_for_insert(
        code,
        dto.description,
        dto.full_description.unwrap_or_default(),
        dto.is_folder,
        dto.parent_id,
        dto.article.unwrap_or_default(),
        dto.comment,
    );
    aggregate.alternative_cost_source_ref = dto.alternative_cost_source_ref.clone();
    aggregate.base_nomenclature_ref = dto.base_nomenclature_ref.clone();
    aggregate.kit_variant_ref = dto.kit_variant_ref.clone();
    aggregate.is_assembly = dto.is_assembly.unwrap_or(false);
    aggregate.is_derivative = aggregate.compute_is_derivative();

    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", normalize_validation_error(&e)))?;
    aggregate.before_write();

    repository::insert(&aggregate).await
}

pub async fn update(dto: NomenclatureDto) -> anyhow::Result<()> {
    let id = dto
        .id
        .as_ref()
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| anyhow::anyhow!("Invalid ID"))?;

    let mut aggregate = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Not found"))?;

    aggregate.update(&dto);

    aggregate
        .validate()
        .map_err(|e| anyhow::anyhow!("Validation failed: {}", normalize_validation_error(&e)))?;
    aggregate.before_write();

    repository::update(&aggregate).await
}

pub async fn delete(id: Uuid) -> anyhow::Result<bool> {
    repository::soft_delete(id).await
}

pub async fn get_by_id(id: Uuid) -> anyhow::Result<Option<Nomenclature>> {
    repository::get_by_id(id).await
}

pub async fn list_all() -> anyhow::Result<Vec<Nomenclature>> {
    repository::list_all().await
}

pub async fn list_paginated(
    limit: u64,
    offset: u64,
    sort_by: &str,
    sort_desc: bool,
    q: &str,
    only_mp: bool,
) -> anyhow::Result<(Vec<Nomenclature>, u64)> {
    repository::list_paginated(limit, offset, sort_by, sort_desc, q, only_mp).await
}

pub async fn sync_kit_variant_links(
) -> anyhow::Result<super::kit_variant_link_sync::KitVariantLinkSyncStats> {
    super::kit_variant_link_sync::sync_links().await
}
