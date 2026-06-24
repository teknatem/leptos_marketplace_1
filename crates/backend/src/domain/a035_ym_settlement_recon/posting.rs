//! Проведение сверки перечислений YM (a035). При проведении в проекцию
//! `p915_mp_order_events` пишутся события «Дата оплаты поставщику» (`supplier_payment`)
//! по каждому заказу, оплаченному в банковском ордере; дата = bank_order_date.
//! Идемпотентно: перед вставкой удаляются прежние события этого документа.

use anyhow::Result;
use sea_orm::TransactionTrait;
use uuid::Uuid;

use crate::shared::data::db::get_connection;

use super::repository;

const REGISTRATOR_TYPE: &str = "a035_ym_settlement_recon";

pub async fn post_document(id: Uuid) -> Result<()> {
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("a035 document not found: {}", id))?;

    document.base.metadata.is_posted = true;
    document.before_write();

    // Расчёты ордера — из p907 («Платёж покупателя» и «Возврат платежа покупателя»).
    let settled = repository::settled_orders(
        &document.header.connection_id,
        document.header.bank_order_id,
    )
    .await?;
    let order_events: Vec<
        crate::projections::p915_mp_order_events::builder::SettledOrderEvent<'_>,
    > = settled
        .iter()
        .map(
            |s| crate::projections::p915_mp_order_events::builder::SettledOrderEvent {
                order_id: &s.order_id,
                amount: s.amount,
                is_return: s.is_return,
            },
        )
        .collect();
    let events = crate::projections::p915_mp_order_events::builder::from_supplier_settlement(
        &order_events,
        &document.header.bank_order_date,
        &document.header.connection_id,
        REGISTRATOR_TYPE,
        &id.to_string(),
    );

    let db = get_connection();
    let txn = db.begin().await?;

    repository::update_with_conn(&txn, &document).await?;

    crate::projections::p915_mp_order_events::repository::delete_by_registrator_with_conn(
        &txn,
        REGISTRATOR_TYPE,
        &id.to_string(),
    )
    .await?;
    for event in &events {
        crate::projections::p915_mp_order_events::repository::insert_entry_raw_with_conn(
            &txn, event,
        )
        .await?;
    }

    txn.commit().await?;
    tracing::info!(
        "a035 проведён: ордер {}, событий supplier_payment: {}",
        document.header.bank_order_id,
        events.len()
    );
    Ok(())
}

pub async fn unpost_document(id: Uuid) -> Result<()> {
    let mut document = repository::get_by_id(id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("a035 document not found: {}", id))?;

    document.base.metadata.is_posted = false;
    document.before_write();

    let db = get_connection();
    let txn = db.begin().await?;

    repository::update_with_conn(&txn, &document).await?;
    crate::projections::p915_mp_order_events::repository::delete_by_registrator_with_conn(
        &txn,
        REGISTRATOR_TYPE,
        &id.to_string(),
    )
    .await?;

    txn.commit().await?;
    tracing::info!(
        "a035 отмена проведения: ордер {}",
        document.header.bank_order_id
    );
    Ok(())
}
