//! Resources and JavaScript tasks bundled with reloadable skill packages.

use serde_json::{json, Value};
use std::collections::HashSet;

const MAX_RESULT_BYTES: usize = 1024 * 1024;

fn parse_arguments(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| json!({}))
}

fn active_skill(
    skill_id: &str,
    active_skill_ids: &HashSet<String>,
    snapshot: &super::skills::SkillRegistrySnapshot,
) -> Result<super::skills::Skill, Value> {
    if !active_skill_ids.contains(skill_id) {
        return Err(json!({
            "error": format!("Skill '{}' is not active in this conversation.", skill_id),
            "_ok": false,
        }));
    }
    super::skills::skill_by_id_in(snapshot, skill_id).ok_or_else(|| {
        json!({
            "error": format!("Skill '{}' is not present in the current catalog.", skill_id),
            "_ok": false,
        })
    })
}

pub fn list_resources(
    arguments: &str,
    active_skill_ids: &HashSet<String>,
    snapshot: &super::skills::SkillRegistrySnapshot,
) -> Value {
    let args = parse_arguments(arguments);
    let skill_id = args
        .get("skill_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let skill = match active_skill(skill_id, active_skill_ids, snapshot) {
        Ok(skill) => skill,
        Err(error) => return error,
    };
    json!({
        "skill_id": skill.id,
        "digest": skill.digest,
        "registry_generation": snapshot.generation,
        "resources": skill.resources.iter().map(|resource| json!({
            "path": resource.path,
            "kind": resource.kind,
        })).collect::<Vec<_>>(),
        "tasks": skill.tasks.iter().map(|task| json!({
            "id": task.id,
            "title": task.title,
            "runtime": task.runtime,
            "mode": task.mode,
            "input_schema": task.input_schema,
        })).collect::<Vec<_>>(),
        "_ok": true,
    })
}

pub fn read_resource(
    arguments: &str,
    active_skill_ids: &HashSet<String>,
    snapshot: &super::skills::SkillRegistrySnapshot,
) -> Value {
    let args = parse_arguments(arguments);
    let skill_id = args
        .get("skill_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let relative = args.get("path").and_then(Value::as_str).unwrap_or_default();
    let skill = match active_skill(skill_id, active_skill_ids, snapshot) {
        Ok(skill) => skill,
        Err(error) => return error,
    };
    let Some(resource) = skill
        .resources
        .iter()
        .find(|resource| resource.path == relative)
    else {
        return json!({
            "error": format!("Resource '{}' is not declared by skill '{}'.", relative, skill.id),
            "_ok": false,
        });
    };
    let offset = args.get("offset").and_then(Value::as_u64).unwrap_or(0) as usize;
    let max_chars = args
        .get("max_chars")
        .and_then(Value::as_u64)
        .unwrap_or(16_000)
        .clamp(1, 32_000) as usize;
    let total_chars = resource.content.chars().count();
    let content = resource
        .content
        .chars()
        .skip(offset)
        .take(max_chars)
        .collect::<String>();
    let consumed = content.chars().count();
    let next_offset = offset.saturating_add(consumed);
    let truncated = next_offset < total_chars;
    json!({
        "skill_id": skill.id,
        "digest": skill.digest,
        "registry_generation": snapshot.generation,
        "path": relative,
        "content": content,
        "offset": offset,
        "next_offset": truncated.then_some(next_offset),
        "truncated": truncated,
        "total_chars": total_chars,
        "_ok": true,
    })
}

fn validate_input_schema(schema: Option<&Value>, input: &Value) -> Result<(), String> {
    let Some(schema) = schema else {
        return Ok(());
    };
    let validator = jsonschema::validator_for(schema)
        .map_err(|error| format!("invalid JSON Schema: {error}"))?;
    validator.validate(input).map_err(|error| error.to_string())
}

pub async fn run_task(
    arguments: &str,
    active_skill_ids: &HashSet<String>,
    snapshot: &super::skills::SkillRegistrySnapshot,
    execute_allowed: bool,
    develop_allowed: bool,
) -> Value {
    let args = parse_arguments(arguments);
    let skill_id = args
        .get("skill_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let task_id = args
        .get("task_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let skill = match active_skill(skill_id, active_skill_ids, snapshot) {
        Ok(skill) => skill,
        Err(error) => return error,
    };
    let Some(task) = skill.tasks.iter().find(|task| task.id == task_id) else {
        return json!({
            "error": format!("Task '{}' is not declared by skill '{}'.", task_id, skill.id),
            "_ok": false,
        });
    };
    let permitted = if task.mode == "stable" {
        execute_allowed
    } else {
        develop_allowed
    };
    if !permitted {
        let capability = if task.mode == "stable" {
            super::skill_policy::SKILL_SCRIPT_EXECUTE
        } else {
            super::skill_policy::SKILL_SCRIPT_DEVELOP
        };
        return json!({
            "error": format!("Specialization does not have capability '{}'.", capability),
            "_ok": false,
        });
    }
    let input = args.get("input").cloned().unwrap_or(Value::Null);
    if let Err(error) = validate_input_schema(task.input_schema.as_ref(), &input) {
        return json!({ "error": format!("Invalid task input: {error}"), "_ok": false });
    }
    match crate::plugins::engine::invoke_ephemeral_server_script(
        task.source.clone(),
        task.export.clone(),
        input,
        task.capabilities.clone(),
    )
    .await
    {
        Ok((result, logs)) => {
            if serde_json::to_vec(&result)
                .map(|bytes| bytes.len())
                .unwrap_or(MAX_RESULT_BYTES + 1)
                > MAX_RESULT_BYTES
            {
                return json!({ "error": "Task result exceeds the 1 MiB limit.", "_ok": false });
            }
            json!({
                "skill_id": skill.id,
                "task_id": task.id,
                "mode": task.mode,
                "digest": skill.digest,
                "registry_generation": snapshot.generation,
                "result": result,
                "logs": logs,
                "_ok": true,
            })
        }
        Err(error) => json!({
            "error": error.to_string(),
            "skill_id": skill.id,
            "task_id": task.id,
            "digest": skill.digest,
            "registry_generation": snapshot.generation,
            "_ok": false,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn json_schema_checks_required_fields_and_types() {
        let schema = json!({ "type": "object", "required": ["rows"] });
        assert!(validate_input_schema(Some(&schema), &json!({ "rows": [] })).is_ok());
        assert!(validate_input_schema(Some(&schema), &json!({})).is_err());
        let typed = json!({
            "type": "object",
            "properties": { "count": { "type": "integer", "minimum": 1 } },
            "required": ["count"]
        });
        assert!(validate_input_schema(Some(&typed), &json!({"count": 2})).is_ok());
        assert!(validate_input_schema(Some(&typed), &json!({"count": 0})).is_err());
    }

    fn funnel_snapshot() -> super::super::skills::SkillRegistrySnapshot {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("skills")
            .join("marketplace-funnel-analysis");
        let skill = super::super::skills::load_package_for_test(&root);
        super::super::skills::SkillRegistrySnapshot {
            generation: 42,
            catalog_digest: "catalog-test".into(),
            loaded_at: "test".into(),
            skills: Arc::new(vec![skill]),
            diagnostics: Vec::new(),
        }
    }

    async fn run_funnel(task_id: &str, input: Value) -> Value {
        let snapshot = funnel_snapshot();
        let active = HashSet::from(["marketplace-funnel-analysis".to_string()]);
        run_task(
            &json!({
                "skill_id": "marketplace-funnel-analysis",
                "task_id": task_id,
                "input": input,
            })
            .to_string(),
            &active,
            &snapshot,
            true,
            false,
        )
        .await
    }

    #[tokio::test]
    async fn funnel_package_runs_all_declared_tasks() {
        let metrics = json!({
            "impressions": 1000,
            "clicks": 200,
            "product_views": 180,
            "cart_adds": 60,
            "orders": 30,
            "deliveries": 25,
            "revenue": 50000
        });
        let calculate = run_funnel(
            "calculate-funnel",
            json!({"marketplace": "ozon", "label": "current", "metrics": metrics}),
        )
        .await;
        assert_eq!(calculate["_ok"], true);
        assert_eq!(calculate["registry_generation"], 42);
        assert_eq!(calculate["result"]["stages"].as_array().unwrap().len(), 6);

        let validate = run_funnel(
            "validate-funnel",
            json!({"marketplace": "ozon", "metrics": {"orders": 10, "deliveries": 8}}),
        )
        .await;
        assert_eq!(validate["result"]["valid"], true);

        let periods = run_funnel(
            "compare-periods",
            json!({
                "marketplace": "ozon",
                "current": {"label": "2026-07", "metrics": {"orders": 20}},
                "previous": {"label": "2026-06", "metrics": {"orders": 10}}
            }),
        )
        .await;
        assert_eq!(periods["result"]["metrics"]["orders"]["relative"], 1.0);

        let segments = run_funnel(
            "compare-segments",
            json!({
                "marketplace": "wildberries",
                "segments": [
                    {"segment": "A", "metrics": {"orders": 20}},
                    {"segment": "B", "metrics": {"orders": 10}}
                ]
            }),
        )
        .await;
        assert_eq!(segments["result"]["ranking_by_orders"][0], "A");
    }

    #[test]
    fn resource_reading_is_paged_and_bound_to_snapshot() {
        let snapshot = funnel_snapshot();
        let active = HashSet::from(["marketplace-funnel-analysis".to_string()]);
        let first = read_resource(
            &json!({
                "skill_id": "marketplace-funnel-analysis",
                "path": "references/funnel-model.md",
                "offset": 0,
                "max_chars": 20
            })
            .to_string(),
            &active,
            &snapshot,
        );
        assert_eq!(first["_ok"], true);
        assert_eq!(first["registry_generation"], 42);
        assert_eq!(first["content"].as_str().unwrap().chars().count(), 20);
        assert_eq!(first["truncated"], true);
    }
}
