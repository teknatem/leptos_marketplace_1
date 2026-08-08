//! Общий разбор YAML-подобного frontmatter в markdown-файлах.
//!
//! Рукописный (без serde_yaml), совместимый с форматом базы знаний: блок между
//! первыми `---`, скаляры `key: value`, списки inline `key: [a, b]` или multiline
//! `key:\n  - a\n  - b`. Используется загрузчиками базы знаний и каталога навыков.

/// Разделить файл на frontmatter (между первыми `---`) и тело.
pub fn split_frontmatter(raw: &str) -> (Option<String>, String) {
    // Frontmatter должен начинаться с первой строки.
    if !raw.starts_with("---") {
        return (None, raw.to_string());
    }

    // Ищем закрывающий `---` начиная со второй строки.
    let after_open = match raw.find('\n') {
        Some(pos) => &raw[pos + 1..],
        None => return (None, raw.to_string()),
    };

    let close_marker = "\n---";
    if let Some(close_pos) = after_open.find(close_marker) {
        let fm = after_open[..close_pos].to_string();
        let body_start = close_pos + close_marker.len();
        let body = after_open[body_start..].to_string();
        (Some(fm), body)
    } else {
        (None, raw.to_string())
    }
}

/// Извлечь скалярное значение: `key: value`.
pub fn parse_scalar(frontmatter: &str, key: &str) -> Option<String> {
    let prefix = format!("{}:", key);
    for line in frontmatter.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            let value = rest.trim().trim_matches('"').trim_matches('\'').to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

/// Извлечь целое число: `key: 42`. Нечисловое значение → `None` + warn в лог.
pub fn parse_u32(frontmatter: &str, key: &str) -> Option<u32> {
    let raw = parse_scalar(frontmatter, key)?;
    match raw.parse::<u32>() {
        Ok(value) => Some(value),
        Err(_) => {
            tracing::warn!("frontmatter: поле '{}' не число: '{}'", key, raw);
            None
        }
    }
}

/// Извлечь дату: `key: 2026-02-25`. Терпит datetime-суффикс (`2026-02-25T10:00:00Z`).
pub fn parse_date(frontmatter: &str, key: &str) -> Option<chrono::NaiveDate> {
    let raw = parse_scalar(frontmatter, key)?;
    // Отрезаем время, если оно есть: берём первые 10 символов вида YYYY-MM-DD.
    let date_part = raw.split(['T', ' ']).next().unwrap_or(&raw);
    match chrono::NaiveDate::parse_from_str(date_part, "%Y-%m-%d") {
        Ok(date) => Some(date),
        Err(_) => {
            tracing::warn!("frontmatter: поле '{}' не дата YYYY-MM-DD: '{}'", key, raw);
            None
        }
    }
}

/// Извлечь список значений из inline-формата `key: [val1, val2]`
/// или multiline-формата:
/// ```yaml
/// key:
///   - val1
///   - val2
/// ```
pub fn parse_list(frontmatter: &str, key: &str) -> Option<Vec<String>> {
    let lines: Vec<&str> = frontmatter.lines().collect();
    let prefix = format!("{}:", key);

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            let rest = rest.trim();

            // Inline: `tags: [a020, wildberries]`
            if rest.starts_with('[') && rest.ends_with(']') {
                let inner = &rest[1..rest.len() - 1];
                let items = inner
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>();
                return Some(items);
            }

            // Inline одно значение без скобок.
            if !rest.is_empty() {
                return Some(vec![rest.to_string()]);
            }

            // Multiline: следующие строки начинаются с `  - `.
            let mut items = Vec::new();
            for subsequent in &lines[i + 1..] {
                let s = subsequent.trim();
                if s.starts_with("- ") {
                    items.push(
                        s[2..]
                            .trim()
                            .trim_matches('"')
                            .trim_matches('\'')
                            .to_string(),
                    );
                } else if !s.is_empty() && !s.starts_with('#') {
                    break;
                }
            }
            if !items.is_empty() {
                return Some(items);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const FM: &str = "title: Тест\nstars: 4\nttl_days: \"90\"\nverified:\nupdated: 2026-02-25\n\
                      touched: 2026-02-25T10:00:00Z\nbroken_num: abc\nbroken_date: вчера\n";

    #[test]
    fn parse_u32_reads_plain_and_quoted() {
        assert_eq!(parse_u32(FM, "stars"), Some(4));
        assert_eq!(parse_u32(FM, "ttl_days"), Some(90));
    }

    #[test]
    fn parse_u32_rejects_garbage_and_missing() {
        assert_eq!(parse_u32(FM, "broken_num"), None);
        assert_eq!(parse_u32(FM, "absent"), None);
    }

    #[test]
    fn parse_date_reads_date_and_tolerates_time_suffix() {
        let expected = chrono::NaiveDate::from_ymd_opt(2026, 2, 25).unwrap();
        assert_eq!(parse_date(FM, "updated"), Some(expected));
        assert_eq!(parse_date(FM, "touched"), Some(expected));
    }

    #[test]
    fn parse_date_rejects_garbage_and_missing() {
        assert_eq!(parse_date(FM, "broken_date"), None);
        assert_eq!(parse_date(FM, "absent"), None);
    }

    #[test]
    fn empty_scalar_reads_as_none() {
        // `verified:` без значения — легальный способ сказать «не верифицировано».
        assert_eq!(parse_scalar(FM, "verified"), None);
        assert_eq!(parse_date(FM, "verified"), None);
    }
}
