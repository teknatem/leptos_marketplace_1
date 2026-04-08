//! Статические конфигурации GL-ведомостей по счетам.
//!
//! Каждая `GlAccountViewDef` описывает, какие пары (turnover_code, layer) входят в
//! **основной блок** ведомости (суммируются). Всё остальное попадает в
//! **информационный блок** (для сверки, в итоги не включается).
//!
//! `layer = ""` означает «любой слой» для данного оборота.

// ─────────────────────────────────────────────────────────────────────────────
// Типы
// ─────────────────────────────────────────────────────────────────────────────

pub struct GlAccountViewEntry {
    pub turnover_code: &'static str,
    /// Конкретный слой ("fact", "oper", "plan") или "" — совпадает с любым.
    pub layer: &'static str,
}

pub struct GlAccountViewDef {
    pub account: &'static str,
    pub title: &'static str,
    /// Пары (turnover_code, layer), которые попадают в основной блок отчёта.
    pub main_entries: &'static [GlAccountViewEntry],
}

impl GlAccountViewDef {
    /// Возвращает `true`, если строка с данным (turnover_code, layer)
    /// должна входить в основной суммируемый блок.
    pub fn is_main_row(&self, turnover_code: &str, layer: &str) -> bool {
        self.main_entries
            .iter()
            .any(|e| e.turnover_code == turnover_code && (e.layer.is_empty() || e.layer == layer))
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Конфигурация для счёта 7609 — «Расчёты с маркетплейсом»
// ─────────────────────────────────────────────────────────────────────────────
//
// Основной блок включает фактические и операционные обороты, непосредственно
// формирующие баланс расчётного счёта с МП.
//
// Информационный блок (всё остальное) содержит операционные
// разложения (spp_discount, wb_extra_discount, storno-варианты oper-слоя),
// которые нужны для аналитики, но не влияют на сальдо 7609 в плане надо
// считать отдельно для сверки.

pub static ACCOUNT_7609_VIEW: GlAccountViewDef = GlAccountViewDef {
    account: "7609",
    title: "Расчёты с маркетплейсом (7609)",
    main_entries: &[
        // ── Выручка по прайслисту (oper) ───────────────────────────────────
        GlAccountViewEntry {
            turnover_code: "customer_revenue_pl",
            layer: "oper",
        },
        GlAccountViewEntry {
            turnover_code: "customer_revenue_pl_storno",
            layer: "oper",
        },
        // ── Скидки СПП (oper) ─────────────────────────────────────────────
        GlAccountViewEntry {
            turnover_code: "spp_discount",
            layer: "oper",
        },
        GlAccountViewEntry {
            turnover_code: "spp_discount_storno",
            layer: "oper",
        },
        // ── Доп. скидка WB (oper) ─────────────────────────────────────────
        GlAccountViewEntry {
            turnover_code: "wb_extra_discount",
            layer: "oper",
        },
        GlAccountViewEntry {
            turnover_code: "wb_extra_discount_storno",
            layer: "oper",
        },
        // ── Соинвестирование (oper) ────────────────────────────────────────
        GlAccountViewEntry {
            turnover_code: "wb_coinvestment",
            layer: "oper",
        },
        GlAccountViewEntry {
            turnover_code: "wb_coinvestment_storno",
            layer: "oper",
        },
        // ── Комиссии (oper) ────────────────────────────────────────────────
        GlAccountViewEntry {
            turnover_code: "mp_commission",
            layer: "oper",
        },
        GlAccountViewEntry {
            turnover_code: "mp_commission_storno",
            layer: "oper",
        },
        // ── Эквайринг (oper) ───────────────────────────────────────────────
        // ── Реклама (oper) ─────────────────────────────────────────────────
        GlAccountViewEntry {
            turnover_code: "advertising_allocated",
            layer: "oper",
        },
        // ── Добровольная компенсация (fact) ───────────────────────────────
        GlAccountViewEntry {
            turnover_code: "voluntary_return_compensation",
            layer: "fact",
        },
        GlAccountViewEntry {
            turnover_code: "mp_acquiring",
            layer: "fact",
        },
        // ── Логистика и хранение (fact) ────────────────────────────────────
        GlAccountViewEntry {
            turnover_code: "mp_logistics",
            layer: "fact",
        },
        GlAccountViewEntry {
            turnover_code: "mp_rebill_logistic_cost",
            layer: "fact",
        },
        GlAccountViewEntry {
            turnover_code: "mp_rebill_logistic_cost_nm",
            layer: "fact",
        },
        GlAccountViewEntry {
            turnover_code: "mp_rebill_logistic_cost_legacy",
            layer: "fact",
        },
        GlAccountViewEntry {
            turnover_code: "mp_ppvz_reward",
            layer: "fact",
        },
        GlAccountViewEntry {
            turnover_code: "mp_ppvz_reward_nm",
            layer: "fact",
        },
        GlAccountViewEntry {
            turnover_code: "mp_storage",
            layer: "fact",
        },
        GlAccountViewEntry {
            turnover_code: "acceptance",
            layer: "fact",
        },
        // ── Штрафы (fact) ──────────────────────────────────────────────────
        GlAccountViewEntry {
            turnover_code: "mp_penalty",
            layer: "fact",
        },
        GlAccountViewEntry {
            turnover_code: "mp_penalty_storno",
            layer: "fact",
        },
    ],
};

// ─────────────────────────────────────────────────────────────────────────────
// Реестр всех конфигураций
// ─────────────────────────────────────────────────────────────────────────────

pub static ACCOUNT_VIEW_REGISTRY: &[&GlAccountViewDef] = &[&ACCOUNT_7609_VIEW];

/// Найти конфигурацию ведомости по номеру счёта.
pub fn find_view(account: &str) -> Option<&'static GlAccountViewDef> {
    ACCOUNT_VIEW_REGISTRY
        .iter()
        .find(|def| def.account == account)
        .copied()
}

#[cfg(test)]
mod tests {
    use super::ACCOUNT_7609_VIEW;

    #[test]
    fn account_7609_view_places_acquiring_fact_in_main_block() {
        assert!(ACCOUNT_7609_VIEW.is_main_row("mp_acquiring", "fact"));
        assert!(!ACCOUNT_7609_VIEW.is_main_row("mp_acquiring", "oper"));
        assert!(!ACCOUNT_7609_VIEW.is_main_row("mp_acquiring_storno", "oper"));
    }
}
