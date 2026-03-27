pub fn format_int_with_triads(value: i64) -> String {
    let negative = value < 0;
    let digits = value.abs().to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);

    for (idx, ch) in digits.chars().rev().enumerate() {
        if idx > 0 && idx % 3 == 0 {
            grouped.push(' ');
        }
        grouped.push(ch);
    }

    let grouped: String = grouped.chars().rev().collect();
    if negative {
        format!("-{grouped}")
    } else {
        grouped
    }
}

pub fn round_to_thousands(value: f64) -> i64 {
    ((value / 1_000.0).round() as i64) * 1_000
}

pub fn format_number_with_triads(value: f64, decimals: usize) -> String {
    let formatted = format!("{value:.prec$}", prec = decimals);
    let mut parts = formatted.splitn(2, '.');
    let whole = parts
        .next()
        .and_then(|s| s.parse::<i64>().ok())
        .map(format_int_with_triads)
        .unwrap_or_else(|| formatted.clone());
    match parts.next() {
        Some(frac) if decimals > 0 => format!("{whole}.{frac}"),
        _ => whole,
    }
}

pub fn format_money_rounded_thousands(value: f64, currency: &str) -> String {
    let amount = format_int_with_triads(round_to_thousands(value));
    if currency == "RUB" {
        format!("{amount} ₽")
    } else {
        format!("{amount} {currency}")
    }
}

pub fn format_money_with_unit_scale(value: f64, currency: &str, decimals: usize) -> String {
    let amount = format_number_with_triads(value, decimals);
    if currency == "RUB" {
        format!("{amount} ₽")
    } else {
        format!("{amount} {currency}")
    }
}

pub fn format_money_with_format_spec(value: f64, format_spec: &serde_json::Value) -> String {
    let currency = format_spec["currency"].as_str().unwrap_or("RUB");
    let scale = format_spec["scale"].as_str().unwrap_or("unit");
    match (currency, scale) {
        ("RUB", "thousand") => {
            let decimals = format_spec["decimals"].as_u64().unwrap_or(0) as usize;
            let scaled = value / 1_000.0;
            format!("{} т. руб", format_number_with_triads(scaled, decimals))
        }
        ("RUB", "million") => {
            let decimals = format_spec["decimals"].as_u64().unwrap_or(2) as usize;
            let scaled = value / 1_000_000.0;
            format!("{} м. руб", format_number_with_triads(scaled, decimals))
        }
        _ => {
            let decimals = format_spec["decimals"].as_u64().unwrap_or(0) as usize;
            format_money_with_unit_scale(value, currency, decimals)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::format_money_with_format_spec;

    #[test]
    fn money_without_scale_is_formatted_in_rubles_not_thousands() {
        let format = serde_json::json!({
            "kind": "Money",
            "currency": "RUB",
            "decimals": 2
        });

        assert_eq!(
            format_money_with_format_spec(12018.0, &format),
            "12 018.00 ₽"
        );
    }
}
