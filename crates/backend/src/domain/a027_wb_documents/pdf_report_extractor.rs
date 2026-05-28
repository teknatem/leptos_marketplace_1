use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use anyhow::{anyhow, Context, Result};
use chrono::{Datelike, Duration, NaiveDate};
use contracts::domain::a027_wb_documents::aggregate::WbWeeklyReportManualData;
use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;
use zip::ZipArchive;

static ROW_CODE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\s*(\d+(?:\.\d+)?)(?:\.)?\s+(.+)$").expect("valid row code regex"));
static MONEY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[-−]?\d{1,3}(?:[ \u{00a0}]\d{3})*(?:[,.]\d{2})|[-−]?\d+(?:[,.]\d{2})")
        .expect("valid money regex")
});
static DATE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(\d{2})\.(\d{2})\.(\d{4})\b").expect("valid date regex"));
static ISO_DATE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(\d{4})-(\d{2})-(\d{2})\b").expect("valid ISO date regex"));
static PERIOD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)[сc]\s+(\d{2})\.(\d{2})\.(\d{4})\s+(?:по|до)\s+(\d{2})\.(\d{2})\.(\d{4})")
        .expect("valid period regex")
});
static REPORT_DATE_ISO_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bот\s+(\d{4})-(\d{2})-(\d{2})\b").expect("valid ISO report date regex")
});
static REPORT_DATE_DMY_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\bот\s+(\d{2})\.(\d{2})\.(\d{4})\b").expect("valid DMY report date regex")
});

#[derive(Debug, Clone, Serialize)]
pub struct WbWeeklyReportExtraction {
    pub report_title: Option<String>,
    pub report_date: Option<String>,
    pub report_period_from: Option<String>,
    pub report_period_to: Option<String>,
    pub rows: Vec<WbWeeklyReportRow>,
    pub manual_data: WbWeeklyReportManualData,
}

#[derive(Debug, Clone, Serialize)]
pub struct WbWeeklyReportRow {
    pub code: String,
    pub name: String,
    pub amount: Option<f64>,
    pub vat_amount: Option<f64>,
    pub raw_text: String,
}

pub fn extract_weekly_report_from_document_bytes(
    bytes: &[u8],
    extension: &str,
    file_name: Option<&str>,
) -> Result<WbWeeklyReportExtraction> {
    let pdf_bytes = extract_pdf_bytes(bytes, extension)?;
    extract_weekly_report_from_pdf(&pdf_bytes, file_name)
}

pub fn extract_weekly_report_from_pdf(
    pdf_bytes: &[u8],
    file_name: Option<&str>,
) -> Result<WbWeeklyReportExtraction> {
    let text = extract_pdf_text(pdf_bytes)?;
    extract_weekly_report_from_text(&text, file_name)
}

/// `pdf_extract` (and the `lopdf` it builds on) can `panic!` on malformed or
/// unusual PDFs instead of returning an error. Catch the unwind so a bad
/// document yields a clean `Err` rather than aborting the request task and
/// dropping the connection.
fn extract_pdf_text(pdf_bytes: &[u8]) -> Result<String> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        pdf_extract::extract_text_from_mem(pdf_bytes)
    }));

    match result {
        Ok(text) => text.context("failed to extract PDF text"),
        Err(_) => Err(anyhow!("PDF text extraction panicked (malformed document)")),
    }
}

pub fn extract_weekly_report_from_text(
    text: &str,
    file_name: Option<&str>,
) -> Result<WbWeeklyReportExtraction> {
    let title = text
        .lines()
        .map(normalize_space)
        .find(|line| line.contains("Отчет Вайлдберриз") || line.contains("Отчёт Вайлдберриз"));
    // Дата отчёта берётся в первую очередь из имени документа/файла
    // ("Отчет № … от DD.MM.YYYY") — это самый надёжный якорь начала недельного
    // периода. Тело PDF используем только как запасной вариант: там встречаются
    // посторонние даты (дата формирования отчёта и т.п.), которые сбивают период.
    let report_date = report_date_from_file_name(file_name).or_else(|| extract_report_date(text));
    // Недельный отчёт WB всегда покрывает понедельник–воскресенье, поэтому от
    // даты отчёта восстанавливаем полную неделю. Скан периода из тела PDF — тоже
    // запасной вариант.
    let (period_from, period_to) = report_date
        .as_deref()
        .and_then(weekly_period_from_report_date)
        .or_else(|| extract_period(text))
        .unwrap_or((None, None));
    let rows = extract_rows(text);
    let rows_by_code: BTreeMap<String, WbWeeklyReportRow> = rows
        .iter()
        .cloned()
        .map(|row| (row.code.clone(), row))
        .collect();

    let manual_data = WbWeeklyReportManualData {
        realized_goods_total: amount_for(&rows_by_code, "1.1"),
        wb_reward_with_vat: reward_with_vat(&rows_by_code),
        // Строка 8 («Итого к перечислению Продавцу») — это итоговая сумма с
        // одним значением и без НДС, к тому же последняя кодовая строка таблицы:
        // после неё в текст подклеиваются числа из футера/соседних строк.
        // Поэтому берём первую сумму строки (само значение идёт раньше мусора),
        // а не second-to-last, как для строк основной таблицы с парой сумма+НДС.
        seller_transfer_total: first_amount_for(&rows_by_code, "8"),
        other_deductions: amount_for(&rows_by_code, "2.10"),
        logistics: sum_amounts(&rows_by_code, &["2.7", "2.8"]),
        acquiring: amount_for(&rows_by_code, "2.6"),
    };

    Ok(WbWeeklyReportExtraction {
        report_title: title,
        report_date,
        report_period_from: period_from,
        report_period_to: period_to,
        rows,
        manual_data,
    })
}

fn extract_pdf_bytes(bytes: &[u8], extension: &str) -> Result<Vec<u8>> {
    if extension.eq_ignore_ascii_case("pdf") || bytes.starts_with(b"%PDF") {
        return Ok(bytes.to_vec());
    }

    if !extension.eq_ignore_ascii_case("zip") {
        anyhow::bail!(
            "document extension '{}' is not supported for PDF extraction",
            extension
        );
    }

    let cursor = Cursor::new(bytes);
    let mut archive = ZipArchive::new(cursor).context("failed to open document zip")?;
    for index in 0..archive.len() {
        let mut file = archive.by_index(index)?;
        if !file.name().to_ascii_lowercase().ends_with(".pdf") {
            continue;
        }

        let mut pdf = Vec::new();
        file.read_to_end(&mut pdf)
            .context("failed to read PDF from zip")?;
        if pdf.starts_with(b"%PDF") {
            return Ok(pdf);
        }
        return Err(anyhow!("zip entry '{}' is not a PDF file", file.name()));
    }

    Err(anyhow!("zip archive does not contain a PDF file"))
}

fn extract_rows(text: &str) -> Vec<WbWeeklyReportRow> {
    let mut rows = Vec::new();
    let mut current: Option<(String, Vec<String>)> = None;

    for raw_line in text.lines() {
        let line = normalize_space(raw_line);
        if line.is_empty() {
            continue;
        }

        if let Some(caps) = ROW_CODE_RE.captures(&line) {
            if let Some((code, parts)) = current.take() {
                rows.push(build_row(code, &parts.join(" ")));
            }
            current = Some((caps[1].to_string(), vec![caps[2].to_string()]));
        } else if let Some((_, parts)) = current.as_mut() {
            parts.push(line);
        }
    }

    if let Some((code, parts)) = current {
        rows.push(build_row(code, &parts.join(" ")));
    }

    rows.into_iter()
        .filter(|row| row.amount.is_some() || is_expected_code(&row.code))
        .collect()
}

fn build_row(code: String, rest: &str) -> WbWeeklyReportRow {
    let amounts: Vec<f64> = collect_amounts(rest);
    let amount = amounts.get(amounts.len().saturating_sub(2)).copied();
    let vat_amount = amounts.last().copied();
    let name = trim_name(rest);

    WbWeeklyReportRow {
        code,
        name,
        amount,
        vat_amount,
        raw_text: rest.to_string(),
    }
}

fn trim_name(value: &str) -> String {
    let stop = MONEY_RE
        .find(value)
        .map(|m| m.start())
        .unwrap_or(value.len());
    value[..stop]
        .trim_matches(|ch: char| ch.is_whitespace() || ch == '-' || ch == '—')
        .to_string()
}

fn extract_period(text: &str) -> Option<(Option<String>, Option<String>)> {
    if let Some(caps) = PERIOD_RE.captures(text) {
        return Some((
            Some(format!("{}-{}-{}", &caps[3], &caps[2], &caps[1])),
            Some(format!("{}-{}-{}", &caps[6], &caps[5], &caps[4])),
        ));
    }

    let line_with_period = text.lines().find(|line| {
        let lower = line.to_lowercase();
        (lower.contains("период") || lower.contains("отчет") || lower.contains("отчёт"))
            && DATE_RE.find_iter(line).count() >= 2
    });
    let Some(source) = line_with_period else {
        return None;
    };
    let dates: Vec<String> = DATE_RE
        .captures_iter(source)
        .take(2)
        .map(|caps| format!("{}-{}-{}", &caps[3], &caps[2], &caps[1]))
        .collect();

    if dates.len() >= 2 {
        Some((dates.get(0).cloned(), dates.get(1).cloned()))
    } else {
        None
    }
}

fn extract_report_date(source: &str) -> Option<String> {
    REPORT_DATE_ISO_RE
        .captures(source)
        .map(|caps| format!("{}-{}-{}", &caps[1], &caps[2], &caps[3]))
        .or_else(|| {
            REPORT_DATE_DMY_RE
                .captures(source)
                .map(|caps| format!("{}-{}-{}", &caps[3], &caps[2], &caps[1]))
        })
        .or_else(|| {
            ISO_DATE_RE
                .captures(source)
                .map(|caps| format!("{}-{}-{}", &caps[1], &caps[2], &caps[3]))
        })
}

/// Дата отчёта из имени документа/файла. Ищем только метку "от DD.MM.YYYY"
/// (или ISO) в самом имени, не в теле PDF, чтобы не подхватить постороннюю дату.
fn report_date_from_file_name(file_name: Option<&str>) -> Option<String> {
    let name = file_name?;
    REPORT_DATE_ISO_RE
        .captures(name)
        .map(|caps| format!("{}-{}-{}", &caps[1], &caps[2], &caps[3]))
        .or_else(|| {
            REPORT_DATE_DMY_RE
                .captures(name)
                .map(|caps| format!("{}-{}-{}", &caps[3], &caps[2], &caps[1]))
        })
}

/// Недельный отчёт WB всегда покрывает понедельник–воскресенье. Привязываем
/// начало периода к понедельнику недели, в которую попадает дата отчёта, а конец
/// — к воскресенью (начало + 6 дней). Так период остаётся корректным, даже если
/// дата отчёта пришлась на другой день недели.
fn weekly_period_from_report_date(report_date: &str) -> Option<(Option<String>, Option<String>)> {
    let date = NaiveDate::parse_from_str(report_date, "%Y-%m-%d").ok()?;
    let from = date - Duration::days(date.weekday().num_days_from_monday() as i64);
    let to = from.checked_add_signed(Duration::days(6))?;
    Some((
        Some(from.format("%Y-%m-%d").to_string()),
        Some(to.format("%Y-%m-%d").to_string()),
    ))
}

/// Собирает денежные значения из строки, защищаясь от «склейки» с соседним
/// идентификатором. В выгрузке PDF номер отчёта (напр. `678987893`) попадает
/// вплотную к сумме: `…678987893 234,56`. Регэксп с разделителем тысяч читает
/// хвост номера `893` как группу тысяч → `893234.56`. Признак склейки —
/// совпадение начинается сразу после цифры (для настоящей суммы перед ней
/// всегда пробел, тире или текст). В этом случае отбрасываем первую группу
/// (хвост идентификатора) и берём остаток.
fn collect_amounts(rest: &str) -> Vec<f64> {
    let bytes = rest.as_bytes();
    MONEY_RE
        .find_iter(rest)
        .filter_map(|m| {
            let token = m.as_str();
            let glued_to_digit = m.start() > 0 && bytes[m.start() - 1].is_ascii_digit();
            if glued_to_digit {
                // Отрезаем первую группу до разделителя тысяч и парсим остаток.
                match token.splitn(2, [' ', '\u{00a0}']).nth(1) {
                    Some(tail) => parse_amount(tail),
                    None => None,
                }
            } else {
                parse_amount(token)
            }
        })
        .collect()
}

fn parse_amount(value: &str) -> Option<f64> {
    let normalized = value
        .trim()
        .replace([' ', '\u{00a0}'], "")
        .replace('−', "-")
        .replace(',', ".");
    normalized.parse::<f64>().ok()
}

fn amount_for(rows_by_code: &BTreeMap<String, WbWeeklyReportRow>, code: &str) -> Option<f64> {
    rows_by_code.get(code).and_then(|row| row.amount)
}

/// Первая денежная сумма в строке (в порядке чтения), извлечённая из исходного
/// текста. Используется для итоговых строк с одним значением, где second-to-last
/// эвристика ломается из-за подклеенных футер-чисел.
fn first_amount_for(rows_by_code: &BTreeMap<String, WbWeeklyReportRow>, code: &str) -> Option<f64> {
    rows_by_code
        .get(code)
        .and_then(|row| collect_amounts(&row.raw_text).into_iter().next())
}

/// Вознаграждение WB с НДС (показатель 2.1 + 2.2).
///
/// В отчёте это две суммы — вознаграждение без НДС и НДС с вознаграждения. Итог
/// — это удержание, и его знак важен для сверки: в книге проводок
/// `mp_commission` хранится отрицательным, а сверка считает `отчёт − проводки`.
/// Величину берём как сумму модулей (знаки колонок в выгрузке PDF нестабильны),
/// но если хотя бы одна из составляющих в отчёте идёт со знаком «минус»
/// (удержание), итог делаем отрицательным.
///
/// Поддерживаем две раскладки таблицы:
/// 1. Раздельные строки 2.1 (без НДС) и 2.2 (НДС) — берём суммы обеих строк.
/// 2. Единая строка 2.1 (или 2) с двумя колонками — берём `amount` (без НДС) и
///    `vat_amount` (НДС) одной строки.
fn reward_with_vat(rows_by_code: &BTreeMap<String, WbWeeklyReportRow>) -> Option<f64> {
    if let (Some(base), Some(vat)) = (rows_by_code.get("2.1"), rows_by_code.get("2.2")) {
        let base_amount = base.amount?;
        return Some(signed_reward(base_amount, vat.amount.unwrap_or(0.0)));
    }

    let row = rows_by_code.get("2.1").or_else(|| rows_by_code.get("2"))?;
    let base_amount = row.amount?;
    Some(signed_reward(base_amount, row.vat_amount.unwrap_or(0.0)))
}

/// Складывает вознаграждение (2.1) и НДС (2.2) по модулю, но сохраняет знак
/// удержания: если в отчёте хотя бы одна из сумм отрицательна, итог тоже
/// отрицательный.
fn signed_reward(base: f64, vat: f64) -> f64 {
    let magnitude = base.abs() + vat.abs();
    if base < 0.0 || vat < 0.0 {
        -magnitude
    } else {
        magnitude
    }
}

fn sum_amounts(rows_by_code: &BTreeMap<String, WbWeeklyReportRow>, codes: &[&str]) -> Option<f64> {
    let mut found = false;
    let total = codes.iter().fold(0.0, |acc, code| {
        if let Some(amount) = amount_for(rows_by_code, code) {
            found = true;
            acc + amount
        } else {
            acc
        }
    });
    found.then_some(total)
}

fn normalize_space(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_expected_code(code: &str) -> bool {
    matches!(
        code,
        "1" | "1.1"
            | "1.2"
            | "1.4"
            | "2"
            | "2.1"
            | "2.2"
            | "2.6"
            | "2.7"
            | "2.8"
            | "2.9"
            | "2.10"
            | "8"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_rows_and_maps_manual_data() {
        let text = r#"
            Отчет Вайлдберриз № 123 за период с 01.01.2026 по 07.01.2026
            Расчеты с Продавцом за текущий период
            1.1 Итого стоимость реализованного товара — — 1 234,56 0,00
            2.1 Вознаграждение Вайлдберриз — — 100,00 20,00
            2.2 НДС с вознаграждения Вайлдберриз — — 20,00 X
            2.6 Эквайринг — — 12,34 Х
            2.7 Логистика — — 30,00 —
            2.8 Хранение — — 40,00 —
            2.10 Прочие удержания — — 55,00 —
            8. Итого к перечислению Продавцу — — 976,22 —
        "#;

        let extracted = extract_weekly_report_from_text(text, None).unwrap();

        assert_eq!(extracted.report_period_from.as_deref(), Some("2026-01-01"));
        assert_eq!(extracted.report_period_to.as_deref(), Some("2026-01-07"));
        assert_eq!(extracted.manual_data.realized_goods_total, Some(1234.56));
        assert_eq!(extracted.manual_data.wb_reward_with_vat, Some(120.0));
        assert_eq!(extracted.manual_data.acquiring, Some(12.34));
        assert_eq!(extracted.manual_data.logistics, Some(70.0));
        assert_eq!(extracted.manual_data.other_deductions, Some(55.0));
        assert_eq!(extracted.manual_data.seller_transfer_total, Some(976.22));
    }

    #[test]
    fn reward_is_negative_deduction_when_report_shows_minus() {
        // В реальных отчётах удержания (вознаграждение и НДС) идут со знаком
        // «минус». Итог 2.1+2.2 складывает модули по величине, но сохраняет знак
        // удержания: при минусе хотя бы в одной строке итог отрицательный.
        let text = "\
            2.1 Вознаграждение Вайлдберриз — — -416 628,60
            2.2 НДС с вознаграждения Вайлдберриз — — -91 658,41";

        let extracted = extract_weekly_report_from_text(text, None).unwrap();
        let reward = extracted.manual_data.wb_reward_with_vat.unwrap();
        assert!((reward + 508_287.01).abs() < 1e-6, "got {reward}");
    }

    #[test]
    fn period_uses_file_name_date_over_body_date() {
        // В теле PDF может встретиться посторонняя дата ("от 2026-04-13" — дата
        // формирования отчёта). Период должен браться от даты из имени файла и
        // покрывать понедельник–воскресенье.
        let text = "\
            Отчет Вайлдберриз № 685740510 от 2026-04-13
            1.1 Итого стоимость реализованного товара — — 1 234,56 0,00";

        let extracted =
            extract_weekly_report_from_text(text, Some("Отчет №685740510 от 2026-04-06.pdf"))
                .unwrap();

        assert_eq!(extracted.report_period_from.as_deref(), Some("2026-04-06"));
        assert_eq!(extracted.report_period_to.as_deref(), Some("2026-04-12"));
    }

    #[test]
    fn weekly_period_snaps_to_monday_sunday() {
        // Даже если дата отчёта пришлась на середину недели, период привязывается
        // к понедельнику–воскресенью.
        let (from, to) = weekly_period_from_report_date("2026-04-08").unwrap();
        assert_eq!(from.as_deref(), Some("2026-04-06"));
        assert_eq!(to.as_deref(), Some("2026-04-12"));
    }

    #[test]
    fn reward_from_single_row_uses_amount_plus_vat() {
        // Единая строка с двумя колонками: вознаграждение без НДС + НДС.
        let text = "2.1 Вознаграждение Вайлдберриз — — 100,00 20,00";

        let extracted = extract_weekly_report_from_text(text, None).unwrap();
        assert_eq!(extracted.manual_data.wb_reward_with_vat, Some(120.0));
    }

    #[test]
    fn seller_transfer_takes_first_amount_despite_footer_pollution() {
        // Строка 8 — последняя кодовая строка; её метка переносится, а после
        // неё идёт футер с посторонними числами. Берём первое значение строки.
        let text = "\
            8. Итого к перечислению Продавцу за текущий период с учетом 4 802 355,80
            Вознаграждений и возвратов Товаров
            Корректировка -74 910,10
            Подпись 12 345,00";

        let extracted = extract_weekly_report_from_text(text, None).unwrap();
        assert_eq!(
            extracted.manual_data.seller_transfer_total,
            Some(4_802_355.80)
        );
    }

    #[test]
    fn ignores_report_number_glued_to_amount() {
        // Хвост номера отчёта 678987893 склеен с суммой 234,56.
        let row = build_row("2.6".to_string(), "Эквайринг 678987893 234,56");
        assert_eq!(row.amount, Some(234.56));

        // Сумма с настоящим разделителем тысяч не должна страдать.
        let clean = build_row("2.7".to_string(), "Логистика 1 234,56");
        assert_eq!(clean.amount, Some(1234.56));
    }

    #[test]
    fn derives_previous_week_from_report_file_date() {
        let extracted = extract_weekly_report_from_text(
            "Отчет Вайлдберриз № 686261594",
            Some("Отчет №686261594 от 2026-04-06.pdf"),
        )
        .unwrap();

        assert_eq!(extracted.report_date.as_deref(), Some("2026-04-06"));
        assert_eq!(extracted.report_period_from.as_deref(), Some("2026-04-06"));
        assert_eq!(extracted.report_period_to.as_deref(), Some("2026-04-12"));
    }
}
