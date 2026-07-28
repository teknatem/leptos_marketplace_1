/// Utilities for date, time and size formatting.
///
/// Date/time display rules:
/// - `DateTime<Utc>` from API is an instant and must be displayed through
///   `format_utc_local(&dt, fmt)`.
/// - Date-only values (`NaiveDate`, `YYYY-MM-DD`) have no timezone and should be
///   displayed as dates only.
/// - `format_datetime(&str)` is a legacy string reshaper: it does not perform
///   timezone conversion and must not be used for typed `DateTime<Utc>` values.
use chrono::{DateTime, FixedOffset, NaiveDateTime, Utc};

/// Смещение часового пояса для отображения времени в UI (МСК = UTC+3).
/// Дублирует значение `[ui].timezone_offset_hours` из `config.toml`.
/// При смене часового пояса меняется только это значение.
pub const TZ_OFFSET_HOURS: i32 = 3;

/// Форматирует UTC-время с учётом локального смещения (`TZ_OFFSET_HOURS`).
/// Пример: `format_utc_local(&dt, "%H:%M:%S")` → "17:05:32"
pub fn format_utc_local(dt: &DateTime<Utc>, fmt: &str) -> String {
    let offset = FixedOffset::east_opt(TZ_OFFSET_HOURS * 3600)
        .unwrap_or_else(|| FixedOffset::east_opt(0).unwrap());
    dt.with_timezone(&offset).format(fmt).to_string()
}

/// Parses an RFC3339 datetime string and displays it using `format_utc_local`.
/// Falls back to legacy string reshaping when the input is not RFC3339.
pub fn format_datetime_utc_local(datetime_str: &str, fmt: &str) -> String {
    DateTime::parse_from_rfc3339(datetime_str)
        .map(|dt| format_utc_local(&dt.with_timezone(&Utc), fmt))
        .unwrap_or_else(|_| format_datetime(datetime_str))
}

/// Форматирует наивную UTC-строку вида `"YYYY-MM-DD HH:MM:SS"` (без часового пояса,
/// как хранится `plugin_run.started_at`), применяя локальное смещение (+3).
/// Пример: `"2025-07-28 12:00:00"` → `"2025-07-28 15:00"`.
/// Возвращает исходную строку, если формат не распознан.
pub fn format_space_naive_utc_local(datetime_str: &str, fmt: &str) -> String {
    NaiveDateTime::parse_from_str(datetime_str.trim(), "%Y-%m-%d %H:%M:%S")
        .map(|naive| format_utc_local(&naive.and_utc(), fmt))
        .unwrap_or_else(|_| datetime_str.to_string())
}

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

    #[test]
    fn test_format_space_naive_utc_local() {
        // 12:00 UTC → 15:00 МСК (+3).
        assert_eq!(
            format_space_naive_utc_local("2025-07-28 12:00:00", "%Y-%m-%d %H:%M"),
            "2025-07-28 15:00"
        );
        // Переход через полночь: 23:30 UTC → 02:30 следующего дня МСК.
        assert_eq!(
            format_space_naive_utc_local("2025-07-28 23:30:00", "%Y-%m-%d %H:%M"),
            "2025-07-29 02:30"
        );
        // Нераспознанный формат возвращается как есть.
        assert_eq!(format_space_naive_utc_local("invalid", "%H:%M"), "invalid");
    }
}

/// Человекочитаемая длительность из миллисекунд.
pub fn format_duration_ms(ms: i64) -> String {
    if ms < 1000 {
        format!("{}мс", ms)
    } else if ms < 60_000 {
        format!("{:.1}с", ms as f64 / 1000.0)
    } else {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1000;
        format!("{}м {}с", mins, secs)
    }
}

/// Компактный размер в байтах (B / KiB / MiB / GiB).
pub fn format_bytes_compact(n: u64) -> String {
    const KB: f64 = 1024.0;
    if n < 1024 {
        format!("{} B", n)
    } else if n < 1024 * 1024 {
        format!("{:.1} KiB", n as f64 / KB)
    } else if n < 1024_u64 * 1024 * 1024 {
        format!("{:.1} MiB", n as f64 / KB / KB)
    } else {
        format!("{:.2} GiB", n as f64 / KB / KB / KB)
    }
}

/// Строка «↑{up} ↓{down}» из байт (пусто если оба 0).
pub fn format_http_traffic(bytes_sent: i64, bytes_received: i64) -> Option<String> {
    let up = bytes_sent.max(0) as u64;
    let down = bytes_received.max(0) as u64;
    if up == 0 && down == 0 {
        None
    } else {
        Some(format!(
            "↑{} ↓{}",
            format_bytes_compact(up),
            format_bytes_compact(down)
        ))
    }
}
