/// Utilities for date and time formatting
///
/// Provides consistent date/time formatting across the application

/// Format ISO datetime string to DD.MM.YYYY HH:MM:SS format
/// Example: "2024-03-15T14:02:26.123Z" -> "15.03.2024 14:02:26"
pub fn format_datetime(datetime_str: &str) -> String {
    if let Some((date_part, time_part)) = datetime_str.split_once('T') {
        if let Some((year, rest)) = date_part.split_once('-') {
            if let Some((month, day)) = rest.split_once('-') {
                let time = time_part.split('.').next().unwrap_or(time_part);
                return format!("{}.{}.{} {}", day, month, year, time);
            }
        }
    }
    datetime_str.to_string()
}

/// Format ISO date string to DD.MM.YYYY format
/// Example: "2024-03-15" or "2024-03-15T14:02:26Z" -> "15.03.2024"
pub fn format_date(date_str: &str) -> String {
    let date_part = date_str.split('T').next().unwrap_or(date_str);
    if let Some((year, rest)) = date_part.split_once('-') {
        if let Some((month, day)) = rest.split_once('-') {
            return format!("{}.{}.{}", day, month, year);
        }
    }
    date_str.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_datetime() {
        assert_eq!(
            format_datetime("2024-03-15T14:02:26.123Z"),
            "15.03.2024 14:02:26"
        );
        assert_eq!(
            format_datetime("2024-12-31T23:59:59Z"),
            "31.12.2024 23:59:59"
        );
    }

    #[test]
    fn test_format_date() {
        assert_eq!(format_date("2024-03-15"), "15.03.2024");
        assert_eq!(format_date("2024-03-15T14:02:26.123Z"), "15.03.2024");
    }

    #[test]
    fn test_invalid_format() {
        assert_eq!(format_datetime("invalid"), "invalid");
        assert_eq!(format_date("invalid"), "invalid");
    }
}
