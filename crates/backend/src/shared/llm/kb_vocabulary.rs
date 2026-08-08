//! Контролируемый словарь тегов базы знаний.
//!
//! Живёт файлом `<kb_root>/_vocabulary.md` — куратор правит его в Obsidian рядом
//! со статьями, поэтому словарь не в коде и не в БД. Ведущее подчёркивание в имени
//! исключает файл из обхода статей (`walk_markdown_files`), загрузчик читает его явно.
//!
//! Формат: `##` — канонический тег, `- key: value` — атрибуты секции.
//!
//! ```markdown
//! ## воронка
//! - group: funnel
//! - label: Воронка продаж
//! - aliases: [funnel, sales-funnel, воронка-продаж]
//! - description: Путь товара показ → переход → корзина → заказ → выкуп.
//! ```
//!
//! Политика неизвестных тегов — **предупреждать и сохранять**: тег вне словаря
//! индексируется как есть и попадает в рабочий список куратора. Отклонять нельзя
//! (сломает существующие статьи и сделает правку в Obsidian враждебной), молча
//! усыновлять — тоже (это убивает смысл контроля).

use std::collections::{BTreeMap, HashMap};

/// Один канонический термин словаря.
#[derive(Debug, Clone, Default)]
pub struct Term {
    /// Канонический тег (ключ в индексе).
    pub tag: String,
    /// Группа для UI: marketplace / funnel / metric / quality / entity / doc-type.
    pub group: String,
    /// Человекочитаемая подпись; пусто → показывать `tag`.
    pub label: String,
    /// Синонимы, приводящиеся к `tag`.
    pub aliases: Vec<String>,
    pub description: String,
}

/// Словарь: канонические термины + плоская карта «алиас → канонический тег».
#[derive(Debug, Clone, Default)]
pub struct Vocabulary {
    canonical: BTreeMap<String, Term>,
    alias_to_canonical: HashMap<String, String>,
}

impl Vocabulary {
    /// Разобрать содержимое `_vocabulary.md`.
    pub fn parse(raw: &str) -> Self {
        // Frontmatter словаря (title/version) нам не нужен — отбрасываем.
        let (_, body) = super::frontmatter::split_frontmatter(raw);

        let mut canonical: BTreeMap<String, Term> = BTreeMap::new();
        let mut current: Option<Term> = None;

        for line in body.lines() {
            let trimmed = line.trim();

            // Секция начинается с `## <тег>`. Ровно два решётки, чтобы `# Заголовок`
            // документа и `### подпункт` не создавали фантомных терминов.
            if let Some(rest) = trimmed.strip_prefix("## ") {
                if let Some(term) = current.take() {
                    insert_term(&mut canonical, term);
                }
                let tag = normalize_form(rest);
                if !tag.is_empty() {
                    current = Some(Term {
                        tag,
                        ..Term::default()
                    });
                }
                continue;
            }

            let Some(term) = current.as_mut() else {
                continue;
            };
            let Some(attr) = trimmed.strip_prefix("- ") else {
                continue;
            };
            let Some((key, value)) = attr.split_once(':') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "group" => term.group = value.to_string(),
                "label" => term.label = value.to_string(),
                "description" => term.description = value.to_string(),
                "aliases" => {
                    // Переиспользуем разбор inline-списка из общего парсера frontmatter.
                    term.aliases = super::frontmatter::parse_list(attr, "aliases")
                        .unwrap_or_default()
                        .iter()
                        .map(|a| normalize_form(a))
                        .filter(|a| !a.is_empty())
                        .collect();
                }
                _ => {}
            }
        }
        if let Some(term) = current.take() {
            insert_term(&mut canonical, term);
        }

        let mut alias_to_canonical = HashMap::new();
        for term in canonical.values() {
            for alias in &term.aliases {
                // Алиас никогда не перекрывает канонический тег.
                if canonical.contains_key(alias) {
                    tracing::warn!(
                        "kb_vocabulary: алиас '{}' у тега '{}' совпадает с каноническим тегом — игнорируется",
                        alias,
                        term.tag
                    );
                    continue;
                }
                alias_to_canonical.insert(alias.clone(), term.tag.clone());
            }
        }

        Self {
            canonical,
            alias_to_canonical,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.canonical.is_empty()
    }

    pub fn len(&self) -> usize {
        self.canonical.len()
    }

    pub fn terms(&self) -> impl Iterator<Item = &Term> {
        self.canonical.values()
    }

    pub fn get(&self, canonical_tag: &str) -> Option<&Term> {
        self.canonical.get(canonical_tag)
    }

    /// Привести произвольный тег к каноническому. `None` — тега нет в словаре.
    pub fn normalize(&self, raw: &str) -> Option<&str> {
        let key = normalize_form(raw);
        if key.is_empty() {
            return None;
        }
        if let Some(term) = self.canonical.get(&key) {
            return Some(term.tag.as_str());
        }
        self.alias_to_canonical.get(&key).map(|s| s.as_str())
    }

    /// Подсказать теги по термам запроса: точное совпадение, затем общий префикс.
    /// Нужно, чтобы при нуле результатов модель получила путь входа, а не пустоту.
    pub fn suggest(&self, query_terms: &[String], limit: usize) -> Vec<&str> {
        let mut scored: Vec<(u32, &str)> = Vec::new();
        for term in self.canonical.values() {
            let mut score = 0u32;
            for q in query_terms {
                if q.is_empty() {
                    continue;
                }
                let hit = std::iter::once(&term.tag)
                    .chain(term.aliases.iter())
                    .any(|candidate| candidate.starts_with(q.as_str()) || q.starts_with(candidate));
                if hit {
                    score += 1;
                }
            }
            if score > 0 {
                scored.push((score, term.tag.as_str()));
            }
        }
        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(b.1)));
        scored.into_iter().take(limit).map(|(_, tag)| tag).collect()
    }
}

fn insert_term(canonical: &mut BTreeMap<String, Term>, term: Term) {
    if canonical.contains_key(&term.tag) {
        tracing::warn!("kb_vocabulary: тег '{}' объявлен дважды", term.tag);
    }
    canonical.insert(term.tag.clone(), term);
}

/// Единая форма тега: lowercase, `ё`→`е`, пробелы/подчёркивания → дефис.
pub fn normalize_form(raw: &str) -> String {
    raw.trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'ё' => 'е',
            ' ' | '_' => '-',
            other => other,
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"---
title: Словарь тегов
version: 1
---

# Словарь тегов

## воронка
- group: funnel
- label: Воронка продаж
- aliases: [funnel, sales-funnel, воронка-продаж]
- description: Путь товара показ → переход → корзина → заказ → выкуп.

## wildberries
- group: marketplace
- aliases: [wb, вб, вайлдберриз]

## drr
- group: metric
"#;

    #[test]
    fn parses_terms_with_and_without_optional_keys() {
        let v = Vocabulary::parse(SAMPLE);
        assert_eq!(v.len(), 3);
        let funnel = v.get("воронка").unwrap();
        assert_eq!(funnel.group, "funnel");
        assert_eq!(funnel.label, "Воронка продаж");
        assert!(funnel.description.contains("выкуп"));
        // wildberries без label/description — не должен ломать разбор.
        let wb = v.get("wildberries").unwrap();
        assert!(wb.label.is_empty());
        assert_eq!(wb.aliases.len(), 3);
        // drr вообще без алиасов.
        assert!(v.get("drr").unwrap().aliases.is_empty());
    }

    #[test]
    fn normalizes_aliases_and_canonical_forms() {
        let v = Vocabulary::parse(SAMPLE);
        assert_eq!(v.normalize("вб"), Some("wildberries"));
        assert_eq!(v.normalize("WB"), Some("wildberries"));
        assert_eq!(v.normalize("Воронка"), Some("воронка"));
        assert_eq!(v.normalize("sales_funnel"), Some("воронка"));
        assert_eq!(v.normalize("неизвестный-тег"), None);
    }

    #[test]
    fn yo_is_folded_to_ye() {
        // «вайлдберриз» пишут и через ё — обе формы должны сойтись.
        let v = Vocabulary::parse("## тест\n- aliases: [ёлка]\n");
        assert_eq!(v.normalize("елка"), Some("тест"));
        assert_eq!(v.normalize("ёлка"), Some("тест"));
    }

    #[test]
    fn missing_file_degrades_to_empty_vocabulary() {
        let v = Vocabulary::parse("");
        assert!(v.is_empty());
        assert_eq!(v.normalize("что-угодно"), None);
        assert!(v.suggest(&["воронк".to_string()], 5).is_empty());
    }

    #[test]
    fn malformed_section_does_not_abort_parse() {
        let raw = "## первый\n- group без двоеточия\nмусор\n## второй\n- group: g\n";
        let v = Vocabulary::parse(raw);
        assert_eq!(v.len(), 2);
        assert_eq!(v.get("второй").unwrap().group, "g");
    }

    #[test]
    fn suggest_ranks_by_matched_terms() {
        let v = Vocabulary::parse(SAMPLE);
        let s = v.suggest(&["воронк".to_string()], 5);
        assert_eq!(s.first(), Some(&"воронка"));
        // Полностью посторонний терм не даёт подсказок.
        assert!(v.suggest(&["zzzzz".to_string()], 5).is_empty());
    }

    #[test]
    fn heading_levels_other_than_two_are_ignored() {
        let v = Vocabulary::parse("# Заголовок\n## тег\n- group: g\n### подпункт\n");
        assert_eq!(v.len(), 1);
        assert!(v.get("тег").is_some());
    }
}
