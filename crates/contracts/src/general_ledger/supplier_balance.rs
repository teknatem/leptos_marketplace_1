//! DTO отчёта «Баланс к перечислению поставщику (YM)».
//!
//! Сальдо счёта расчётов 7609 в контуре entity=ym = сумма, которую маркетплейс
//! ещё должен перечислить поставщику. Отчёт показывает движение за период:
//! входящее сальдо → +начислено → −удержано → −перечислено → исходящее (= доступно
//! к перечислению), плюс отдельный баланс кошелька баллов/промо (76YB).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SupplierBalanceQuery {
    pub date_from: String,
    pub date_to: String,
    /// Кабинет маркетплейса; None — по всем кабинетам контура.
    pub connection_mp_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplierBalanceResponse {
    pub entity: String,
    /// Денежный счёт расчётов (7609).
    pub account: String,
    /// Сальдо 7609 на начало периода (всё до date_from).
    pub opening_balance: f64,
    /// Начислено за период (увеличения 7609: выручка/прочие доходы).
    pub accrued: f64,
    /// Удержано за период (комиссии/логистика/услуги/штрафы и пр., без перечислений).
    pub deductions: f64,
    /// Перечислено за период на расчётный счёт (оборот ym_settlement).
    pub settled: f64,
    /// Чистое движение за период = accrued − deductions − settled.
    pub period_net: f64,
    /// Сальдо 7609 на конец периода = доступно к перечислению поставщику.
    pub closing_balance: f64,
    /// Сальдо кошелька баллов/промо (76YB) на конец периода — справочно.
    pub points_balance: f64,
}
