//! База знаний для LLM в формате Obsidian.
//!
//! Сканирует директорию `knowledge_base_path` из конфига, парсит YAML frontmatter
//! каждого MD-файла и строит in-memory индекс `tag → [doc_id]`.
//!
//! Инструменты LLM:
//! - `search_knowledge(tags)` — поиск по тегам, возвращает список (id, title)
//! - `get_knowledge(id)` — полный текст документа без frontmatter

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ─── Структуры ───────────────────────────────────────────────────────────────

/// Один документ базы знаний (один MD-файл).
#[derive(Debug, Clone)]
pub struct KnowledgeDoc {
    /// Slug-идентификатор (имя файла без `.md`)
    pub id: String,
    /// Заголовок из frontmatter `title:`
    pub title: String,
    /// Теги из frontmatter `tags: [...]`
    pub tags: Vec<String>,
    /// Связанные агрегаты из frontmatter `related: [...]`
    pub related: Vec<String>,
    /// Тело MD без frontmatter-блока
    pub content: String,
    pub source_path: Option<String>,
}

/// In-memory база знаний со встроенным тег-индексом.
pub struct KnowledgeBase {
    /// tag (lowercase) → список doc_id
    index: HashMap<String, Vec<String>>,
    /// doc_id → документ
    docs: HashMap<String, KnowledgeDoc>,
}

// ─── Глобальный синглтон ─────────────────────────────────────────────────────

pub static KNOWLEDGE_BASE: Lazy<KnowledgeBase> = Lazy::new(|| {
    let path = match crate::shared::config::load_config() {
        Ok(cfg) => crate::shared::config::get_knowledge_base_path(&cfg),
        Err(e) => {
            tracing::warn!(
                "KnowledgeBase: cannot load config: {}; using default path",
                e
            );
            std::path::PathBuf::from("data/knowledge")
        }
    };
    KnowledgeBase::load(&path)
});

struct EmbeddedKnowledgeSource {
    id: &'static str,
    source_path: &'static str,
    raw: &'static str,
}

const EMBEDDED_LLM_DOCS: &[EmbeddedKnowledgeSource] = &[
    EmbeddedKnowledgeSource {
        id: "domain-a012_wb_sales",
        source_path: "crates/backend/src/domain/a012_wb_sales/llm.md",
        raw: include_str!("../../domain/a012_wb_sales/llm.md"),
    },
    EmbeddedKnowledgeSource {
        id: "usecase-u504_import_from_wildberries",
        source_path: "crates/backend/src/usecases/u504_import_from_wildberries/llm.md",
        raw: include_str!("../../usecases/u504_import_from_wildberries/llm.md"),
    },
    EmbeddedKnowledgeSource {
        id: "projection-p904_sales_data",
        source_path: "crates/backend/src/projections/p904_sales_data/llm.md",
        raw: include_str!("../../projections/p904_sales_data/llm.md"),
    },
    EmbeddedKnowledgeSource {
        id: "data-view-dv001",
        source_path: "crates/backend/src/data_view/dv001/llm.md",
        raw: include_str!("../../data_view/dv001/llm.md"),
    },
    EmbeddedKnowledgeSource {
        id: "domain-a024_bi_indicator",
        source_path: "crates/backend/src/domain/a024_bi_indicator/llm.md",
        raw: include_str!("../../domain/a024_bi_indicator/llm.md"),
    },
    EmbeddedKnowledgeSource {
        id: "domain-a025_bi_dashboard",
        source_path: "crates/backend/src/domain/a025_bi_dashboard/llm.md",
        raw: include_str!("../../domain/a025_bi_dashboard/llm.md"),
    },
];

// ─── Реализация ──────────────────────────────────────────────────────────────

impl KnowledgeBase {
    /// Загрузить все `*.md` файлы из директории `dir`.
    pub fn load(dir: &Path) -> Self {
        let mut docs: HashMap<String, KnowledgeDoc> = HashMap::new();
        let mut index: HashMap<String, Vec<String>> = HashMap::new();

        if !dir.exists() {
            tracing::warn!(
                "KnowledgeBase: directory '{}' does not exist; knowledge tools will return empty results",
                dir.display()
            );
            let mut kb = Self { index, docs };
            kb.load_embedded_sources();
            return kb;
        }

        let mut loaded = 0usize;
        for path in walk_markdown_files(dir) {
            let id = build_doc_id(dir, &path);
            if id.is_empty() {
                continue;
            }

            match std::fs::read_to_string(&path) {
                Ok(raw) => {
                    let mut doc = parse_doc(id, &raw);
                    doc.source_path = Some(path.display().to_string());
                    insert_doc(&mut docs, &mut index, doc);
                    loaded += 1;
                }
                Err(e) => {
                    tracing::warn!("KnowledgeBase: cannot read '{}': {}", path.display(), e);
                }
            }
        }

        let mut kb = Self { index, docs };
        kb.load_embedded_sources();

        tracing::info!(
            "KnowledgeBase: loaded {} documents from '{}'",
            loaded,
            dir.display()
        );
        kb
    }

    /// Поиск по тегам (OR-семантика: совпадение хотя бы по одному тегу).
    /// Возвращает документы без дублей, отсортированные по количеству совпавших тегов.
    pub fn search_by_tags(&self, tags: &[&str]) -> Vec<&KnowledgeDoc> {
        let mut scores: HashMap<&str, usize> = HashMap::new();

        for tag in tags {
            let key = tag.to_lowercase();
            if let Some(ids) = self.index.get(&key) {
                for id in ids {
                    *scores.entry(id.as_str()).or_default() += 1;
                }
            }
        }

        let mut results: Vec<(&str, usize)> = scores.into_iter().collect();
        results.sort_by(|a, b| b.1.cmp(&a.1));

        results
            .into_iter()
            .filter_map(|(id, _)| self.docs.get(id))
            .collect()
    }

    /// Получить документ по id.
    pub fn get(&self, id: &str) -> Option<&KnowledgeDoc> {
        self.docs.get(id)
    }

    /// Вернуть все документы (для отладки).
    pub fn all_docs(&self) -> Vec<&KnowledgeDoc> {
        self.docs.values().collect()
    }

    fn load_embedded_sources(&mut self) {
        for source in EMBEDDED_LLM_DOCS {
            let mut doc = parse_doc(source.id.to_string(), source.raw);
            doc.source_path = Some(source.source_path.to_string());
            insert_doc(&mut self.docs, &mut self.index, doc);
        }
    }
}

fn walk_markdown_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];

    while let Some(current_dir) = stack.pop() {
        let entries = match std::fs::read_dir(&current_dir) {
            Ok(entries) => entries,
            Err(error) => {
                tracing::warn!(
                    "KnowledgeBase: cannot read directory '{}': {}",
                    current_dir.display(),
                    error
                );
                continue;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }

            if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                files.push(path);
            }
        }
    }

    files.sort();
    files
}

fn build_doc_id(base_dir: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(base_dir).unwrap_or(path);
    let segments = relative
        .iter()
        .filter_map(|part| part.to_str())
        .map(|part| part.trim_end_matches(".md"))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    if segments.is_empty() {
        String::new()
    } else if segments.len() == 1 {
        segments[0].to_string()
    } else {
        segments.join("__")
    }
}

// ─── Парсинг frontmatter ─────────────────────────────────────────────────────

/// Разобрать MD-файл: извлечь frontmatter и тело.
fn parse_doc(id: String, raw: &str) -> KnowledgeDoc {
    let (frontmatter, content) = split_frontmatter(raw);

    let title = frontmatter
        .as_deref()
        .and_then(|fm| parse_scalar(fm, "title"))
        .unwrap_or_else(|| id.replace('-', " "));

    let tags = frontmatter
        .as_deref()
        .and_then(|fm| parse_list(fm, "tags"))
        .unwrap_or_default();

    let related = frontmatter
        .as_deref()
        .and_then(|fm| parse_list(fm, "related"))
        .unwrap_or_default();

    KnowledgeDoc {
        id,
        title,
        tags,
        related,
        content: content.trim_start().to_string(),
        source_path: None,
    }
}

fn insert_doc(
    docs: &mut HashMap<String, KnowledgeDoc>,
    index: &mut HashMap<String, Vec<String>>,
    doc: KnowledgeDoc,
) {
    let id = doc.id.clone();
    for tag in &doc.tags {
        let bucket = index.entry(tag.to_lowercase()).or_default();
        if !bucket.iter().any(|existing| existing == &id) {
            bucket.push(id.clone());
        }
    }
    docs.insert(id, doc);
}

/// Разделить файл на frontmatter (между первыми `---`) и тело.
fn split_frontmatter(raw: &str) -> (Option<String>, String) {
    // Frontmatter должен начинаться с первой строки
    if !raw.starts_with("---") {
        return (None, raw.to_string());
    }

    // Ищем закрывающий `---` начиная со второй строки
    let after_open = match raw.find('\n') {
        Some(pos) => &raw[pos + 1..],
        None => return (None, raw.to_string()),
    };

    // Ищем `---` в начале строки
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

/// Извлечь скалярное значение: `key: value`
fn parse_scalar(frontmatter: &str, key: &str) -> Option<String> {
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

/// Извлечь список значений из inline-формата `key: [val1, val2]`
/// или multiline-формата:
/// ```yaml
/// key:
///   - val1
///   - val2
/// ```
fn parse_list(frontmatter: &str, key: &str) -> Option<Vec<String>> {
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

            // Inline одно значение без скобок
            if !rest.is_empty() {
                return Some(vec![rest.to_string()]);
            }

            // Multiline: следующие строки начинаются с `  - `
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

// ─── Тесты ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"---
title: Акции Wildberries
tags: [a020, wildberries, акции, скидки]
related: [a006, a012]
updated: 2026-02-25
---

# Акции Wildberries

Текст статьи.
"#;

    #[test]
    fn test_parse_frontmatter() {
        let doc = parse_doc("wb-promotions".to_string(), SAMPLE);
        assert_eq!(doc.title, "Акции Wildberries");
        assert!(doc.tags.contains(&"a020".to_string()));
        assert!(doc.tags.contains(&"скидки".to_string()));
        assert!(doc.related.contains(&"a006".to_string()));
        assert!(doc.content.contains("# Акции Wildberries"));
        assert!(!doc.content.contains("---"));
    }

    #[test]
    fn test_search_by_tags() {
        let mut kb = KnowledgeBase {
            index: HashMap::new(),
            docs: HashMap::new(),
        };
        let doc = parse_doc("wb-promotions".to_string(), SAMPLE);
        for tag in &doc.tags {
            kb.index
                .entry(tag.to_lowercase())
                .or_default()
                .push(doc.id.clone());
        }
        kb.docs.insert(doc.id.clone(), doc);

        let results = kb.search_by_tags(&["a020"]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "wb-promotions");

        let no_results = kb.search_by_tags(&["nonexistent"]);
        assert!(no_results.is_empty());
    }

    #[test]
    fn test_multiline_tags() {
        let fm = "title: Test\ntags:\n  - a020\n  - wildberries\n";
        let tags = parse_list(fm, "tags").unwrap();
        assert_eq!(tags, vec!["a020", "wildberries"]);
    }

    #[test]
    fn test_build_doc_id_for_nested_path() {
        let base = Path::new("data/knowledge");
        let nested = Path::new("data/knowledge/marketplaces/wildberries/p909-p910-projections.md");
        assert_eq!(
            build_doc_id(base, nested),
            "marketplaces__wildberries__p909-p910-projections"
        );
    }
}
