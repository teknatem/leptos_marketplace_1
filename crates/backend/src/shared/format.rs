/// Форматирует число с разделителями тысяч (точками)
///
/// # Примеры
/// ```
/// use backend::shared::format::format_number;
/// assert_eq!(format_number(1234567), "1.234.567");
/// assert_eq!(format_number(42), "42");
/// assert_eq!(format_number(0), "0");
/// ```
pub fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push('.');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(42), "42");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1.000");
        assert_eq!(format_number(1234), "1.234");
        assert_eq!(format_number(1234567), "1.234.567");
        assert_eq!(format_number(1234567890), "1.234.567.890");
    }
}
