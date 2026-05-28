/// Минимальная значимая сумма (1 коп.). Меньшие значения не создают строк атрибуции.
pub const MIN_MONEY_AMOUNT: f64 = 0.01;

/// Сумма достаточна для проводки/строки атрибуции (≥ 1 коп.).
pub fn is_significant_amount(value: f64) -> bool {
    value.abs() >= MIN_MONEY_AMOUNT
}

#[cfg(test)]
mod tests {
    use super::{is_significant_amount, MIN_MONEY_AMOUNT};

    #[test]
    fn significant_amount_threshold_is_one_kopeck() {
        assert_eq!(MIN_MONEY_AMOUNT, 0.01);
        assert!(!is_significant_amount(0.0));
        assert!(!is_significant_amount(0.009));
        assert!(!is_significant_amount(-0.004));
        assert!(is_significant_amount(0.01));
        assert!(is_significant_amount(-0.01));
        assert!(is_significant_amount(100.0));
    }
}

/// Нормализует расходную сумму: всегда отрицательное значение, None если ≈0.
pub fn normalize_expense(value: Option<f64>) -> Option<f64> {
    value.and_then(|v| {
        if v.abs() <= f64::EPSILON {
            None
        } else if v > 0.0 {
            Some(-v)
        } else {
            Some(v)
        }
    })
}

/// Возвращает абсолютное значение, None если ≈0.
pub fn normalize_positive(value: Option<f64>) -> Option<f64> {
    value.and_then(|v| {
        if v.abs() <= f64::EPSILON {
            None
        } else {
            Some(v.abs())
        }
    })
}

/// Возвращает значение со знаком минус, None если ≈0.
pub fn normalize_negative(value: Option<f64>) -> Option<f64> {
    value.and_then(|v| {
        if v.abs() <= f64::EPSILON {
            None
        } else {
            Some(-v.abs())
        }
    })
}

/// Возвращает значение без изменения знака, None если ≈0.
pub fn opt_nonzero(value: Option<f64>) -> Option<f64> {
    value.filter(|v| v.abs() > f64::EPSILON)
}
