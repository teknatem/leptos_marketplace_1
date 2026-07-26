//! Deterministic controller for declarative chart/table publication.

use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuilderKind {
    Chart,
    Table,
}

#[derive(Debug, Clone)]
pub struct BuilderWorkflow {
    kind: BuilderKind,
    next_tool: Option<&'static str>,
}

impl BuilderWorkflow {
    pub fn from_active_skills<S: AsRef<str>>(
        active_skills: &[S],
        publishing_allowed: bool,
    ) -> Option<Self> {
        if !publishing_allowed || active_skills.len() != 1 {
            return None;
        }
        let kind = match active_skills[0].as_ref() {
            "chart-builder" => BuilderKind::Chart,
            "table-builder" => BuilderKind::Table,
            _ => return None,
        };
        Some(Self {
            kind,
            next_tool: Some("find_data_sources"),
        })
    }

    pub fn next_tool(&self) -> Option<&'static str> {
        self.next_tool
    }

    pub fn observe(&mut self, tool: &str, is_ok: bool, result: Option<&Value>) {
        self.next_tool = match tool {
            "find_data_sources" if is_ok => Some("preview_data"),
            "find_data_sources" => Some("find_data_sources"),
            "preview_data"
                if is_ok
                    && (self.kind != BuilderKind::Chart
                        || result
                            .and_then(|v| v.get("build_ready"))
                            .and_then(Value::as_bool)
                            == Some(true)) =>
            {
                Some(match self.kind {
                    BuilderKind::Chart => "build_chart",
                    BuilderKind::Table => "build_table",
                })
            }
            "preview_data" => Some("preview_data"),
            "build_chart" | "build_table" if is_ok => None,
            "build_chart" | "build_table" => Some("preview_data"),
            _ => self.next_tool,
        };
    }

    pub fn completion_label(&self, artifact_created: bool) -> &'static str {
        match (self.kind, artifact_created) {
            (BuilderKind::Chart, true) => "График создан",
            (BuilderKind::Chart, false) => "График не создан",
            (BuilderKind::Table, true) => "Таблица создана",
            (BuilderKind::Table, false) => "Таблица не создана",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_requires_build_ready_before_publish() {
        let mut flow = BuilderWorkflow::from_active_skills(&["chart-builder"], true).unwrap();
        flow.observe("find_data_sources", true, None);
        assert_eq!(flow.next_tool(), Some("preview_data"));
        flow.observe(
            "preview_data",
            true,
            Some(&serde_json::json!({"build_ready": false})),
        );
        assert_eq!(flow.next_tool(), Some("preview_data"));
        flow.observe(
            "preview_data",
            true,
            Some(&serde_json::json!({"build_ready": true})),
        );
        assert_eq!(flow.next_tool(), Some("build_chart"));
    }

    #[test]
    fn publishing_capability_is_required() {
        assert!(BuilderWorkflow::from_active_skills(&["table-builder"], false).is_none());
    }
}
