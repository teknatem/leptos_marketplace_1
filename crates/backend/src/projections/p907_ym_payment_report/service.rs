use anyhow::Result;
use contracts::domain::common::AggregateId;
use sea_orm::{EntityTrait, Set, TransactionTrait};

use crate::shared::data::db::get_connection;

pub async fn rebuild_entry_from_existing(id: &str) -> Result<usize> {
    let Some(row) =
        crate::projections::p907_ym_payment_report::repository::get_by_uuid(id).await?
    else {
        return Ok(0);
    };
    rebuild_from_row(row).await
}

/// Перепровести уже загруженную строку p907: дозаполнить производные ссылки и
/// перестроить GL/p914. Принимает строку напрямую, избегая повторного SELECT
/// (вызывается и по `id`, и по `record_key`).
async fn rebuild_from_row(
    mut row: crate::projections::p907_ym_payment_report::repository::Model,
) -> Result<usize> {
    // Первый этап проведения: дозаполнить производные ссылки (если пусто) и
    // сохранить в строке p907 — далее они просто копируются в p914.
    resolve_and_persist_marketplace_refs(&mut row).await?;

    let db = get_connection();
    let txn = db.begin().await?;

    crate::general_ledger::repository::delete_by_registrator_with_conn(
        &txn,
        "p907_ym_payment_report",
        &row.id,
    )
    .await?;
    crate::projections::p914_mp_finance_turnovers::repository::delete_by_registrator_refs_with_conn(
        &txn,
        std::slice::from_ref(&row.id),
    )
    .await?;

    let general_ledger_entries =
        crate::projections::p907_ym_payment_report::general_ledger_builder::build_general_ledger_entries(
            &row,
            "",
        )?;
    for entry in &general_ledger_entries {
        crate::general_ledger::repository::save_entry_with_conn(&txn, entry).await?;
    }

    let finance_turnovers =
        crate::projections::p907_ym_payment_report::general_ledger_builder::build_finance_turnover_entries(
            &row,
            &general_ledger_entries,
        );
    for turnover in &finance_turnovers {
        crate::projections::p914_mp_finance_turnovers::repository::save_entry_with_conn(
            &txn, turnover,
        )
        .await?;
    }

    txn.commit().await?;

    Ok(general_ledger_entries.len())
}

/// Резолвит и заполняет производные ссылки строки p907 и сохраняет изменения в БД:
/// `marketplace_product_ref` (a007 по shop_sku) и `marketplace_order_ref`
/// (a013_ym_order по order_id) — резолвятся только если ещё пусто.
/// `nomenclature_ref` — зеркало `a007.nomenclature_ref` по marketplace_product_ref;
/// перерезолвится на каждом проведении, чтобы отражать актуальную привязку a007 к
/// номенклатуре 1С (она может появиться позже через сопоставление u505), по аналогии
/// с WB-веткой (p903 → resolve_wb_nomenclature_ref). Все три копируются затем в p914.
async fn resolve_and_persist_marketplace_refs(
    row: &mut crate::projections::p907_ym_payment_report::repository::Model,
) -> Result<()> {
    use crate::projections::p907_ym_payment_report::repository::{ActiveModel, Entity};

    let mut changed = false;

    if row.marketplace_product_ref.is_none() {
        if let Some(sku) = row.shop_sku.as_deref().map(str::trim).filter(|v| !v.is_empty()) {
            if let Some(mp_ref) =
                crate::domain::a007_marketplace_product::service::resolve_marketplace_product_ref(
                    &row.connection_mp_ref,
                    sku,
                    None,
                )
                .await?
            {
                row.marketplace_product_ref = Some(mp_ref);
                changed = true;
            }
        }
    }

    if row.marketplace_order_ref.is_none() {
        if let Some(order_id) = row.order_id {
            if let Some(order) =
                crate::domain::a013_ym_order::repository::get_by_document_no(&order_id.to_string())
                    .await?
            {
                row.marketplace_order_ref = Some(order.base.id.as_string());
                changed = true;
            }
        }
    }

    // Зеркалим актуальную привязку a007 → номенклатура 1С (всегда, не только если пусто).
    let nomenclature_ref = match row
        .marketplace_product_ref
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .and_then(|v| uuid::Uuid::parse_str(v).ok())
    {
        Some(mp_id) => crate::domain::a007_marketplace_product::service::get_by_id(mp_id)
            .await?
            .and_then(|product| product.nomenclature_ref)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        None => None,
    };
    if row.nomenclature_ref != nomenclature_ref {
        row.nomenclature_ref = nomenclature_ref;
        changed = true;
    }

    if changed {
        let am = ActiveModel {
            record_key: Set(row.record_key.clone()),
            marketplace_product_ref: Set(row.marketplace_product_ref.clone()),
            marketplace_order_ref: Set(row.marketplace_order_ref.clone()),
            nomenclature_ref: Set(row.nomenclature_ref.clone()),
            ..Default::default()
        };
        Entity::update(am).exec(get_connection()).await?;
    }

    Ok(())
}

/// Массовое перепроведение всех существующих строк p907: для каждой строки
/// перерезолвит ссылки и перестроит GL/p914. Возвращает (число обработанных строк,
/// суммарное число GL-проводок). Используется после изменения маппинга оборотов,
/// чтобы провести ранее не отражавшиеся операции.
pub async fn repost_all() -> Result<(usize, usize)> {
    let ids = crate::projections::p907_ym_payment_report::repository::list_all_ids().await?;
    let mut rows = 0usize;
    let mut gl_entries = 0usize;
    for id in ids {
        gl_entries += rebuild_entry_from_existing(&id).await?;
        rows += 1;
    }
    Ok((rows, gl_entries))
}

pub async fn rebuild_record_key_from_existing(record_key: &str) -> Result<usize> {
    let Some(row) =
        crate::projections::p907_ym_payment_report::repository::get_by_record_key(record_key)
            .await?
    else {
        return Ok(0);
    };

    // Строка уже загружена — перестраиваем напрямую, без повторного SELECT по id.
    rebuild_from_row(row).await
}
