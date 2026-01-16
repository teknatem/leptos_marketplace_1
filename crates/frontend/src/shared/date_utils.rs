/// Utilities for date and time formatting
///
/// Provides consistent date/time formatting across the application

/// Format ISO datetime string to DD.MM.YYYY HH:MM:SS format
/// Example: "2024-03-15T14:02:26.123Z" -> "15.03.2024 14:02:26"
/// Removes milliseconds, timezone indicators (Z, +00:00, etc.)
pub fn format_datetime(datetime_str: &str) -> String {
    if let Some((date_part, time_part)) = datetime_str.split_once('T') {
        if let Some((year, rest)) = date_part.split_once('-') {
            if let Some((month, day)) = rest.split_once('-') {
                // Remove milliseconds and timezone: split by '.' first
                let time_clean = time_part.split('.').next().unwrap_or(time_part);
                // Remove timezone indicators like 'Z', '+00:00', '-05:00'
                let time_final = time_clean
                    .trim_end_matches('Z')
                    .split('+')
                    .next()
                    .unwrap_or(time_clean)
                    .split('-')
                    .next()
                    .unwrap_or(time_clean);
                return format!("{}.{}.{} {}", day, month, year, time_final);
            }
        }
    }
    datetime_str.to_string()
}

/// Format datetime string with space separator to DD.MM.YYYY HH:MM:SS format
/// Example: "2025-10-11 00:00:00" -> "11.10.2025 00:00:00"
/// Handles format where date and time are separated by space instead of 'T'
pub fn format_datetime_space(datetime_str: &str) -> String {
    if let Some((date_part, time_part)) = datetime_str.split_once(' ') {
        if let Some((year, rest)) = date_part.split_once('-') {
            if let Some((month, day)) = rest.split_once('-') {
                // Take only HH:MM part (first 5 characters) if time is longer
                let time_short = if time_part.len() >= 5 {
                    &time_part[..5]
                } else {
                    time_part
                };
                return format!("{}.{}.{} {}", day, month, year, time_short);
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
    fn test_format_datetime_with_z() {
        // Test that Z is removed from the end
        assert_eq!(
            format_datetime("2025-12-23T16:30:15Z"),
            "23.12.2025 16:30:15"
        );
        // Test without milliseconds but with Z
        assert_eq!(
            format_datetime("2025-01-01T00:00:00Z"),
            "01.01.2025 00:00:00"
        );
    }

    #[test]
    fn test_format_datetime_with_timezone() {
        // Test with +00:00 timezone
        assert_eq!(
            format_datetime("2024-03-15T14:02:26+00:00"),
            "15.03.2024 14:02:26"
        );
        // Test with milliseconds and timezone
        assert_eq!(
            format_datetime("2024-03-15T14:02:26.123+05:00"),
            "15.03.2024 14:02:26"
        );
    }

    #[test]
    fn test_format_datetime_space() {
        assert_eq!(
            format_datetime_space("2025-10-11 00:00:00"),
            "11.10.2025 00:00"
        );
        assert_eq!(
            format_datetime_space("2024-12-31 23:59:59"),
            "31.12.2024 23:59"
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
        assert_eq!(format_datetime_space("invalid"), "invalid");
        assert_eq!(format_date("invalid"), "invalid");
    }
}
