//! Утилиты форматирования чисел для таблиц

/// Форматирует число с разделителем тысяч (пробел) и указанным количеством знаков после запятой
///
/// # Примеры
///
/// ```
/// let formatted = format_number_with_decimals(1234.567, 2);
/// assert_eq!(formatted, "1 234.57");
/// ```
pub fn format_number_with_decimals(value: f64, decimals: u8) -> String {
    // Форматируем с нужным количеством знаков после запятой
    let formatted = match decimals {
        0 => format!("{:.0}", value),
        1 => format!("{:.1}", value),
        2 => format!("{:.2}", value),
        3 => format!("{:.3}", value),
        _ => format!("{:.2}", value), // По умолчанию 2 знака
    };

    // Разделяем целую и дробную части
    let parts: Vec<&str> = formatted.split('.').collect();
    let integer_part = parts[0];
    let decimal_part = parts.get(1);

    // Вставляем пробелы каждые 3 цифры с конца целой части
    let mut result = String::new();
    let chars: Vec<char> = integer_part.chars().rev().collect();

    for (i, c) in chars.iter().enumerate() {
        if i > 0 && i % 3 == 0 && *c != '-' {
            result.push(' ');
        }
        result.push(*c);
    }

    // Переворачиваем обратно
    let formatted_integer = result.chars().rev().collect::<String>();

    // Добавляем дробную часть если есть
    match decimal_part {
        Some(d) => format!("{}.{}", formatted_integer, d),
        None => formatted_integer,
    }
}

/// Форматирует денежное значение с 2 знаками после запятой и разделителем тысяч
///
/// # Примеры
///
/// ```
/// let formatted = format_money(1234567.89);
/// assert_eq!(formatted, "1 234 567.89");
/// ```
pub fn format_money(value: f64) -> String {
    format_number_with_decimals(value, 2)
}

/// Форматирует целое число с разделителем тысяч
///
/// # Примеры
///
/// ```
/// let formatted = format_number_int(1234567.0);
/// assert_eq!(formatted, "1 234 567");
/// ```
pub fn format_number_int(value: f64) -> String {
    format_number_with_decimals(value, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_money() {
        assert_eq!(format_money(1234.56), "1 234.56");
        assert_eq!(format_money(1234567.89), "1 234 567.89");
        assert_eq!(format_money(0.0), "0.00");
        assert_eq!(format_money(-1234.56), "-1 234.56");
    }

    #[test]
    fn test_format_number_with_decimals() {
        assert_eq!(format_number_with_decimals(1234.567, 0), "1 235");
        assert_eq!(format_number_with_decimals(1234.567, 1), "1 234.6");
        assert_eq!(format_number_with_decimals(1234.567, 2), "1 234.57");
        assert_eq!(format_number_with_decimals(1234.567, 3), "1 234.567");
    }

    #[test]
    fn test_format_number_int() {
        assert_eq!(format_number_int(1234567.0), "1 234 567");
        assert_eq!(format_number_int(0.0), "0");
        assert_eq!(format_number_int(-1234.0), "-1 234");
    }
}
