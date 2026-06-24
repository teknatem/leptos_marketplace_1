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
pub fn turnover_code_for_source(source: &str) -> Option<&'static str> {
    match source.trim() {
        "Платёж покупателя" => Some("customer_revenue"),
        "Возврат платежа покупателя" => Some("customer_revenue_storno"),
        "Баллы за скидку Маркета" | "Баллы за скидку Яндекс Плюс" => {
            Some("wb_coinvestment")
        }
        "Возврат баллов за скидку Маркета" | "Возврат баллов за скидку Яндекс Плюс" => {
            Some("wb_coinvestment_storno")
        }
        "Скидка за участие в совместных акциях" => {
            Some("spp_discount")
        }
        "Возврат скидки за участие в совместных акциях" => {
            Some("spp_discount_storno")
        }
        // Известные регулярные операции без отдельного счёта — относим на «Прочие
        // доходы/расходы» явно, чтобы не зашумлять лог fallback-предупреждением.
        "Оплата услуг Яндекс.Маркета" => Some("other_expense"),
        "Премия" => Some("other_income"),
        _ => None,
    }
}

/// Универсальный («прочий») оборот для операций YM без специального правила:
/// положительная сумма → «Прочие доходы», отрицательная → «Прочие расходы».
/// Знак суммы сохраняется как есть (конвенция знака — см. реестр оборотов).
/// Гарантирует, что любая проведённая ненулевая строка p907 формирует проводку.
pub fn fallback_turnover_code(amount: f64) -> &'static str {
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

const BUYER_PAYMENT_SOURCE: &str = "Платёж покупателя";
const BUYER_PAYMENT_RETURN_SOURCE: &str = "Возврат платежа покупателя";

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
    let Some(txn_date) = row.transaction_date.as_deref().and_then(msk_date) else {
        return Ok(Vec::new());
    };

    // Денежные расчёты покупателя разносятся по времени: оплата (дата транзакции)
    // и реализация (дата отгрузки) проходят через 76YA/62. См. build_buyer_payment_entries.
    match source.trim() {
        BUYER_PAYMENT_SOURCE => Ok(build_buyer_payment_entries(row, amount, txn_date, false)),
        BUYER_PAYMENT_RETURN_SOURCE => Ok(build_buyer_payment_entries(row, amount, txn_date, true)),
        _ => Ok(build_single_entry(row, source, amount, txn_date)),
    }
}

/// Разнесение строки «Платёж покупателя» / «Возврат платежа покупателя» на ноги
/// расчётов (76YA/62) и реализации (62/9001, 7609/76YA). `is_return` инвертирует
/// поток (суммы из источника уже отрицательные для возврата).
///
/// Продажа (есть дата доставки): prepayment@txn + customer_revenue@deliv +
/// prepayment_settle@deliv. Без даты доставки: только prepayment (выручка отложена).
/// Возврат: всё датой транзакции; prepayment_storno всегда, ноги выручки/зачёта —
/// только если у строки есть дата доставки (заказ был отгружен).
fn build_buyer_payment_entries(
    row: &crate::projections::p907_ym_payment_report::repository::Model,
    amount: f64,
    txn_date: &str,
    is_return: bool,
) -> Vec<GeneralLedgerModel> {
    let delivery_date = row.order_delivery_date.as_deref().and_then(msk_date);
    // qty — только на ноге выручки; знак как у суммы (возврат — отрицательный).
    let revenue_qty = row
        .count
        .map(|c| if is_return { -(c as f64) } else { c as f64 });

    let mut entries = Vec::new();
    // Денежная нога выручки YM закрывает аванс 62 (оверрайд дебета 7609→62);
    // ноги расчётов (prepayment*) уже имеют свои счета в реестре.
    let mut push = |turnover_code: &str,
                    entry_date: &str,
                    qty: Option<f64>,
                    mirror_p914: bool,
                    debit_override: Option<&str>| {
        if let Some(entry) = make_entry_with_override(
            row,
            turnover_code,
            entry_date,
            amount,
            qty,
            mirror_p914,
            debit_override,
        ) {
            entries.push(entry);
        }
    };

    if is_return {
        // Возврат — все ноги датой транзакции (отдельной даты реализации нет).
        push("prepayment_storno", txn_date, None, false, None);
        if delivery_date.is_some() {
            push(
                "customer_revenue_storno",
                txn_date,
                revenue_qty,
                true,
                Some("62"),
            );
            push("prepayment_settle_storno", txn_date, None, false, None);
        }
    } else {
        // Получение предоплаты — всегда по дате транзакции.
        push("prepayment", txn_date, None, false, None);
        // Реализация — по дате отгрузки, если она известна.
        if let Some(deliv) = delivery_date {
            push("customer_revenue", deliv, revenue_qty, true, Some("62"));
            push("prepayment_settle", deliv, None, false, None);
        }
    }

    entries
}

/// Проводка для прочих источников YM (соинвестирование, скидки, удержания и т.п.):
/// один оборот по дате платёжной операции, как раньше.
fn build_single_entry(
    row: &crate::projections::p907_ym_payment_report::repository::Model,
    source: &str,
    amount: f64,
    txn_date: &str,
) -> Vec<GeneralLedgerModel> {
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

    // Контур: операции payment_status «Справочно: …баланс» — кошелёк баллов/промо,
    // не живые деньги. Уводим их денежную ногу с 7609 на отдельный счёт 76YB, чтобы
    // сальдо 7609 = строго «доступно к перечислению поставщику». P&L-нога (91/9001)
    // не меняется; в конвенции YM денежный счёт всегда на дебете.
    let is_points_wallet = row
        .payment_status
        .as_deref()
        .map(|s| s.trim_start().starts_with("Справочно"))
        .unwrap_or(false);
    let debit_override = is_points_wallet.then_some("76YB");

    make_entry_with_override(
        row,
        turnover_code,
        txn_date,
        amount,
        qty,
        true,
        debit_override,
    )
    .into_iter()
    .collect()
}

/// Сборка одной GL-проводки слоя fina по обороту, с возможностью переопределить
/// дебетовый счёт. `mirror_p914` помечает строку для зеркала p914 (`resource_table`);
/// балансовые расчётные ноги (prepayment*) не зеркалятся. Оверрайд дебета нужен для
/// YM-специфики: денежная нога выручки закрывает аванс 62 (а не 7609, как у общего
/// `customer_revenue` для WB), и для кошелька баллов 76YB. Только дебет переопределяемый:
/// `7609`-конвенция YM держит расчётный счёт всегда на дебете. Возвращает `None`,
/// если оборот неизвестен или у него нет счетов.
fn make_entry_with_override(
    row: &crate::projections::p907_ym_payment_report::repository::Model,
    turnover_code: &str,
    entry_date: &str,
    amount: f64,
    qty: Option<f64>,
    mirror_p914: bool,
    debit_override: Option<&str>,
) -> Option<GeneralLedgerModel> {
    let class = get_turnover_class(turnover_code)?;
    // Слой fina формирует проводку по обороту с заданными счетами; глобальный
    // `generates_journal_entry` не используется (см. p903 builder). Так,
    // `customer_revenue_storno` (generates_journal_entry=false, но счета заданы)
    // корректно формирует красное сторно в слое fina.
    if class.debit_account.is_empty() || class.credit_account.is_empty() {
        return None;
    }

    // Выручка YM закрывает аванс 62 вместо 7609 (общий customer_revenue для WB —
    // Дт7609/Кт9001; YM проходит через 62). Применяем оверрайд только когда базовый
    // дебет = 7609, чтобы не задеть прочие обороты.
    let debit_account = match debit_override {
        Some(account) if class.debit_account == "7609" => account,
        _ => class.debit_account,
    };

    Some(GeneralLedgerModel {
        id: Uuid::new_v4().to_string(),
        entry_date: entry_date.to_string(),
        layer: TurnoverLayer::Fina.as_str().to_string(),
        // Весь платёжный отчёт YM — операции субъекта-маркетплейса «ym».
        entity: Some(GlEntity::Ym.as_str().to_string()),
        connection_mp_ref: Some(row.connection_mp_ref.clone()),
        registrator_type: REGISTRATOR_TYPE.to_string(),
        registrator_ref: row.id.clone(),
        order_id: row.order_id.map(|value| value.to_string()),
        debit_account: debit_account.to_string(),
        credit_account: class.credit_account.to_string(),
        amount,
        qty,
        turnover_code: turnover_code.to_string(),
        // Зеркало p914 хранит ту же сумму (gl.amount), поэтому сверка идёт по полю
        // `amount` со знаком +1. Балансовые расчётные ноги (prepayment*) не зеркалятся.
        resource_table: if mirror_p914 {
            RESOURCE_TABLE.to_string()
        } else {
            String::new()
        },
        resource_field: "amount".to_string(),
        resource_sign: 1,
        created_at: now_str(),
    })
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

    // p914 — зеркало финансовых оборотов (выручка/доходы/расходы). Балансовые
    // расчётные ноги предоплат (prepayment*) в нём не нужны: они не несут
    // номенклатурной/fina-аналитики и помечены пустым resource_table.
    let mirrored: Vec<GeneralLedgerModel> = gl_entries
        .iter()
        .filter(|entry| !entry.turnover_code.starts_with("prepayment"))
        .cloned()
        .collect();

    from_general_ledger_entries(&mirrored, &ctx)
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

    /// Найти проводку по коду оборота среди набора.
    fn by_code<'a>(entries: &'a [GeneralLedgerModel], code: &str) -> &'a GeneralLedgerModel {
        entries
            .iter()
            .find(|e| e.turnover_code == code)
            .unwrap_or_else(|| panic!("нет проводки с оборотом {code}"))
    }

    #[test]
    fn buyer_payment_without_delivery_date_only_prepayment() {
        // Нет даты доставки → выручка отложена; только денежная нога предоплаты.
        let row = base_row();
        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();

        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.turnover_code, "prepayment");
        assert_eq!(entry.layer, "fina");
        assert_eq!(entry.entity.as_deref(), Some("ym"));
        assert_eq!(entry.order_id.as_deref(), Some("12345"));
        assert_eq!(entry.debit_account, "76YA");
        assert_eq!(entry.credit_account, "62");
        assert_eq!(entry.amount, 1000.0);
        // Денежная нога — по дате транзакции; в p914 не зеркалится.
        assert_eq!(entry.entry_date, "2026-05-19");
        assert!(entry.qty.is_none());
        assert_eq!(entry.resource_table, "");
    }

    #[test]
    fn buyer_payment_with_delivery_splits_into_three_legs() {
        let mut row = base_row();
        row.order_delivery_date = Some("2026-05-25 09:00".to_string());
        row.count = Some(2);
        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();
        assert_eq!(entries.len(), 3);

        // Денежная нога — по дате транзакции, через 76YA/62, без qty и без p914.
        let prepay = by_code(&entries, "prepayment");
        assert_eq!(prepay.debit_account, "76YA");
        assert_eq!(prepay.credit_account, "62");
        assert_eq!(prepay.entry_date, "2026-05-19");
        assert_eq!(prepay.amount, 1000.0);
        assert!(prepay.qty.is_none());
        assert_eq!(prepay.resource_table, "");

        // Выручка — по дате доставки, закрывает аванс (Дт62/Кт9001), с qty и p914.
        let revenue = by_code(&entries, "customer_revenue");
        assert_eq!(revenue.debit_account, "62");
        assert_eq!(revenue.credit_account, "9001");
        assert_eq!(revenue.entry_date, "2026-05-25");
        assert_eq!(revenue.amount, 1000.0);
        assert_eq!(revenue.qty, Some(2.0));
        assert_eq!(revenue.resource_table, "p914_mp_finance_turnovers");

        // Зачёт предоплаты в долг МП — по дате доставки, Дт7609/Кт76YA.
        let settle = by_code(&entries, "prepayment_settle");
        assert_eq!(settle.debit_account, "7609");
        assert_eq!(settle.credit_account, "76YA");
        assert_eq!(settle.entry_date, "2026-05-25");
        assert_eq!(settle.amount, 1000.0);
        assert!(settle.qty.is_none());
        assert_eq!(settle.resource_table, "");
    }

    #[test]
    fn spravochno_status_routes_money_leg_to_points_wallet() {
        // «Баллы за скидку Маркета» со статусом «Справочно: …баланс» — кошелёк баллов.
        let mut row = base_row();
        row.transaction_source = Some("Баллы за скидку Маркета".to_string());
        row.transaction_sum = Some(300.0);
        row.payment_status = Some("Справочно: пополнен баланс".to_string());
        let entry = &build_general_ledger_entries(&row, "").unwrap()[0];
        assert_eq!(entry.turnover_code, "wb_coinvestment");
        // Денежная нога ушла с 7609 на 76YB; P&L-нога (91) не изменилась.
        assert_eq!(entry.debit_account, "76YB");
        assert_eq!(entry.credit_account, "91");
    }

    #[test]
    fn buyer_payment_return_without_delivery_only_prepayment_storno() {
        // Отмена предоплаты до отгрузки → закрываем только аванс, без P&L.
        let mut row = base_row();
        row.transaction_type = Some("Возврат".to_string());
        row.transaction_source = Some("Возврат платежа покупателя".to_string());
        row.transaction_sum = Some(-500.0);
        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();

        assert_eq!(entries.len(), 1);
        let entry = &entries[0];
        assert_eq!(entry.turnover_code, "prepayment_storno");
        assert_eq!(entry.debit_account, "76YA");
        assert_eq!(entry.credit_account, "62");
        assert_eq!(entry.amount, -500.0);
        assert_eq!(entry.entry_date, "2026-05-19");
    }

    #[test]
    fn buyer_payment_return_delivered_reverses_all_legs_by_transaction_date() {
        let mut row = base_row();
        row.transaction_type = Some("Возврат".to_string());
        row.transaction_source = Some("Возврат платежа покупателя".to_string());
        row.transaction_sum = Some(-500.0);
        row.order_delivery_date = Some("2026-05-25 09:00".to_string());
        row.count = Some(1);
        let entries = build_general_ledger_entries(&row, "posting-1").unwrap();
        assert_eq!(entries.len(), 3);

        // Все ноги возврата датируются датой транзакции (отдельной даты реализации нет).
        for e in &entries {
            assert_eq!(e.entry_date, "2026-05-19", "оборот {}", e.turnover_code);
        }

        let prepay = by_code(&entries, "prepayment_storno");
        assert_eq!(prepay.debit_account, "76YA");
        assert_eq!(prepay.credit_account, "62");
        assert_eq!(prepay.amount, -500.0);

        let revenue = by_code(&entries, "customer_revenue_storno");
        assert_eq!(revenue.debit_account, "62");
        assert_eq!(revenue.credit_account, "9001");
        assert_eq!(revenue.amount, -500.0);
        assert_eq!(revenue.qty, Some(-1.0));
        assert_eq!(revenue.resource_table, "p914_mp_finance_turnovers");

        let settle = by_code(&entries, "prepayment_settle_storno");
        assert_eq!(settle.debit_account, "7609");
        assert_eq!(settle.credit_account, "76YA");
        assert_eq!(settle.amount, -500.0);
    }

    #[test]
    fn market_and_plus_points_use_wb_coinvestment() {
        for source in ["Баллы за скидку Маркета", "Баллы за скидку Яндекс Плюс"]
        {
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
        // Неизвестное начисление без спец-правила: сумма > 0 → Прочие доходы.
        let entry = single_entry("Начисление", "Неизвестное начисление XYZ", 4031418.61);
        assert_eq!(entry.turnover_code, "other_income");
        assert_eq!(entry.debit_account, "7609");
        assert_eq!(entry.credit_account, "91");
        assert_eq!(entry.amount, 4031418.61);
        assert_eq!(entry.layer, "fina");
    }

    #[test]
    fn unknown_negative_source_falls_back_to_other_expense() {
        // Неизвестное удержание без спец-правила: сумма < 0 → Прочие расходы.
        let entry = single_entry("Списание", "Неизвестное удержание XYZ", -21871760.16);
        assert_eq!(entry.turnover_code, "other_expense");
        assert_eq!(entry.debit_account, "7609");
        assert_eq!(entry.credit_account, "9102");
        assert_eq!(entry.amount, -21871760.16);
        assert_eq!(entry.layer, "fina");
    }

    #[test]
    fn known_service_and_premium_sources_map_without_fallback_warning() {
        // Регулярные операции с явным маппингом (не через fallback → без WARN).
        let service = single_entry("Списание", "Оплата услуг Яндекс.Маркета", -21871760.16);
        assert_eq!(service.turnover_code, "other_expense");
        assert_eq!(service.amount, -21871760.16);

        let premium = single_entry("Начисление", "Премия", 4031418.61);
        assert_eq!(premium.turnover_code, "other_income");
        assert_eq!(premium.amount, 4031418.61);
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
    fn finance_turnover_mirrors_only_revenue_leg_not_prepayment() {
        let mut row = base_row();
        row.nomenclature_ref = Some("nom-1".to_string());
        row.order_delivery_date = Some("2026-05-25 09:00".to_string());
        let gl_entries = build_general_ledger_entries(&row, "posting-1").unwrap();
        // 3 GL-ноги (prepayment + customer_revenue + prepayment_settle), но в p914
        // зеркалится только выручка — расчётные ноги предоплаты исключены.
        assert_eq!(gl_entries.len(), 3);
        let turnovers = build_finance_turnover_entries(&row, &gl_entries);

        assert_eq!(turnovers.len(), 1);
        let mirror = &turnovers[0];
        let revenue_gl = by_code(&gl_entries, "customer_revenue");
        assert_eq!(
            mirror.general_ledger_ref.as_deref(),
            Some(revenue_gl.id.as_str())
        );
        assert_eq!(mirror.amount, 1000.0);
        assert_eq!(mirror.layer, "fina");
        assert_eq!(mirror.turnover_code, "customer_revenue");
        assert_eq!(mirror.transaction_date, revenue_gl.entry_date);
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
