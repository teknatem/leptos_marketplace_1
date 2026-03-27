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
