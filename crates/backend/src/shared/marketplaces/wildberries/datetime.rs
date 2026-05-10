use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Utc};

pub const WB_TIMEZONE_OFFSET_HOURS: i32 = 3;

pub fn wb_timezone() -> FixedOffset {
    FixedOffset::east_opt(WB_TIMEZONE_OFFSET_HOURS * 3600)
        .unwrap_or_else(|| FixedOffset::east_opt(0).unwrap())
}

/// Parse WB datetime values.
///
/// WB APIs often return datetimes without an explicit offset. In that case the
/// value is treated as WB business time (MSK, UTC+3). RFC3339 values with an
/// explicit offset keep their own offset.
pub fn parse_wb_datetime(value: &str) -> Option<DateTime<Utc>> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
        return Some(dt.with_timezone(&Utc));
    }

    parse_wb_naive_datetime(value).and_then(|naive| {
        wb_timezone()
            .from_local_datetime(&naive)
            .single()
            .map(|dt| dt.with_timezone(&Utc))
    })
}

pub fn parse_wb_naive_datetime(value: &str) -> Option<NaiveDateTime> {
    [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
    ]
    .iter()
    .find_map(|fmt| NaiveDateTime::parse_from_str(value, fmt).ok())
}

pub fn wb_day_start_utc(date: NaiveDate) -> Option<DateTime<Utc>> {
    date.and_hms_opt(0, 0, 0).and_then(|naive| {
        wb_timezone()
            .from_local_datetime(&naive)
            .single()
            .map(|dt| dt.with_timezone(&Utc))
    })
}

pub fn wb_day_end_utc(date: NaiveDate) -> Option<DateTime<Utc>> {
    date.and_hms_milli_opt(23, 59, 59, 999).and_then(|naive| {
        wb_timezone()
            .from_local_datetime(&naive)
            .single()
            .map(|dt| dt.with_timezone(&Utc))
    })
}

pub fn wb_business_date(dt: &DateTime<Utc>) -> NaiveDate {
    dt.with_timezone(&wb_timezone()).date_naive()
}

pub fn format_wb_local_datetime_seconds(dt: &DateTime<Utc>) -> String {
    dt.with_timezone(&wb_timezone())
        .format("%Y-%m-%dT%H:%M:%S")
        .to_string()
}

pub fn format_wb_cursor_datetime(dt: &DateTime<Utc>) -> String {
    dt.with_timezone(&wb_timezone())
        .format("%Y-%m-%dT%H:%M:%S%.3f")
        .to_string()
}
