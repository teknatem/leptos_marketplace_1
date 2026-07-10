//! Совместимость с моделями, которые возвращают вызовы инструментов ТЕКСТОМ, а не в
//! стандартном поле `tool_calls` ответа. Наблюдается у DeepSeek-v4 через OpenRouter: модель
//! эмитит разметку в своём формате (условно «DSML») прямо в `content`:
//!
//! ```text
//! <｜｜DSML｜｜tool_calls>
//! <｜｜DSML｜｜invoke name="execute_query">
//! <｜｜DSML｜｜parameter name="sql" string="true">SELECT ...</｜｜DSML｜｜parameter>
//! <｜｜DSML｜｜parameter name="limit" string="false">20</｜｜DSML｜｜parameter>
//! </｜｜DSML｜｜invoke>
//! </｜｜DSML｜｜tool_calls>
//! ```
//!
//! `string="true"` — значение строка, `string="false"` — сырой JSON (число/булево/…).
//! Парсер ТОЛЕРАНТЕН к точному виду маркера-разделителя: опознаёт по ключевым словам
//! `invoke name="…"` / `parameter name="…"`, поэтому не зависит от спецсимволов DeepSeek
//! и заодно ловит обычный `<invoke …>`-стиль.

use super::types::ToolCall;
use once_cell::sync::Lazy;
use regex::Regex;
use serde_json::{Map, Value};

static INVOKE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?s)<[^>]*?invoke\s+name="([^"]+)"[^>]*?>(.*?)</[^>]*?invoke>"#).unwrap()
});

static PARAM_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?s)<[^>]*?parameter\s+name="([^"]+)"(?:[^>]*?\bstring="(true|false)")?[^>]*?>(.*?)</[^>]*?parameter>"#,
    )
    .unwrap()
});

static WRAPPER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?s)</?[^>]*?tool_calls>"#).unwrap());

/// Если в тексте есть инлайновые вызовы инструментов — распарсить их в `ToolCall` и вернуть
/// `(вызовы, очищенный_текст)`. `None`, если разметки нет или ни один блок не распознан.
pub fn parse_inline_tool_calls(content: &str) -> Option<(Vec<ToolCall>, String)> {
    // Дешёвая отсечка: без ключевого слова не запускаем регекспы на обычных ответах.
    if !content.contains("invoke name=") {
        return None;
    }

    let mut calls = Vec::new();
    for (idx, caps) in INVOKE_RE.captures_iter(content).enumerate() {
        let name = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
        if name.is_empty() {
            continue;
        }
        let inner = caps.get(2).map(|m| m.as_str()).unwrap_or("");

        let mut args = Map::new();
        for pcaps in PARAM_RE.captures_iter(inner) {
            let Some(pname) = pcaps.get(1).map(|m| m.as_str().trim().to_string()) else {
                continue;
            };
            // По умолчанию (атрибут отсутствует) считаем значение строкой — безопаснее.
            let is_string = pcaps.get(2).map(|m| m.as_str() == "true").unwrap_or(true);
            let raw = pcaps.get(3).map(|m| m.as_str()).unwrap_or("");
            let value = if is_string {
                Value::String(raw.to_string())
            } else {
                // string="false" — сырой JSON (число/булево/массив). Фолбэк — строка.
                serde_json::from_str::<Value>(raw.trim())
                    .unwrap_or_else(|_| Value::String(raw.to_string()))
            };
            args.insert(pname, value);
        }

        calls.push(ToolCall {
            id: format!("inline_call_{idx}"),
            name: name.to_string(),
            arguments: Value::Object(args).to_string(),
        });
    }

    if calls.is_empty() {
        return None;
    }

    // Очистить текст: убрать invoke-блоки и обёртку tool_calls.
    let cleaned = INVOKE_RE.replace_all(content, "");
    let cleaned = WRAPPER_RE.replace_all(&cleaned, "");
    Some((calls, cleaned.trim().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Маркер DeepSeek использует U+FF5C (FULLWIDTH VERTICAL LINE); собираем его явно,
    /// чтобы тест был верен исходному формату.
    fn dsml(tag: &str) -> String {
        format!("\u{ff5c}\u{ff5c}DSML\u{ff5c}\u{ff5c}{tag}")
    }

    #[test]
    fn parses_deepseek_dsml_tool_call() {
        let input = format!(
            "<{tc}>\n<{inv} name=\"execute_query\">\n\
             <{p} name=\"description\" string=\"true\">Суммы продаж WB</{pc}>\n\
             <{p} name=\"limit\" string=\"false\">20</{pc}>\n\
             <{p} name=\"sql\" string=\"true\">SELECT subject_name FROM p903_wb_finance_report</{pc}>\n\
             </{invc}>\n</{tcc}>",
            tc = dsml("tool_calls"),
            inv = dsml("invoke"),
            invc = dsml("invoke"),
            p = dsml("parameter"),
            pc = dsml("parameter"),
            tcc = dsml("tool_calls"),
        );

        let (calls, cleaned) = parse_inline_tool_calls(&input).expect("should parse");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "execute_query");

        let args: Value = serde_json::from_str(&calls[0].arguments).unwrap();
        assert_eq!(
            args["sql"],
            "SELECT subject_name FROM p903_wb_finance_report"
        );
        assert_eq!(args["description"], "Суммы продаж WB");
        // string="false" → число, а не строка.
        assert_eq!(args["limit"], serde_json::json!(20));
        assert!(
            cleaned.is_empty(),
            "markup must be stripped, got: {cleaned:?}"
        );
    }

    #[test]
    fn parses_plain_invoke_style() {
        let input = "<invoke name=\"list_data_sources\"><parameter name=\"kind\" string=\"true\">base</parameter></invoke>";
        let (calls, _) = parse_inline_tool_calls(input).expect("should parse");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "list_data_sources");
        let args: Value = serde_json::from_str(&calls[0].arguments).unwrap();
        assert_eq!(args["kind"], "base");
    }

    #[test]
    fn ignores_normal_text() {
        assert!(parse_inline_tool_calls("Обычный ответ без инструментов.").is_none());
        assert!(parse_inline_tool_calls("Вот данные: 1) Тумба 2) Шкаф").is_none());
    }
}
