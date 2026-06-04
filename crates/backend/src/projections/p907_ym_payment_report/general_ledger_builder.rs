use anyhow::Result;
use contracts::general_ledger::GlEntity;
use contracts::shared::analytics::TurnoverLayer;
use uuid::Uuid;

use crate::general_ledger::repository::Model as GeneralLedgerModel;
use crate::general_ledger::turnover_registry::get_turnover_class;
use crate::shared::analytics::normalization::opt_nonzero;

/// Сверка GL-проводки идёт против зеркальной проекции p914 (а не сырой строки
/// p907): p914 строится 1:1 из этих же проводок и совпадает по сумме и ключевым
/// измерениям, тогда как сырой p907 не зарегистрирован в `detail_links`.
const RESOURCE_TABLE: &str = "p914_mp_finance_turnovers";
const REGISTRATOR_TYPE: &str = "p907_ym_payment_report";

fn now_str() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Маппинг операции YM (`transaction_source`) на код оборота. Источник YM уже
/// несёт корректный знак суммы (начисления +, списания/возвраты −), поэтому
/// сумма проводки = `transaction_sum` как есть, а возвраты переиспользуют
/// зеркальные `_storno`-обороты с теми же счетами.
///
/// Совпадающие по смыслу с WB операции используют те же обороты:
/// - «Платёж покупателя» → выручка (как WB);
/// - «Баллы за скидку Маркета/Яндекс Плюс» → соинвестирование (компенсация
///   маркетплейсом скидки покупателю), один общий оборот `wb_coinvestment`;
/// - «Скидка за участие в совместных акциях» → скидка продавца `spp_discount`.
///
/// Неизвестные источники → `None`: проводка не формируется (см. вызов).
fn turnover_code_for_source(source: &str) -> Option<&'static str> {
    match source.trim() {
        "Платёж покупателя" => Some("customer_revenue"),
        "Возврат платежа покупателя" => Some("customer_revenue_storno"),
        "Баллы за скидку Маркета" | "Баллы за скидку Яндекс Плюс" => Some("wb_coinvestment"),
        "Возврат баллов за скидку Маркета" | "Возврат баллов за скидку Яндекс Плюс" => {
            Some("wb_coinvestment_storno")
        }
        "Скидка за участие в совместных акциях" => Some("spp_discount"),
        "Возврат скидки за участие в совместных акциях" => Some("spp_discount_storno"),
        _ => None,
    }
}

/// Универсальный («прочий») оборот для операций YM без специального правила:
/// положительная сумма → «Прочие доходы», отрицательная → «Прочие расходы».
/// Знак суммы сохраняется как есть (конвенция знака — см. реестр оборотов).
/// Гарантирует, что любая проведённая ненулевая строка p907 формирует проводку.
fn fallback_turnover_code(amount: f64) -> &'static str {
    if amount > 0.0 {
        "other_income"
    } else {
        "other_expense"
    }
}

/// Дата проводки по МСК: источник `transaction_date` хранится как
/// `"YYYY-MM-DD HH:MM"` без зоны и уже в МСК, поэтому берём только дату-часть
/// (первые 10 символов). Работает и для значений без времени.
fn msk_date(transaction_date: &str) -> Option<&str> {
    let trimmed = transaction_date.trim();
    if trimmed.len() < 10 {
        return None;
    }
    let date = &trimmed[0..10];
    if date.is_empty() {
        None
    } else {
        Some(date)
    }
}

pub fn build_general_ledger_entries(
    row: &crate::projections::p907_ym_payment_report::repository::Model,
    _posting_id: &str,
) -> Result<Vec<GeneralLedgerModel>> {
    let Some(source) = row.transaction_source.as_deref() else {
        return Ok(Vec::new());
    };
    let Some(amount) = opt_nonzero(row.transaction_sum) else {
        return Ok(Vec::new());
    };
    let Some(entry_date) = row
        .transaction_date
        .as_deref()
        .and_then(msk_date)
    else {
        return Ok(Vec::new());
    };

    // Известная операция → специальный оборот. Иначе сумму не теряем, а относим
    // на универсальный «Прочие доходы/расходы» по знаку — это даёт 100% покрытие
    // проведённых (ненулевых) строк p907 в GL. Лог оставляем, чтобы операцию
    // можно было затем формализовать отдельным правилом в turnover_code_for_source.
    let turnover_code = match turnover_code_for_source(source) {
        Some(code) => code,
        None => {
            let fallback = fallback_turnover_code(amount);
            tracing::warn!(
                "p907 GL: неизвестный transaction_source «{}» (row {}) — отнесён на «{}» (fallback)",
                source.trim(),
                row.id,
                fallback
            );
            fallback
        }
    };

    let Some(class) = get_turnover_class(turnover_code) else {
        return Ok(Vec::new());
    };
    // Слой fina формирует проводку по обороту с заданными счетами; глобальный
    // `generates_journal_entry` не используется (см. p903 builder). Так,
    // `customer_revenue_storno` (generates_journal_entry=false, но счета заданы)
    // корректно формирует красное сторно в слое fina.
    if class.debit_account.is_empty() || class.credit_account.is_empty() {
        return Ok(Vec::new());
    }

    // Количественный учёт: знак количества как у суммы (сторно/возврат —
    // отрицательное), чтобы нетто-кол-во = продажи − возвраты.
    let qty = row.count.map(|c| {
        let q = c as f64;
        if turnover_code.ends_with("_storno") {
            -q
        } else {
            q
        }
    });

    Ok(vec![GeneralLedgerModel {
        id: Uuid::new_v4().to_string(),
        entry_date: entry_date.to_string(),
        layer: TurnoverLayer::Fina.as_str().to_string(),
        // Весь платёжный отчёт YM — операции субъекта-маркетплейса «ym».
        entity: Some(GlEntity::Ym.as_str().to_string()),
        connection_mp_ref: Some(row.connection_mp_ref.clone()),
        registrator_type: REGISTRATOR_TYPE.to_string(),
        registrator_ref: row.id.clone(),
        order_id: row.order_id.map(|value| value.to_string()),
        debit_account: class.debit_account.to_string(),
        credit_account: class.credit_account.to_string(),
        amount,
        qty,
        turnover_code: turnover_code.to_string(),
        // Зеркало p914 хранит ту же сумму (gl.amount), поэтому сверка идёт по
        // полю `amount` со знаком +1 — строка p914 совпадает с проводкой 1:1.
        resource_table: RESOURCE_TABLE.to_string(),
        resource_field: "amount".to_string(),
        resource_sign: 1,
        created_at: now_str(),
    }])
}

/// Строит строки проекции p914 (слой `fina`) как зеркало GL-проводок строки
/// p907. `marketplace_product_ref`, `marketplace_order_ref` и `nomenclature_ref`
/// уже резолвятся и хранятся в самой строке p907 (на этапе проведения) — здесь
/// просто копируются. `customer_kind`/`fulfillment_type` выводятся из полей YM.
pub fn build_finance_turnover_entries(
    row: &crate::projections::p907_ym_payment_report::repository::Model,
    gl_entries: &[GeneralLedgerModel],
) -> Vec<crate::projections::p914_mp_finance_turnovers::repository::Model> {
    use crate::projections::p914_mp_finance_turnovers::builder::{
        from_general_ledger_entries, FinanceTurnoverContext,
    };

    // customer_kind из order_type: «Продажа физлицу» → FIZ, «Продажа бизнесу» → URL.
    let customer_kind = row.order_type.as_deref().map(str::trim).and_then(|value| {
        if value.eq_ignore_ascii_case("Продажа физлицу") {
            Some("FIZ".to_string())
        } else if value.eq_ignore_ascii_case("Продажа бизнесу") {
            Some("URL".to_string())
        } else {
            None
        }
    });
    // fulfillment_type из модели YM (FBS / FBY / DBS / …).
    let fulfillment_type = row
        .model
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string());

    // marketplace_product_ref/marketplace_order_ref уже резолвятся и хранятся
    // в самой строке p907 (на этапе проведения) — здесь просто копируются.
    let order_ref = row
        .marketplace_order_ref
        .clone()
        .filter(|value| !value.trim().is_empty());
    let order_registrator_type = order_ref.as_ref().map(|_| "a013_ym_order".to_string());

    let ctx = FinanceTurnoverContext {
        nomenclature_ref: row
            .nomenclature_ref
            .clone()
            .filter(|value| !value.trim().is_empty()),
        marketplace_product_ref: row
            .marketplace_product_ref
            .clone()
            .filter(|value| !value.trim().is_empty()),
        order_key: row
            .order_id
            .map(|value| value.to_string())
            .unwrap_or_default(),
        order_ref,
        order_registrator_type,
        customer_kind,
        fulfillment_type,
        quantity: row.count.map(|value| value as f64),
    };

    from_general_ledger_entries(gl_entries, &ctx)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_row() -> crate::projections::p907_ym_payment_report::repository::Model {
        crate::projections::p907_ym_payment_report::repository::Model {
            record_key: "ymid_1".to_string(),
            id: "p907-row-1".to_string(),
            connection_mp_ref: "conn-1".to_string(),
            organization_ref: "org-1".to_string(),
            business_id: None,
            partner_id: None,
            shop_name: None,
            inn: None,
            model: None,
            transaction_id: Some("tx-1".to_string()),
            transaction_date: Some("2026-05-19 11:23".to_string()),
            transaction_type: Some("Начисление".to_string()),
            transaction_source: Some("Платёж покупателя".to_string()),
            transaction_sum: Some(1000.0),
            payment_status: None,
            order_id: Some(12345),
            shop_order_id: None,
            order_creation_date: None,
            order_delivery_date: None,
            order_type: None,
            shop_sku: None,
            offer_or_service_name: None,
            count: None,
            act_id: None,
            act_date: None,
            bank_order_id: None,
            bank_order_date: None,
            bank_sum: None,
            claim_number: None,
            bonus_account_year_month: None,
            comments: None,
            marketplace_product_ref: None,
            marketplace_order_ref: None,
            nomenclature_ref: None,
            loaded_at_utc: "2026-05-19T00:00:00Z".to_string(),
            payload_version: 1,
        }
    }

    /// Хелпер: построить единственную проводку для заданной операции YM.
    fn single_entry(
        transaction_type: &str,
        transaction_source: &str,
        sum: f64,
    ) -> GeneralLedgerModel {
        let mut row = base_row();
        row.transaction_type = Some(transaction_type.to_string());
        row.transaction_source = Some(transaction_source.to_string());
        row.transaction_sum = Some(sum);
        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();
        assert_eq!(entries.len(), 1, "ожидалась ровно одна проводка");
        entries.into_iter().next().unwrap()
    }

    #[test]
    fn accrual_buyer_payment_generates_customer_revenue_fina_entry() {
        let row = base_row();
        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();

        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.layer, "fina");
        assert_eq!(entry.entity.as_deref(), Some("ym"));
        assert_eq!(entry.registrator_type, "p907_ym_payment_report");
        assert_eq!(entry.registrator_ref, row.id);
        assert_eq!(entry.order_id.as_deref(), Some("12345"));
        assert_eq!(entry.turnover_code, "customer_revenue");
        assert_eq!(entry.debit_account, "7609");
        assert_eq!(entry.credit_account, "9001");
        assert_eq!(entry.amount, 1000.0);
        // entry_date усечён до даты по МСК (время отброшено).
        assert_eq!(entry.entry_date, "2026-05-19");
        assert_eq!(entry.resource_table, "p914_mp_finance_turnovers");
        assert_eq!(entry.resource_field, "amount");
        assert_eq!(entry.resource_sign, 1);
    }

    #[test]
    fn buyer_payment_return_uses_customer_revenue_storno() {
        let entry = single_entry("Возврат", "Возврат платежа покупателя", -500.0);
        assert_eq!(entry.turnover_code, "customer_revenue_storno");
        assert_eq!(entry.debit_account, "7609");
        assert_eq!(entry.credit_account, "9001");
        assert_eq!(entry.amount, -500.0);
    }

    #[test]
    fn market_and_plus_points_use_wb_coinvestment() {
        for source in ["Баллы за скидку Маркета", "Баллы за скидку Яндекс Плюс"] {
            let entry = single_entry("Начисление", source, 300.0);
            assert_eq!(entry.turnover_code, "wb_coinvestment", "source: {source}");
            assert_eq!(entry.debit_account, "7609");
            assert_eq!(entry.credit_account, "91");
            assert_eq!(entry.amount, 300.0);
        }
    }

    #[test]
    fn points_return_uses_wb_coinvestment_storno() {
        for source in [
            "Возврат баллов за скидку Маркета",
            "Возврат баллов за скидку Яндекс Плюс",
        ] {
            let entry = single_entry("Возврат", source, -300.0);
            assert_eq!(
                entry.turnover_code, "wb_coinvestment_storno",
                "source: {source}"
            );
            assert_eq!(entry.debit_account, "7609");
            assert_eq!(entry.credit_account, "91");
            assert_eq!(entry.amount, -300.0);
        }
    }

    #[test]
    fn coop_promo_discount_uses_spp_discount() {
        let entry = single_entry("Списание", "Скидка за участие в совместных акциях", -120.0);
        assert_eq!(entry.turnover_code, "spp_discount");
        assert_eq!(entry.debit_account, "7609");
        assert_eq!(entry.credit_account, "9001");
        assert_eq!(entry.amount, -120.0);
    }

    #[test]
    fn coop_promo_discount_return_uses_spp_discount_storno() {
        let entry = single_entry(
            "Возврат списания",
            "Возврат скидки за участие в совместных акциях",
            120.0,
        );
        assert_eq!(entry.turnover_code, "spp_discount_storno");
        assert_eq!(entry.debit_account, "7609");
        assert_eq!(entry.credit_account, "9001");
        assert_eq!(entry.amount, 120.0);
    }

    #[test]
    fn unknown_positive_source_falls_back_to_other_income() {
        // «Премия» и подобные начисления без спец-правила: сумма > 0 → Прочие доходы.
        let entry = single_entry("Начисление", "Премия", 4031418.61);
        assert_eq!(entry.turnover_code, "other_income");
        assert_eq!(entry.debit_account, "7609");
        assert_eq!(entry.credit_account, "91");
        assert_eq!(entry.amount, 4031418.61);
        assert_eq!(entry.layer, "fina");
    }

    #[test]
    fn unknown_negative_source_falls_back_to_other_expense() {
        // «Оплата услуг Яндекс.Маркета»: сумма < 0 → Прочие расходы, знак сохраняется.
        let entry = single_entry("Списание", "Оплата услуг Яндекс.Маркета", -21871760.16);
        assert_eq!(entry.turnover_code, "other_expense");
        assert_eq!(entry.debit_account, "7609");
        assert_eq!(entry.credit_account, "9102");
        assert_eq!(entry.amount, -21871760.16);
        assert_eq!(entry.layer, "fina");
    }

    #[test]
    fn zero_sum_unknown_source_generates_no_entry() {
        // Нулевая сумма не порождает проводку даже при неизвестном источнике.
        let mut row = base_row();
        row.transaction_source = Some("Неизвестная операция".to_string());
        row.transaction_sum = Some(0.0);

        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();

        assert!(entries.is_empty());
    }

    #[test]
    fn finance_turnover_mirrors_fina_general_ledger_entry() {
        let mut row = base_row();
        row.nomenclature_ref = Some("nom-1".to_string());
        let gl_entries = build_general_ledger_entries(&row, "posting-1").unwrap();
        let turnovers = build_finance_turnover_entries(&row, &gl_entries);

        assert_eq!(turnovers.len(), 1);
        let mirror = &turnovers[0];
        assert_eq!(mirror.general_ledger_ref.as_deref(), Some(gl_entries[0].id.as_str()));
        assert_eq!(mirror.amount, 1000.0);
        assert_eq!(mirror.layer, "fina");
        assert_eq!(mirror.turnover_code, "customer_revenue");
        assert_eq!(mirror.transaction_date, gl_entries[0].entry_date);
        assert_eq!(mirror.order_key, "12345");
        assert_eq!(mirror.event_kind, "sold");
        // nomenclature_ref зеркалится из строки p907 (заполняется при проведении из a007).
        assert_eq!(mirror.nomenclature_ref.as_deref(), Some("nom-1"));
    }

    #[test]
    fn fallback_other_expense_mirrors_into_p914() {
        // Прочий расход тоже зеркалится в p914 (слой fina), сумма совпадает 1:1.
        let mut row = base_row();
        row.transaction_source = Some("Оплата услуг Яндекс.Маркета".to_string());
        row.transaction_sum = Some(-1000.0);
        let gl_entries = build_general_ledger_entries(&row, "posting-1").unwrap();
        let turnovers = build_finance_turnover_entries(&row, &gl_entries);

        assert_eq!(gl_entries.len(), 1);
        assert_eq!(turnovers.len(), 1);
        assert_eq!(turnovers[0].turnover_code, "other_expense");
        assert_eq!(turnovers[0].amount, -1000.0);
        assert_eq!(turnovers[0].layer, "fina");
    }
}
