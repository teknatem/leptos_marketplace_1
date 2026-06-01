//! Навигация по регистраторам (документам-источникам GL-проводок).
//!
//! group_key строки drilldown по регистратору = "{registrator_type}~~{ref}".
//! Здесь — разбор ключа и сопоставление типа регистратора с ключом вкладки
//! детального документа. Используется страницами drilldown (GL и DataView).

/// Разбирает "{type}~~{ref}" на (type, ref). Если разделителя нет — ("", key).
pub fn split_group_key(group_key: &str) -> (&str, &str) {
    match group_key.split_once("~~") {
        Some((t, r)) => (t, r),
        None => ("", group_key),
    }
}

/// Это ключ регистратора ("{type}~~{ref}")?
pub fn is_registrator_key(group_key: &str) -> bool {
    group_key.contains("~~")
}

/// Человекочитаемое имя типа регистратора (для подписи вкладки/фолбэка).
pub fn reg_type_name(reg_type: &str) -> &'static str {
    match reg_type {
        "a012_wb_sales" => "WB Продажа",
        "a013_ym_order" => "YM Заказ",
        "a014_ozon_transactions" => "OZON Транзакция",
        "a015_wb_orders" => "WB Заказ",
        "a016_ym_returns" => "YM Возврат",
        "a026_wb_advert_daily" => "WB Реклама",
        "a021_production_output" => "Производство",
        "a022_kit_variant" => "Комплект",
        "a023_purchase_of_goods" => "Закупка",
        "a028_missing_cost_registry" => "Реестр себестоимости",
        "p903_wb_finance_report" => "WB Финотчёт",
        "p907_ym_payment_report" => "YM Платёж",
        _ => "Документ",
    }
}

/// Ключ вкладки детального документа по (типу, ссылке). None — навигация для
/// этого типа не поддержана.
pub fn reg_tab_key(reg_type: &str, reg_ref: &str) -> Option<String> {
    let id = reg_ref;
    match reg_type {
        "a012_wb_sales" => Some(format!("a012_wb_sales_details_{id}")),
        "a013_ym_order" => Some(format!("a013_ym_order_details_{id}")),
        "a014_ozon_transactions" => Some(format!("a014_ozon_transactions_details_{id}")),
        "a015_wb_orders" => Some(format!("a015_wb_orders_details_{id}")),
        "a016_ym_returns" => Some(format!("a016_ym_returns_details_{id}")),
        "a026_wb_advert_daily" => Some(format!("a026_wb_advert_daily_details_{id}")),
        "a021_production_output" => Some(format!("a021_production_output_details_{id}")),
        "a022_kit_variant" => Some(format!("a022_kit_variant_details_{id}")),
        "a023_purchase_of_goods" => Some(format!("a023_purchase_of_goods_details_{id}")),
        "a028_missing_cost_registry" => Some(format!("a028_missing_cost_registry_details_{id}")),
        "p907_ym_payment_report" if !id.trim().is_empty() => {
            Some(format!("p907_ym_payment_report_details_{id}"))
        }
        "p903_wb_finance_report" if !id.trim().is_empty() => Some(format!(
            "p903_wb_finance_report_details_id_{}",
            urlencoding::encode(id)
        )),
        _ => None,
    }
}

/// Короткая подпись вкладки документа (фолбэк, когда нет реального имени).
pub fn reg_tab_label(reg_type: &str, reg_ref: &str) -> String {
    let short = if reg_ref.len() >= 8 {
        &reg_ref[..8]
    } else {
        reg_ref
    };
    format!("{} · {}", reg_type_name(reg_type), short)
}
