use contracts::plugins::{PluginBundle, PluginDefinition, PluginHealth, PluginRunContext};
use leptos::prelude::*;
use std::collections::HashMap;

pub(super) fn commit_selected_sql(
    resources: RwSignal<HashMap<String, String>>,
    selected_name: RwSignal<Option<String>>,
    sql_source: RwSignal<String>,
) {
    if let Some(name) = selected_name.get_untracked() {
        resources.update(|items| {
            items.insert(name, sql_source.get_untracked());
        });
    }
}

pub(super) fn sorted_resource_names(resources: &HashMap<String, String>) -> Vec<String> {
    let mut names = resources.keys().cloned().collect::<Vec<_>>();
    names.sort();
    names
}

pub(super) fn first_sql_resource(resources: &HashMap<String, String>) -> (Option<String>, String) {
    let name = sorted_resource_names(resources).into_iter().next();
    let source = name
        .as_ref()
        .and_then(|item| resources.get(item))
        .cloned()
        .unwrap_or_default();
    (name, source)
}

pub(crate) fn default_run_context(bundle: &PluginBundle) -> PluginRunContext {
    let params = bundle
        .params
        .iter()
        .filter_map(|param| {
            param
                .default_value
                .as_ref()
                .map(|value| (param.key.clone(), value.clone()))
        })
        .collect();

    PluginRunContext {
        date_from: None,
        date_to: None,
        connection_mp_refs: Vec::new(),
        group_by: None,
        params,
    }
}

pub(super) fn pretty_context(context: &PluginRunContext) -> String {
    serde_json::to_string_pretty(context).unwrap_or_else(|_| "{}".to_string())
}

pub(super) fn parse_context(source: &str) -> Result<PluginRunContext, String> {
    if source.trim().is_empty() {
        return Ok(PluginRunContext::default());
    }
    serde_json::from_str(source).map_err(|error| format!("Invalid context JSON: {error}"))
}

pub(super) fn build_current_bundle(
    def: ReadSignal<Option<PluginDefinition>>,
    client_src: RwSignal<String>,
    server_src: RwSignal<String>,
    styles_src: RwSignal<String>,
    sql_resources: RwSignal<HashMap<String, String>>,
    selected_sql_name: RwSignal<Option<String>>,
    sql_src: RwSignal<String>,
) -> Option<PluginBundle> {
    commit_selected_sql(sql_resources, selected_sql_name, sql_src);
    let current = def.get_untracked()?;
    let mut bundle = current.bundle.clone();
    bundle.client_script = Some(client_src.get_untracked());
    bundle.server_script = Some(server_src.get_untracked());
    bundle.styles = Some(styles_src.get_untracked());
    bundle.sql_resources = sql_resources.get_untracked();
    Some(bundle)
}

pub(super) fn health_badge(health: PluginHealth) -> (&'static str, &'static str) {
    match health {
        PluginHealth::Ok => ("OK", "ok"),
        PluginHealth::Warn => ("Warning", "warn"),
        PluginHealth::Crit => ("Critical", "crit"),
        PluginHealth::NoData => ("No data", "nodata"),
    }
}

pub(super) fn format_invoke_body(body: &serde_json::Value) -> String {
    let mut out = String::new();

    if body.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
        let result = body
            .get("result")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        out.push_str(&format!(
            "Result:\n{}",
            serde_json::to_string_pretty(&result).unwrap_or_default()
        ));
    } else {
        if let Some(err) = body.get("error").and_then(|v| v.as_str()) {
            out.push_str(&format!("{err}\n"));
        }
        if let Some(detail) = body.get("error_detail").filter(|d| !d.is_null()) {
            let stage = detail.get("stage").and_then(|v| v.as_str()).unwrap_or("");
            let message = detail.get("message").and_then(|v| v.as_str()).unwrap_or("");
            out.push_str(&format!("\nstage: {stage}\n{message}\n"));
            if let Some(stack) = detail.get("stack").and_then(|v| v.as_str()) {
                out.push_str(&format!("\n{stack}\n"));
            }
        }
        if out.is_empty() {
            out = "Unknown error".to_string();
        }
    }

    if let Some(logs) = body.get("logs").and_then(|v| v.as_array()) {
        let lines: Vec<String> = logs
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        if !lines.is_empty() {
            out.push_str(&format!("\n-- log --\n{}", lines.join("\n")));
        }
    }
    out
}
