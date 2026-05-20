use axum::{extract::Path, http::StatusCode, Json};
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path as FsPath, PathBuf};

use crate::shared::llm::knowledge_base::{knowledge_base_dir, KnowledgeDoc, KNOWLEDGE_BASE};

// Compile-time metadata for known DataView modules — used to enrich tree segment names.
static DV_LABELS: Lazy<HashMap<String, String>> = Lazy::new(|| {
    let entries: &[(&str, &str)] = &[
        ("dv001", include_str!("../../data_view/dv001/metadata.json")),
        ("dv004", include_str!("../../data_view/dv004/metadata.json")),
    ];
    entries
        .iter()
        .filter_map(|(id, json_str)| {
            let v: serde_json::Value = serde_json::from_str(json_str).ok()?;
            let name = v.get("name")?.as_str()?;
            Some((id.to_string(), format!("{} {}", id, name)))
        })
        .collect()
});

#[derive(Debug, Clone, Serialize)]
pub struct KbArticleSummary {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub related: Vec<String>,
    pub source_path: Option<String>,
    pub display_path: String,
    pub is_embedded: bool,
}

#[derive(Debug, Serialize)]
pub struct KbArticleDetail {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
    pub related: Vec<String>,
    pub source_path: Option<String>,
    pub display_path: String,
    pub is_embedded: bool,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct KbStatsResponse {
    pub total_articles: usize,
    pub file_articles: usize,
    pub embedded_articles: usize,
    pub total_tags: usize,
    pub total_related: usize,
    pub total_folders: usize,
    pub knowledge_base_path: String,
    pub top_tags: Vec<KbCountItem>,
}

#[derive(Debug, Serialize)]
pub struct KbCountItem {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct KbTreeResponse {
    pub roots: Vec<KbTreeNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KbTreeNode {
    pub name: String,
    pub path: String,
    pub node_type: String,
    pub article: Option<KbArticleSummary>,
    pub children: Vec<KbTreeNode>,
}

#[derive(Debug, Default)]
struct MutableTreeNode {
    name: String,
    path: String,
    article: Option<KbArticleSummary>,
    children: BTreeMap<String, MutableTreeNode>,
}

pub async fn stats() -> Json<KbStatsResponse> {
    let kb_dir = knowledge_base_dir();
    let kb = KNOWLEDGE_BASE.read().expect("KnowledgeBase lock poisoned");
    let docs = kb.all_docs();
    let mut tags = BTreeMap::<String, usize>::new();
    let mut related = BTreeSet::<String>::new();
    let mut folders = BTreeSet::<String>::new();
    let mut file_articles = 0usize;
    let mut embedded_articles = 0usize;

    for doc in docs.iter() {
        let summary = article_summary(doc, &kb_dir);
        if summary.is_embedded {
            embedded_articles += 1;
        } else {
            file_articles += 1;
        }
        for tag in &summary.tags {
            *tags.entry(tag.clone()).or_insert(0) += 1;
        }
        for item in &summary.related {
            related.insert(item.clone());
        }
        let segments = path_segments(&summary.display_path);
        for depth in 1..segments.len() {
            folders.insert(segments[..depth].join("/"));
        }
    }

    let total_tags = tags.len();
    let mut top_tags = tags
        .into_iter()
        .map(|(name, count)| KbCountItem { name, count })
        .collect::<Vec<_>>();
    top_tags.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.name.cmp(&b.name)));
    top_tags.truncate(20);

    Json(KbStatsResponse {
        total_articles: docs.len(),
        file_articles,
        embedded_articles,
        total_tags,
        total_related: related.len(),
        total_folders: folders.len(),
        knowledge_base_path: kb_dir.display().to_string(),
        top_tags,
    })
}

pub async fn tree() -> Json<KbTreeResponse> {
    let kb_dir = knowledge_base_dir();
    let kb = KNOWLEDGE_BASE.read().expect("KnowledgeBase lock poisoned");
    let mut root = MutableTreeNode::default();

    let mut docs = kb.all_docs();
    docs.sort_by(|a, b| a.id.cmp(&b.id));
    for doc in docs {
        insert_article(&mut root, article_summary(doc, &kb_dir));
    }

    Json(KbTreeResponse {
        roots: root
            .children
            .into_values()
            .map(MutableTreeNode::into_tree_node)
            .collect(),
    })
}

pub async fn get_article(Path(id): Path<String>) -> Result<Json<KbArticleDetail>, StatusCode> {
    let kb_dir = knowledge_base_dir();
    let kb = KNOWLEDGE_BASE.read().expect("KnowledgeBase lock poisoned");
    let Some(doc) = kb.get(&id) else {
        return Err(StatusCode::NOT_FOUND);
    };
    let summary = article_summary(doc, &kb_dir);
    Ok(Json(KbArticleDetail {
        id: summary.id,
        title: summary.title,
        tags: summary.tags,
        related: summary.related,
        source_path: summary.source_path,
        display_path: summary.display_path,
        is_embedded: summary.is_embedded,
        content: doc.content.clone(),
    }))
}

impl MutableTreeNode {
    fn folder(name: String, path: String) -> Self {
        Self {
            name,
            path,
            article: None,
            children: BTreeMap::new(),
        }
    }

    fn into_tree_node(self) -> KbTreeNode {
        KbTreeNode {
            name: self.name,
            path: self.path,
            node_type: if self.article.is_some() {
                "article".to_string()
            } else {
                "folder".to_string()
            },
            article: self.article,
            children: self
                .children
                .into_values()
                .map(MutableTreeNode::into_tree_node)
                .collect(),
        }
    }
}

fn insert_article(root: &mut MutableTreeNode, article: KbArticleSummary) {
    let mut segments = path_segments(&article.display_path);
    if segments.is_empty() {
        segments.push(article.id.clone());
    }

    let mut current = root;
    let mut path_acc = Vec::<String>::new();
    for segment in segments.iter().take(segments.len().saturating_sub(1)) {
        path_acc.push(segment.clone());
        let path = path_acc.join("/");
        // Enrich known DataView folder segments with their human-readable name.
        let display_name = DV_LABELS
            .get(segment.as_str())
            .cloned()
            .unwrap_or_else(|| segment.clone());
        current = current
            .children
            .entry(segment.clone())
            .or_insert_with(|| MutableTreeNode::folder(display_name, path));
    }

    // Use the article title as the leaf node display name for embedded docs;
    // for Obsidian files keep the raw segment (matches the filename).
    let key = segments
        .last()
        .cloned()
        .unwrap_or_else(|| article.id.clone());
    let leaf_name = if article.is_embedded {
        article.title.clone()
    } else {
        key.clone()
    };
    let path = segments.join("/");
    current.children.insert(
        key,
        MutableTreeNode {
            name: leaf_name,
            path,
            article: Some(article),
            children: BTreeMap::new(),
        },
    );
}

fn article_summary(doc: &KnowledgeDoc, kb_dir: &FsPath) -> KbArticleSummary {
    let source_path = doc.source_path.clone();
    let display_path = source_path
        .as_deref()
        .map(|path| display_path(path, kb_dir))
        .unwrap_or_else(|| format!("embedded/{}.md", doc.id));
    let is_embedded = source_path
        .as_deref()
        .map(|path| !is_under_kb_dir(path, kb_dir))
        .unwrap_or(true);

    KbArticleSummary {
        id: doc.id.clone(),
        title: doc.title.clone(),
        tags: doc.tags.clone(),
        related: doc.related.clone(),
        source_path,
        display_path,
        is_embedded,
    }
}

/// Returns a user-friendly display path.
///
/// - Obsidian business files: path relative to kb_dir.
/// - Embedded app-docs: strip leading `crates/backend/src/` (first 3 segments) so the tree
///   starts directly from `domain/`, `data_view/`, etc.
fn display_path(source_path: &str, kb_dir: &FsPath) -> String {
    let path = PathBuf::from(source_path);
    if let Ok(relative) = path.strip_prefix(kb_dir) {
        return normalize_path(relative.display().to_string());
    }
    let normalized = normalize_path(source_path.to_string());
    // Strip the first 3 segments (crates/backend/src) for embedded app docs.
    let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() > 3 {
        segments[3..].join("/")
    } else {
        normalized
    }
}

fn is_under_kb_dir(source_path: &str, kb_dir: &FsPath) -> bool {
    PathBuf::from(source_path).strip_prefix(kb_dir).is_ok()
}

fn normalize_path(path: String) -> String {
    path.replace('\\', "/")
}

fn path_segments(path: &str) -> Vec<String> {
    normalize_path(path.to_string())
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
