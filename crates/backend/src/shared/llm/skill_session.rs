//! Per-message orchestration state for specialization-aware skills.

use super::skill_policy::{self, SkillAccessLevel};
use super::skills::{self, Skill, SkillRegistrySnapshot};
use super::types::ToolDefinition;
use contracts::domain::a017_llm_agent::aggregate::AgentType;
pub use contracts::domain::a018_llm_chat::aggregate::SkillTraceEntry;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub fn activation_tool_trace(trace: &SkillTraceEntry, title: &str) -> Value {
    json!({
        "iteration": 0,
        "call": 0,
        "stage": "skill_selection",
        "tool": "use_skill",
        "ok": true,
        "ms": 0,
        "summary": format!("Активирован навык {title}"),
        "input": { "id": trace.skill_id, "source": trace.source },
        "output": {
            "_activate_skill": trace.skill_id,
            "_skill_access_level": trace.access_level,
            "_skill_inefficiency": trace.inefficient,
            "_skill_digest": trace.skill_digest,
            "_registry_generation": trace.registry_generation,
            "source": trace.source,
        }
    })
}

#[derive(Debug, Clone)]
pub struct SkillSession {
    specialization: AgentType,
    snapshot: Arc<SkillRegistrySnapshot>,
    access: HashMap<String, SkillAccessLevel>,
    artifact_publish_allowed: bool,
    script_execute_allowed: bool,
    script_develop_allowed: bool,
    active_skill_ids: Vec<String>,
}

impl SkillSession {
    pub async fn load(specialization: AgentType) -> anyhow::Result<Self> {
        let snapshot = skills::snapshot();
        let access = skill_policy::accesses_for_snapshot(&specialization, &snapshot).await?;
        let artifact_publish_allowed =
            skill_policy::capability_allowed(&specialization, skill_policy::ARTIFACT_PUBLISH)
                .await?;
        let script_execute_allowed =
            skill_policy::capability_allowed(&specialization, skill_policy::SKILL_SCRIPT_EXECUTE)
                .await?;
        let script_develop_allowed =
            skill_policy::capability_allowed(&specialization, skill_policy::SKILL_SCRIPT_DEVELOP)
                .await?;
        Ok(Self {
            specialization,
            snapshot,
            access,
            artifact_publish_allowed,
            script_execute_allowed,
            script_develop_allowed,
            active_skill_ids: Vec::new(),
        })
    }

    pub fn snapshot(&self) -> &Arc<SkillRegistrySnapshot> {
        &self.snapshot
    }

    pub fn access(&self) -> &HashMap<String, SkillAccessLevel> {
        &self.access
    }

    pub fn artifact_publish_allowed(&self) -> bool {
        self.artifact_publish_allowed
    }

    pub fn script_execute_allowed(&self) -> bool {
        self.script_execute_allowed
    }

    pub fn script_develop_allowed(&self) -> bool {
        self.script_develop_allowed
    }

    pub fn active_skill_ids(&self) -> &[String] {
        &self.active_skill_ids
    }

    pub fn active_skill_set(&self) -> HashSet<String> {
        self.active_skill_ids.iter().cloned().collect()
    }

    pub fn skill(&self, id: &str) -> Option<Skill> {
        skills::skill_by_id_in(&self.snapshot, id)
    }

    pub fn skill_for_intent(&self, intent: &str) -> Option<Skill> {
        skills::skill_for_intent_in(&self.snapshot, intent)
    }

    pub fn access_level(&self, skill_id: &str) -> SkillAccessLevel {
        self.access
            .get(skill_id)
            .copied()
            .unwrap_or(SkillAccessLevel::Denied)
    }

    pub fn select_initial(
        &mut self,
        intent: &str,
        confidence: f64,
        previous_skill_id: Option<&str>,
    ) -> Option<(Skill, SkillTraceEntry)> {
        let intent_skill = self.skill_for_intent(intent);
        let candidate = if confidence <= 0.25 {
            previous_skill_id
                .and_then(|id| self.skill(id))
                .map(|skill| (skill, "follow_up"))
                .or_else(|| intent_skill.map(|skill| (skill, "selector")))
        } else {
            intent_skill.map(|skill| (skill, "selector"))
        }?;
        self.activate(&candidate.0.id, candidate.1)
            .map(|trace| (candidate.0, trace))
    }

    pub fn activate(&mut self, skill_id: &str, source: &str) -> Option<SkillTraceEntry> {
        let skill = self.skill(skill_id)?;
        let level = self.access_level(skill_id);
        if level == SkillAccessLevel::Denied {
            return None;
        }
        if !self.active_skill_ids.iter().any(|id| id == skill_id) {
            self.active_skill_ids.push(skill_id.to_string());
        }
        Some(SkillTraceEntry {
            skill_id: skill.id,
            skill_digest: skill.digest,
            registry_generation: self.snapshot.generation,
            access_level: level.as_str().to_string(),
            source: source.to_string(),
            inefficient: level == SkillAccessLevel::Extended,
        })
    }

    pub fn immediate_catalog_prompt(&self) -> String {
        let mut text = String::new();
        let immediate = self
            .snapshot
            .skills
            .iter()
            .filter(|skill| self.access_level(&skill.id) == SkillAccessLevel::Immediate)
            .collect::<Vec<_>>();
        if !immediate.is_empty() {
            text.push_str(
                "\n\n---\nОсновные навыки специализации (при необходимости активируй через use_skill):\n",
            );
            for skill in immediate {
                text.push_str(&format!("- {} — {}\n", skill.id, skill.description));
            }
        }
        text
    }

    pub fn active_prompts(&self) -> String {
        let mut text = String::new();
        for id in &self.active_skill_ids {
            if let Some(skill) = self.skill(id) {
                text.push_str("\n\n---\n");
                text.push_str(&skills::activation_prompt(&skill));
            }
        }
        text
    }

    /// Tool schemas advertised to the model. Builder controllers may narrow
    /// this list to one deterministic next step.
    pub fn advertised_tools(&self, next_tool: Option<&str>) -> Vec<ToolDefinition> {
        let mut tools = skills::assemble_tools_in(&self.snapshot, &self.active_skill_ids);
        if !self.artifact_publish_allowed {
            tools.retain(|tool| !skill_policy::is_artifact_mutation(&tool.name));
        }
        if let Some(next) = next_tool {
            tools.retain(|tool| tool.name == next);
        }
        tools
    }

    /// Runtime authorization always retains core tools, even when the model
    /// currently sees a deliberately narrow builder workflow.
    pub fn executable_tools(&self) -> HashSet<String> {
        skills::active_tool_names_in(&self.snapshot, &self.active_skill_ids)
    }

    pub fn list_skills_result(&self) -> Value {
        skills::list_skills_result_from(&self.snapshot, &self.access)
    }

    pub fn use_skill_result(&self, arguments: &str) -> Value {
        skills::use_skill_result_from(arguments, &self.snapshot, &self.access)
    }

    pub fn specialization(&self) -> &AgentType {
        &self.specialization
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_serializes_registry_identity() {
        let trace = SkillTraceEntry {
            skill_id: "sample".into(),
            skill_digest: "abc".into(),
            registry_generation: 7,
            access_level: "extended".into(),
            source: "model".into(),
            inefficient: true,
        };
        let value = serde_json::to_value(trace).unwrap();
        assert_eq!(value["registry_generation"], 7);
        assert_eq!(value["skill_digest"], "abc");
    }

    fn session_with_access(access: HashMap<String, SkillAccessLevel>) -> SkillSession {
        SkillSession {
            specialization: AgentType::SalesAnalyst,
            snapshot: skills::snapshot(),
            access,
            artifact_publish_allowed: true,
            script_execute_allowed: true,
            script_develop_allowed: false,
            active_skill_ids: Vec::new(),
        }
    }

    #[test]
    fn activation_respects_access_and_marks_extended() {
        let mut session = session_with_access(HashMap::from([
            ("data-analytics".into(), SkillAccessLevel::Denied),
            ("chart-builder".into(), SkillAccessLevel::Extended),
        ]));
        assert!(session.activate("data-analytics", "model").is_none());
        let trace = session.activate("chart-builder", "selector").unwrap();
        assert!(trace.inefficient);
        assert_eq!(trace.source, "selector");
    }

    #[test]
    fn builder_advertisement_does_not_remove_core_runtime_tools() {
        let mut session = session_with_access(HashMap::from([(
            "chart-builder".into(),
            SkillAccessLevel::Immediate,
        )]));
        session.activate("chart-builder", "selector").unwrap();
        let advertised = session
            .advertised_tools(None)
            .into_iter()
            .map(|tool| tool.name)
            .collect::<HashSet<_>>();
        assert_eq!(
            advertised,
            HashSet::from([
                "find_data_sources".to_string(),
                "preview_data".to_string(),
                "build_chart".to_string(),
            ])
        );
        let executable = session.executable_tools();
        assert!(executable.contains("use_skill"));
        assert!(executable.contains("read_skill_resource"));
        assert!(executable.contains("build_chart"));
    }
}
