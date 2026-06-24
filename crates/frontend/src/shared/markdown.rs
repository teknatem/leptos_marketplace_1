//! Лёгкий рендер Markdown для ответов LLM и просмотра контекста.
//!
//! Поддерживает: заголовки (#..####), маркированные и нумерованные списки,
//! блоки кода ```` ``` ````, цитаты `>`, простые таблицы `| a | b |`, а также
//! инлайн `**bold**`, `*italic*`, `` `code` ``. Не полноценный CommonMark, но
//! воспроизводит типичное форматирование ответов модели.

use leptos::prelude::*;

#[derive(Debug, Clone)]
enum Block {
    H1(String),
    H2(String),
    H3(String),
    Ul(Vec<String>),
    Ol(Vec<String>),
    Code(Vec<String>),
    Quote(Vec<String>),
    Table(Vec<Vec<String>>),
    Text(String),
    Empty,
}

fn is_table_sep(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells.iter().all(|c| {
            let t = c.trim();
            !t.is_empty() && t.chars().all(|ch| ch == '-' || ch == ':')
        })
}

fn split_row(line: &str) -> Vec<String> {
    let t = line.trim().trim_start_matches('|').trim_end_matches('|');
    t.split('|').map(|c| c.trim().to_string()).collect()
}

fn parse_blocks(text: &str) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut ul: Vec<String> = Vec::new();
    let mut ol: Vec<String> = Vec::new();
    let mut quote: Vec<String> = Vec::new();
    let mut table: Vec<Vec<String>> = Vec::new();
    let mut code: Option<Vec<String>> = None;

    fn flush_ul(b: &mut Vec<Block>, buf: &mut Vec<String>) {
        if !buf.is_empty() {
            b.push(Block::Ul(std::mem::take(buf)));
        }
    }
    fn flush_ol(b: &mut Vec<Block>, buf: &mut Vec<String>) {
        if !buf.is_empty() {
            b.push(Block::Ol(std::mem::take(buf)));
        }
    }
    fn flush_quote(b: &mut Vec<Block>, buf: &mut Vec<String>) {
        if !buf.is_empty() {
            b.push(Block::Quote(std::mem::take(buf)));
        }
    }
    fn flush_table(b: &mut Vec<Block>, buf: &mut Vec<Vec<String>>) {
        if !buf.is_empty() {
            b.push(Block::Table(std::mem::take(buf)));
        }
    }

    for line in text.lines() {
        // Блок кода — переключатель.
        if line.trim_start().starts_with("```") {
            if let Some(lines) = code.take() {
                blocks.push(Block::Code(lines));
            } else {
                flush_ul(&mut blocks, &mut ul);
                flush_ol(&mut blocks, &mut ol);
                flush_quote(&mut blocks, &mut quote);
                flush_table(&mut blocks, &mut table);
                code = Some(Vec::new());
            }
            continue;
        }
        if let Some(buf) = &mut code {
            buf.push(line.to_string());
            continue;
        }

        let trimmed = line.trim_start();

        // Таблица.
        if trimmed.starts_with('|') {
            flush_ul(&mut blocks, &mut ul);
            flush_ol(&mut blocks, &mut ol);
            flush_quote(&mut blocks, &mut quote);
            let cells = split_row(trimmed);
            if !is_table_sep(&cells) {
                table.push(cells);
            }
            continue;
        } else {
            flush_table(&mut blocks, &mut table);
        }

        // Заголовки.
        if let Some(rest) = trimmed
            .strip_prefix("#### ")
            .or_else(|| trimmed.strip_prefix("### "))
        {
            flush_ul(&mut blocks, &mut ul);
            flush_ol(&mut blocks, &mut ol);
            flush_quote(&mut blocks, &mut quote);
            blocks.push(Block::H3(rest.to_string()));
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            flush_ul(&mut blocks, &mut ul);
            flush_ol(&mut blocks, &mut ol);
            flush_quote(&mut blocks, &mut quote);
            blocks.push(Block::H2(rest.to_string()));
        } else if let Some(rest) = trimmed.strip_prefix("# ") {
            flush_ul(&mut blocks, &mut ul);
            flush_ol(&mut blocks, &mut ol);
            flush_quote(&mut blocks, &mut quote);
            blocks.push(Block::H1(rest.to_string()));
        } else if let Some(rest) = trimmed.strip_prefix("> ") {
            flush_ul(&mut blocks, &mut ul);
            flush_ol(&mut blocks, &mut ol);
            quote.push(rest.to_string());
        } else if let Some(rest) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            flush_ol(&mut blocks, &mut ol);
            flush_quote(&mut blocks, &mut quote);
            ul.push(rest.to_string());
        } else if let Some(rest) = strip_ordered(trimmed) {
            flush_ul(&mut blocks, &mut ul);
            flush_quote(&mut blocks, &mut quote);
            ol.push(rest);
        } else if trimmed.is_empty() {
            flush_ul(&mut blocks, &mut ul);
            flush_ol(&mut blocks, &mut ol);
            flush_quote(&mut blocks, &mut quote);
            if !matches!(blocks.last(), Some(Block::Empty)) {
                blocks.push(Block::Empty);
            }
        } else {
            flush_ul(&mut blocks, &mut ul);
            flush_ol(&mut blocks, &mut ol);
            flush_quote(&mut blocks, &mut quote);
            blocks.push(Block::Text(line.to_string()));
        }
    }

    flush_ul(&mut blocks, &mut ul);
    flush_ol(&mut blocks, &mut ol);
    flush_quote(&mut blocks, &mut quote);
    flush_table(&mut blocks, &mut table);
    if let Some(lines) = code.take() {
        blocks.push(Block::Code(lines));
    }
    blocks
}

/// `123. текст` → Some("текст").
fn strip_ordered(line: &str) -> Option<String> {
    let mut digits = 0;
    for c in line.chars() {
        if c.is_ascii_digit() {
            digits += 1;
        } else {
            break;
        }
    }
    if digits == 0 {
        return None;
    }
    let rest = &line[digits..];
    rest.strip_prefix(". ").map(|s| s.to_string())
}

// ── Инлайн-разметка ────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Span {
    Text(String),
    Bold(String),
    Italic(String),
    Code(String),
}

fn find_char(chars: &[char], from: usize, ch: char) -> Option<usize> {
    (from..chars.len()).find(|&i| chars[i] == ch)
}

fn find_double(chars: &[char], from: usize, ch: char) -> Option<usize> {
    let mut i = from;
    while i + 1 < chars.len() {
        if chars[i] == ch && chars[i + 1] == ch {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn parse_inline(input: &str) -> Vec<Span> {
    let chars: Vec<char> = input.chars().collect();
    let mut spans: Vec<Span> = Vec::new();
    let mut buf = String::new();
    let mut i = 0;

    fn flush(buf: &mut String, spans: &mut Vec<Span>) {
        if !buf.is_empty() {
            spans.push(Span::Text(std::mem::take(buf)));
        }
    }

    while i < chars.len() {
        let c = chars[i];
        // `code`
        if c == '`' {
            if let Some(end) = find_char(&chars, i + 1, '`') {
                flush(&mut buf, &mut spans);
                spans.push(Span::Code(chars[i + 1..end].iter().collect()));
                i = end + 1;
                continue;
            }
        }
        // **bold**
        if c == '*' && i + 1 < chars.len() && chars[i + 1] == '*' {
            if let Some(end) = find_double(&chars, i + 2, '*') {
                if end > i + 2 {
                    flush(&mut buf, &mut spans);
                    spans.push(Span::Bold(chars[i + 2..end].iter().collect()));
                    i = end + 2;
                    continue;
                }
            }
        }
        // *italic* (только звёздочка, чтобы не ломать snake_case)
        if c == '*' {
            if let Some(end) = find_char(&chars, i + 1, '*') {
                if end > i + 1 {
                    flush(&mut buf, &mut spans);
                    spans.push(Span::Italic(chars[i + 1..end].iter().collect()));
                    i = end + 1;
                    continue;
                }
            }
        }
        buf.push(c);
        i += 1;
    }
    flush(&mut buf, &mut spans);
    spans
}

fn render_inline(text: &str) -> impl IntoView {
    parse_inline(text)
        .into_iter()
        .map(|span| match span {
            Span::Text(t) => view! { <span>{t}</span> }.into_any(),
            Span::Bold(t) => view! { <strong>{t}</strong> }.into_any(),
            Span::Italic(t) => view! { <em>{t}</em> }.into_any(),
            Span::Code(t) => view! {
                <code style="background: var(--colorNeutralBackground3); padding: 0 4px; border-radius: 4px; font-family: var(--fontFamilyMonospace, monospace); font-size: 0.88em;">
                    {t}
                </code>
            }
            .into_any(),
        })
        .collect_view()
}

#[component]
#[allow(non_snake_case)]
pub fn Markdown(text: String) -> impl IntoView {
    let blocks = parse_blocks(&text);
    view! {
        <div class="md">
            {blocks.into_iter().map(|block| match block {
                Block::H1(t) => view! {
                    <div style="font-size: 1.25em; font-weight: 700; margin: 0.6em 0 0.25em;">{render_inline(&t)}</div>
                }.into_any(),
                Block::H2(t) => view! {
                    <div style="font-size: 1.12em; font-weight: 700; margin: 0.5em 0 0.2em;">{render_inline(&t)}</div>
                }.into_any(),
                Block::H3(t) => view! {
                    <div style="font-size: 1.0em; font-weight: 600; margin: 0.4em 0 0.15em;">{render_inline(&t)}</div>
                }.into_any(),
                Block::Ul(items) => view! {
                    <ul style="margin: 0.2em 0 0.2em 1.2em; padding: 0; list-style: disc;">
                        {items.into_iter().map(|item| view! {
                            <li style="margin: 0.12em 0;">{render_inline(&item)}</li>
                        }).collect_view()}
                    </ul>
                }.into_any(),
                Block::Ol(items) => view! {
                    <ol style="margin: 0.2em 0 0.2em 1.4em; padding: 0;">
                        {items.into_iter().map(|item| view! {
                            <li style="margin: 0.12em 0;">{render_inline(&item)}</li>
                        }).collect_view()}
                    </ol>
                }.into_any(),
                Block::Code(lines) => view! {
                    <pre style="background: var(--colorNeutralBackground3); padding: 8px 10px; border-radius: 6px; font-family: var(--fontFamilyMonospace, monospace); font-size: 0.85em; overflow-x: auto; margin: 0.35em 0; white-space: pre-wrap; word-break: break-word;">
                        {lines.join("\n")}
                    </pre>
                }.into_any(),
                Block::Quote(lines) => view! {
                    <div style="border-left: 3px solid var(--colorNeutralStroke2); padding: 2px 10px; margin: 0.3em 0; color: var(--colorNeutralForeground2);">
                        {lines.into_iter().map(|l| view! { <div>{render_inline(&l)}</div> }).collect_view()}
                    </div>
                }.into_any(),
                Block::Table(rows) => {
                    let mut iter = rows.into_iter();
                    let header = iter.next();
                    view! {
                        <table style="border-collapse: collapse; margin: 0.4em 0; font-size: 0.92em;">
                            {header.map(|h| view! {
                                <thead>
                                    <tr>
                                        {h.into_iter().map(|c| view! {
                                            <th style="border: 1px solid var(--colorNeutralStroke2); padding: 4px 8px; text-align: left; background: var(--colorNeutralBackground2);">
                                                {render_inline(&c)}
                                            </th>
                                        }).collect_view()}
                                    </tr>
                                </thead>
                            })}
                            <tbody>
                                {iter.map(|row| view! {
                                    <tr>
                                        {row.into_iter().map(|c| view! {
                                            <td style="border: 1px solid var(--colorNeutralStroke2); padding: 4px 8px;">
                                                {render_inline(&c)}
                                            </td>
                                        }).collect_view()}
                                    </tr>
                                }).collect_view()}
                            </tbody>
                        </table>
                    }.into_any()
                }
                Block::Text(t) => view! {
                    <div style="margin: 0.1em 0; line-height: 1.5;">{render_inline(&t)}</div>
                }.into_any(),
                Block::Empty => view! { <div style="height: 0.4em;"></div> }.into_any(),
            }).collect_view()}
        </div>
    }
}
