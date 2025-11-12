use anyhow::{anyhow, Result};
use chrono::Utc;

use super::repository::{NomenclatureBarcodeEntry, Model, BarcodeWithNomenclature};

/// Валидация штрихкода
pub fn validate_barcode(barcode: &str) -> Result<()> {
    if barcode.is_empty() {
        return Err(anyhow!("Barcode cannot be empty"));
    }

    if barcode.len() > 100 {
        return Err(anyhow!("Barcode is too long (max 100 characters)"));
    }

    // Проверка на допустимые символы (цифры, буквы, дефисы)
    if !barcode.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(anyhow!("Barcode contains invalid characters"));
    }

    Ok(())
}

/// Создать entry для upsert
pub fn create_entry(
    barcode: String,
    source: String,
    nomenclature_ref: Option<String>,
    article: Option<String>,
) -> Result<NomenclatureBarcodeEntry> {
    validate_barcode(&barcode)?;

    let now = Utc::now();

    Ok(NomenclatureBarcodeEntry {
        barcode,
        source,
        nomenclature_ref,
        article,
        created_at: now,
        updated_at: now,
        is_active: true,
    })
}

/// Обновить entry с новым updated_at
pub fn update_entry(
    existing: &Model,
    nomenclature_ref: Option<String>,
    article: Option<String>,
) -> NomenclatureBarcodeEntry {
    NomenclatureBarcodeEntry {
        barcode: existing.barcode.clone(),
        source: existing.source.clone(),
        nomenclature_ref: nomenclature_ref.or_else(|| existing.nomenclature_ref.clone()),
        article: article.or_else(|| existing.article.clone()),
        created_at: existing.created_at.parse().unwrap_or_else(|_| Utc::now()),
        updated_at: Utc::now(),
        is_active: existing.is_active,
    }
}

/// Конвертировать Model в DTO
pub fn model_to_dto(model: &Model) -> contracts::projections::p901_nomenclature_barcodes::NomenclatureBarcodeDto {
    contracts::projections::p901_nomenclature_barcodes::NomenclatureBarcodeDto {
        barcode: model.barcode.clone(),
        nomenclature_ref: model.nomenclature_ref.clone(),
        nomenclature_name: None,  // Для простых запросов без JOIN
        article: model.article.clone(),
        source: model.source.clone(),
        created_at: model.created_at.parse().unwrap_or_else(|_| Utc::now()),
        updated_at: model.updated_at.parse().unwrap_or_else(|_| Utc::now()),
        is_active: model.is_active,
    }
}

/// Конвертировать список Models в DTOs
pub fn models_to_dtos(models: Vec<Model>) -> Vec<contracts::projections::p901_nomenclature_barcodes::NomenclatureBarcodeDto> {
    models.iter().map(model_to_dto).collect()
}

/// Конвертировать BarcodeWithNomenclature в DTO
pub fn barcode_with_nomenclature_to_dto(model: &BarcodeWithNomenclature) -> contracts::projections::p901_nomenclature_barcodes::NomenclatureBarcodeDto {
    contracts::projections::p901_nomenclature_barcodes::NomenclatureBarcodeDto {
        barcode: model.barcode.clone(),
        nomenclature_ref: model.nomenclature_ref.clone(),
        nomenclature_name: model.nomenclature_name.clone(),
        article: model.article.clone(),
        source: model.source.clone(),
        created_at: model.created_at.parse().unwrap_or_else(|_| Utc::now()),
        updated_at: model.updated_at.parse().unwrap_or_else(|_| Utc::now()),
        is_active: model.is_active,
    }
}

/// Конвертировать список BarcodeWithNomenclature в DTOs
pub fn barcodes_with_nomenclature_to_dtos(models: Vec<BarcodeWithNomenclature>) -> Vec<contracts::projections::p901_nomenclature_barcodes::NomenclatureBarcodeDto> {
    models.iter().map(barcode_with_nomenclature_to_dto).collect()
}

/// Найти nomenclature_ref по штрихкоду из источника 1C
/// Используется при импорте из маркетплейсов для автоматической привязки к номенклатуре
pub async fn find_nomenclature_ref_by_barcode_from_1c(barcode: &str) -> Result<Option<String>> {
    use super::repository;

    // Ищем запись с source='1C' для данного штрихкода
    let result = repository::get_by_barcode_and_source(barcode, "1C").await?;

    Ok(result.and_then(|model| model.nomenclature_ref))
}

/// Найти nomenclature_ref по штрихкоду из конкретного маркетплейса
/// Сначала ищет по указанному источнику, потом по '1C' в качестве fallback
pub async fn find_nomenclature_ref_by_barcode_from_marketplace(
    barcode: &str,
    marketplace_source: &str,
) -> Result<Option<String>> {
    use super::repository;

    // Сначала пробуем найти по источнику маркетплейса
    if let Some(model) = repository::get_by_barcode_and_source(barcode, marketplace_source).await? {
        if let Some(ref nomenclature_ref) = model.nomenclature_ref {
            return Ok(Some(nomenclature_ref.clone()));
        }
    }

    // Если не нашли или nomenclature_ref=NULL, пробуем 1C
    find_nomenclature_ref_by_barcode_from_1c(barcode).await
}
