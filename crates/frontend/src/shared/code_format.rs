//! Simple HTML and CSS pretty-printers for BI indicator templates.
//!
//! These are intentionally minimal — no external dependencies, pure Rust,
//! good enough for the indented KPI-card snippets we work with.

// ============================================================================
// HTML formatter
// ============================================================================

/// Pretty-print a compact HTML string.
///
/// Rules:
/// - Block elements get their own line + indent.
/// - An element whose only child is a text node stays on one line:
///   `<div class="lbl">{{title}}</div>`
/// - Void/self-closing elements are single-line.
pub fn format_html(input: &str) -> String {
    let input = input.trim();
    if input.is_empty() {
        return String::new();
    }

    let tokens = tokenize_html(input);
    let mut out = String::new();
    let mut indent = 0usize;
    let mut i = 0;

    while i < tokens.len() {
        match &tokens[i] {
            HtmlToken::Open { name, raw, void } => {
                // Look-ahead: Open → Text → matching Close  ⟹  keep on one line
                let inline = !*void
                    && i + 2 < tokens.len()
                    && matches!(&tokens[i + 1], HtmlToken::Text(_))
                    && matches!(&tokens[i + 2], HtmlToken::Close { name: cn, .. } if cn == name);

                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(&"  ".repeat(indent));
                out.push_str(raw);

                if inline {
                    if let HtmlToken::Text(txt) = &tokens[i + 1] {
                        out.push_str(txt.trim());
                    }
                    if let HtmlToken::Close { raw: cr, .. } = &tokens[i + 2] {
                        out.push_str(cr);
                    }
                    i += 3;
                    continue;
                }

                if !*void {
                    indent += 1;
                }
            }

            HtmlToken::Close { raw, .. } => {
                if indent > 0 {
                    indent -= 1;
                }
                out.push('\n');
                out.push_str(&"  ".repeat(indent));
                out.push_str(raw);
            }

            HtmlToken::Text(txt) => {
                let t = txt.trim();
                if !t.is_empty() {
                    out.push_str(t);
                }
            }
        }
        i += 1;
    }

    out
}

// ----- tokenizer -----

#[derive(Debug)]
enum HtmlToken {
    Open { name: String, raw: String, void: bool },
    Close { name: String, raw: String },
    Text(String),
}

const VOID: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param",
    "source", "track", "wbr",
];

fn tokenize_html(input: &str) -> Vec<HtmlToken> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut pos = 0;

    while pos < chars.len() {
        if chars[pos] != '<' {
            // Text node
            let start = pos;
            while pos < chars.len() && chars[pos] != '<' {
                pos += 1;
            }
            let txt: String = chars[start..pos].iter().collect();
            if !txt.trim().is_empty() {
                tokens.push(HtmlToken::Text(txt));
            }
            continue;
        }

        // Tag — consume until `>`, respecting quoted attribute values
        let start = pos;
        pos += 1; // skip `<`
        let mut in_q: Option<char> = None;
        while pos < chars.len() {
            match (in_q, chars[pos]) {
                (None, '"') | (None, '\'') => {
                    in_q = Some(chars[pos]);
                }
                (Some(q), c) if c == q => {
                    in_q = None;
                }
                (None, '>') => {
                    pos += 1;
                    break;
                }
                _ => {}
            }
            pos += 1;
        }

        let raw: String = chars[start..pos].iter().collect();

        if raw.starts_with("</") {
            // Closing tag
            let name = raw[2..]
                .trim_end_matches(|c: char| c == '>' || c.is_whitespace())
                .to_lowercase();
            tokens.push(HtmlToken::Close { name, raw });
        } else {
            // Opening or self-closing
            let self_closing = raw.ends_with("/>");
            let inner = &raw[1..raw.len().saturating_sub(if self_closing { 2 } else { 1 })];
            let name_end = inner
                .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
                .unwrap_or(inner.len());
            let name = inner[..name_end].to_lowercase();
            let void = self_closing || VOID.contains(&name.as_str());
            tokens.push(HtmlToken::Open { name, raw, void });
        }
    }

    tokens
}

// ============================================================================
// CSS formatter
// ============================================================================

/// Pretty-print a minified CSS string.
///
/// ```text
/// .kpi{display:flex;gap:8px}.kpi__value{font-size:2rem}
/// →
/// .kpi {
///   display: flex;
///   gap: 8px;
/// }
/// .kpi__value {
///   font-size: 2rem;
/// }
/// ```
pub fn format_css(input: &str) -> String {
    let input = input.trim();
    if input.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    let mut buf = String::new();
    let mut in_block = false;
    let mut prev_closed = false;
    let mut in_q: Option<char> = None;

    for ch in input.chars() {
        // Handle quoted strings (e.g., font-family: 'Arial')
        if let Some(q) = in_q {
            buf.push(ch);
            if ch == q {
                in_q = None;
            }
            continue;
        }
        if ch == '"' || ch == '\'' {
            in_q = Some(ch);
            buf.push(ch);
            continue;
        }

        match ch {
            '{' => {
                let selector = normalise_ws(&buf);
                buf.clear();
                if !selector.is_empty() {
                    if prev_closed && !out.is_empty() {
                        out.push('\n');
                    }
                    out.push_str(&selector);
                    out.push_str(" {\n");
                }
                in_block = true;
                prev_closed = false;
            }
            '}' => {
                let prop = normalise_ws(&buf);
                buf.clear();
                if !prop.is_empty() {
                    push_property(&mut out, &prop);
                }
                out.push_str("}\n");
                in_block = false;
                prev_closed = true;
            }
            ';' if in_block => {
                let prop = normalise_ws(&buf);
                buf.clear();
                if !prop.is_empty() {
                    push_property(&mut out, &prop);
                }
            }
            '\n' | '\r' | '\t' => {
                // Collapse whitespace
                if !buf.ends_with(' ') && !buf.is_empty() {
                    buf.push(' ');
                }
            }
            _ => buf.push(ch),
        }
    }

    // Flush any remaining text
    let tail = normalise_ws(&buf);
    if !tail.is_empty() {
        out.push_str(&tail);
    }

    out.trim_end().to_string()
}

fn push_property(out: &mut String, prop: &str) {
    out.push_str("  ");
    out.push_str(prop);
    if !prop.ends_with(';') {
        out.push(';');
    }
    out.push('\n');
}

fn normalise_ws(s: &str) -> String {
    // Collapse multiple spaces into one, trim
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for c in s.chars() {
        if c == ' ' {
            if !prev_space && !out.is_empty() {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out.trim_end().to_string()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_css_basic() {
        let input = ".kpi{display:flex;gap:8px}.kpi__value{font-size:2rem}";
        let out = format_css(input);
        assert!(out.contains(".kpi {"));
        assert!(out.contains("  display: flex;"));
        assert!(out.contains("  gap: 8px;"));
        assert!(out.contains(".kpi__value {"));
        assert!(out.contains("  font-size: 2rem;"));
    }

    #[test]
    fn test_format_html_inline() {
        let input = r#"<div class="kpi"><div class="lbl">{{title}}</div></div>"#;
        let out = format_html(input);
        // Inner div with text should be on one line
        assert!(out.contains(r#"  <div class="lbl">{{title}}</div>"#));
    }
}
