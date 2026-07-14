//! Реестр Skills (capability-пакетов) для LLM-чата.
//!
//! Skill = id + описание + промпт-фрагмент + набор инструментов (по именам).
//! Заменяет статичный выбор инструментов по роли агента: набор инструментов и
//! системный промпт собираются из **активных** навыков (progressive disclosure).
//!
//! - `core` — всегда активен (понять систему + мета-инструменты list_skills/use_skill);
//! - бандлы (напр. `b_sql`) переиспользуются несколькими навыками;
//! - `agent_type` сохраняется как default-bias + allow-list (граница безопасности `use_skill`).

use super::types::ToolDefinition;
use contracts::domain::a017_llm_agent::aggregate::AgentType;
use serde_json::{json, Value};
use std::collections::HashSet;

// ─── Промпты ─────────────────────────────────────────────────────────────────

const CORE_PROMPT: &str = include_str!("../../domain/a018_llm_chat/prompts/core.md");
const PROMPT_DATA_ANALYTICS: &str =
    include_str!("../../domain/a018_llm_chat/prompts/skill_data_analytics.md");
const PROMPT_BI_AUTHORING: &str =
    include_str!("../../domain/a018_llm_chat/prompts/skill_bi_authoring.md");
const PROMPT_PLUGIN: &str =
    include_str!("../../domain/a018_llm_chat/prompts/plugin_admin_agent.md");
const PROMPT_CHART: &str =
    include_str!("../../domain/a018_llm_chat/prompts/skill_chart_builder.md");
const PROMPT_TABLE: &str =
    include_str!("../../domain/a018_llm_chat/prompts/skill_table_builder.md");
const PROMPT_SYS_ADMIN: &str =
    include_str!("../../domain/a018_llm_chat/prompts/system_admin_agent.md");
const PROMPT_KB: &str = include_str!("../../domain/a018_llm_chat/prompts/kb_admin_analyze.md");
const PROMPT_MAILBOX: &str =
    include_str!("../../domain/a018_llm_chat/prompts/skill_mailbox.md");

// ─── Core: всегда активные инструменты ───────────────────────────────────────

/// Базовый набор (понять систему) + мета-инструменты навыков. Всегда активен.
const CORE_TOOLS: &[&str] = &[
    "get_architecture_overview",
    "get_entity_schema",
    "search_knowledge",
    "get_knowledge",
    "list_skills",
    "use_skill",
];

// Бандл интроспекции БД (`list_entities`, `get_join_hint`, `execute_query`) переиспользуется
// несколькими навыками — имена перечислены в их `tool_names` (дедуп при сборке).

// ─── Реестр навыков ──────────────────────────────────────────────────────────

pub struct Skill {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// Router-интенты, ведущие к этому навыку.
    pub intents: &'static [&'static str],
    pub prompt: &'static str,
    /// Имена инструментов навыка (резолвятся в определения из «вселенной» tool'ов).
    pub tool_names: &'static [&'static str],
}

pub static SKILLS: &[Skill] = &[
    Skill {
        id: "data-analytics",
        title: "Аналитика данных",
        description: "SQL-аналитика по маркетплейсам: продажи, заказы, остатки, выручка, \
                      обороты GL, поиск UUID в справочниках.",
        intents: &["data_query"],
        prompt: PROMPT_DATA_ANALYTICS,
        tool_names: &[
            "list_entities",
            "get_join_hint",
            "list_data_sources",
            "query_data_schema",
            "run_data_view_scalar",
            "run_data_view_drilldown",
            "execute_query",
            "get_chart_of_accounts",
            "list_gl_turnovers",
        ],
    },
    Skill {
        id: "bi-authoring",
        title: "BI и drilldown-отчёты",
        description:
            "Создание drilldown-отчётов и работа с BI-индикаторами/дашбордами (a024/a025), \
                      DataView, метриками и измерениями.",
        intents: &["bi_authoring"],
        prompt: PROMPT_BI_AUTHORING,
        tool_names: &[
            "list_entities",
            "get_join_hint",
            "list_data_sources",
            "run_data_view_scalar",
            "run_data_view_drilldown",
            "execute_query",
            "create_drilldown_report",
        ],
    },
    Skill {
        id: "plugin-authoring",
        title: "Разработка плагинов",
        description:
            "Создание/доработка/тест JS-плагинов (client+server) из чата: шаблоны, примеры, \
                      валидация, upsert, invoke, журнал запусков.",
        intents: &["plugin_dev"],
        prompt: PROMPT_PLUGIN,
        tool_names: &[
            "list_entities",
            "get_join_hint",
            "list_data_sources",
            "query_data_schema",
            "run_data_view_drilldown",
            "execute_query",
            "plugin_list",
            "plugin_get",
            "plugin_validate",
            "plugin_smoke_test",
            "plugin_upsert",
            "plugin_invoke",
            "plugin_template",
            "plugin_examples",
            "get_plugin_ui_contract",
            "plugin_data_catalog",
            "plugin_runs",
            "chart_template",
            "chart_examples",
            "get_chart_ui_contract",
            "table_template",
            "table_examples",
            "get_table_ui_contract",
        ],
    },
    Skill {
        id: "chart-builder",
        title: "Графики и диаграммы",
        description:
            "Построение графиков из данных: пользователь описывает, что показать — агент собирает \
                      SELECT, выбирает тип (линия/столбцы/доли) и публикует график-плагин (Chart.js).",
        intents: &["chart_build"],
        prompt: PROMPT_CHART,
        tool_names: &[
            "find_data_sources",
            "preview_data",
            "build_chart",
        ],
    },
    Skill {
        id: "table-builder",
        title: "Таблицы данных",
        description:
            "Построение таблиц из данных: пользователь описывает, что показать — агент собирает \
                      SELECT, задаёт колонки/форматы/условное форматирование и публикует таблицу-плагин \
                      (фильтры, сортировка, итоги, экспорт).",
        intents: &["table_build"],
        prompt: PROMPT_TABLE,
        tool_names: &[
            "find_data_sources",
            "preview_data",
            "build_table",
        ],
    },
    Skill {
        id: "system-admin",
        title: "Системная диагностика",
        description: "Состояние системы, производительность, фоновые задачи, целостность данных.",
        intents: &["sys_admin"],
        prompt: PROMPT_SYS_ADMIN,
        tool_names: &[
            "check_system_health",
            "get_performance_stats",
            "list_background_jobs",
            "get_data_integrity_report",
        ],
    },
    Skill {
        id: "mailbox",
        title: "Почта",
        description: "Чтение входящих и отправка писем от лица почтового ящика системы \
                      (IMAP/SMTP): найти письмо, прочитать, ответить или написать новое.",
        intents: &["mailbox"],
        prompt: PROMPT_MAILBOX,
        tool_names: &["list_emails", "read_email", "send_email"],
    },
    Skill {
        id: "kb-curation",
        title: "База знаний",
        description:
            "Работа с базой знаний: чтение статей, подготовка правок, тикеты, анализ пробелов.",
        intents: &["kb_curation"],
        prompt: PROMPT_KB,
        tool_names: &[
            "list_kb_documents",
            "get_kb_document",
            "create_kb_edit",
            "update_kb_edit_articles",
            "list_open_kb_edits",
            "write_kb_document",
            "execute_query",
        ],
    },
];

// ─── Мета-инструменты (Фаза B: progressive disclosure) ───────────────────────

/// Определения мета-инструментов навыков (входят в core).
pub fn meta_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: "list_skills".into(),
            description: "Список доступных навыков (id, title, description). Вызови, если для задачи \
                          нужен набор возможностей, которого нет в текущих инструментах."
                .into(),
            parameters: json!({ "type": "object", "properties": {} }),
        },
        ToolDefinition {
            name: "use_skill".into(),
            description: "Активировать навык по id (из list_skills): добавит его инструменты и \
                          инструкции в текущий диалог. Активируй по одному навыку под текущую задачу."
                .into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "id навыка, напр. 'plugin-authoring'." }
                },
                "required": ["id"]
            }),
        },
    ]
}

/// Результат `list_skills` — только разрешённые агенту навыки.
pub fn list_skills_result(agent_type: &AgentType) -> Value {
    let allowed = allowed_skills_for(agent_type);
    let items: Vec<Value> = SKILLS
        .iter()
        .filter(|s| allowed.contains(&s.id))
        .map(|s| json!({ "id": s.id, "title": s.title, "description": s.description }))
        .collect();
    json!({
        "skills": items,
        "hint": "Активируй нужный навык: use_skill(\"<id>\")."
    })
}

/// Результат `use_skill` — сигнал активации (`_activate_skill`) ловится tool-циклом.
pub fn use_skill_result(arguments_json: &str, agent_type: &AgentType) -> Value {
    let id = serde_json::from_str::<Value>(arguments_json)
        .ok()
        .and_then(|v| v.get("id").and_then(|x| x.as_str()).map(str::to_string))
        .unwrap_or_default();
    let Some(skill) = skill_by_id(&id) else {
        return json!({ "error": format!("Навык '{}' не найден. Вызови list_skills.", id) });
    };
    if !allowed_skills_for(agent_type).contains(&skill.id) {
        return json!({
            "error": format!("Навык '{}' недоступен для текущего агента.", skill.id)
        });
    }
    json!({
        "_activate_skill": skill.id,
        "activated": skill.title,
        "hint": "Навык активирован: его инструменты и инструкции доступны со следующего шага."
    })
}

// ─── Поиск/маппинг ───────────────────────────────────────────────────────────

pub fn skill_by_id(id: &str) -> Option<&'static Skill> {
    SKILLS.iter().find(|s| s.id == id)
}

/// Навык по интенту роутера (первое совпадение).
pub fn skill_for_intent(intent: &str) -> Option<&'static Skill> {
    SKILLS.iter().find(|s| s.intents.contains(&intent))
}

/// Навыки по умолчанию (default-bias по роли) — активируются, если роутер не выбрал.
pub fn default_skills_for(agent_type: &AgentType) -> Vec<&'static str> {
    match agent_type {
        AgentType::BusinessAnalyst => vec!["data-analytics"],
        AgentType::SystemAdmin => vec!["system-admin"],
        AgentType::KbAdmin => vec!["kb-curation"],
        AgentType::PluginAdmin => vec!["plugin-authoring"],
        AgentType::General => vec!["data-analytics"],
    }
    // chart-builder активируется по интенту chart_build (см. router), не как default-bias.
}

/// Allow-list навыков по роли — граница безопасности `use_skill`.
pub fn allowed_skills_for(agent_type: &AgentType) -> Vec<&'static str> {
    match agent_type {
        AgentType::BusinessAnalyst => {
            vec![
                "data-analytics",
                "bi-authoring",
                "chart-builder",
                "table-builder",
                "mailbox",
            ]
        }
        AgentType::SystemAdmin => vec!["system-admin"],
        AgentType::KbAdmin => vec!["kb-curation"],
        AgentType::PluginAdmin => {
            vec![
                "plugin-authoring",
                "data-analytics",
                "chart-builder",
                "table-builder",
            ]
        }
        AgentType::General => SKILLS.iter().map(|s| s.id).collect(),
    }
}

// ─── Сборка инструментов/guard ───────────────────────────────────────────────

/// «Вселенная» всех определений инструментов (core + все бандлы + мета).
/// NB: при добавлении нового бандла обнови также `tools_catalog()` (там бандлы
/// перечислены заново парами `(category, defs)`, т.к. здесь метка категории теряется).
fn tool_universe() -> Vec<ToolDefinition> {
    let mut v = Vec::new();
    v.extend(super::data_tools::tool_definitions());
    v.extend(super::tool_executor::shared_tool_definitions());
    v.extend(super::tool_executor::analyst_tool_definitions());
    v.extend(super::admin_tools::admin_tool_definitions());
    v.extend(super::kb_admin_tools::kb_admin_tool_definitions());
    v.extend(super::plugin_tools::plugin_tool_definitions());
    v.extend(super::chart_tools::chart_tool_definitions());
    v.extend(super::table_tools::table_tool_definitions());
    v.extend(super::mail_tools::mail_tool_definitions());
    v.extend(meta_tool_definitions());
    v
}

/// Множество имён активных инструментов (core ∪ инструменты активных навыков).
/// Единый источник истины для авторизации в `execute_tool_call`.
pub fn active_tool_names(active_skills: &[&str]) -> HashSet<String> {
    // Confident chart/table intents use a deliberately closed three-tool workflow.
    // Meta/introspection tools in this case only compete with the canonical path.
    if active_skills.len() == 1 && matches!(active_skills[0], "chart-builder" | "table-builder") {
        return skill_by_id(active_skills[0])
            .map(|skill| {
                skill
                    .tool_names
                    .iter()
                    .map(|name| name.to_string())
                    .collect()
            })
            .unwrap_or_default();
    }
    let mut names: HashSet<String> = CORE_TOOLS.iter().map(|s| s.to_string()).collect();
    for id in active_skills {
        if let Some(s) = skill_by_id(id) {
            for t in s.tool_names {
                names.insert(t.to_string());
            }
        }
    }
    names
}

/// Собрать определения инструментов для передачи LLM: core + активные навыки, дедуп по имени.
pub fn assemble_tools(active_skills: &[&str]) -> Vec<ToolDefinition> {
    let wanted = active_tool_names(active_skills);
    let chart_builder = active_skills == ["chart-builder"];
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for mut def in tool_universe() {
        if wanted.contains(def.name.as_str()) && seen.insert(def.name.clone()) {
            // В общем data-analysis preview может быть без presentation. В chart-builder
            // это другой контракт: без chart невозможно получить build_ready, и модель
            // раньше зацикливалась на успешных, но бесполезных preview.
            if chart_builder && def.name == "preview_data" {
                def.parameters["required"] = serde_json::json!(["source", "chart"]);
                def.parameters["properties"]["chart"]["description"] = serde_json::json!(
                    "Обязательный presentation-спек. Preview должен вернуть build_ready перед build_chart."
                );
            }
            out.push(def);
        }
    }
    out
}

/// Базовый системный промпт (role-agnostic).
pub fn core_prompt() -> &'static str {
    CORE_PROMPT
}

/// Каталог навыков для UI/обзора: id, описание, интенты, инструменты, для каких ролей доступен.
pub fn catalog() -> Value {
    const ALL_AGENTS: &[AgentType] = &[
        AgentType::BusinessAnalyst,
        AgentType::General,
        AgentType::PluginAdmin,
        AgentType::SystemAdmin,
        AgentType::KbAdmin,
    ];
    let items: Vec<Value> = SKILLS
        .iter()
        .map(|s| {
            let allowed_for: Vec<&str> = ALL_AGENTS
                .iter()
                .filter(|t| allowed_skills_for(t).contains(&s.id))
                .map(|t| t.as_str())
                .collect();
            json!({
                "id": s.id,
                "title": s.title,
                "description": s.description,
                "intents": s.intents,
                "tools": s.tool_names,
                "tool_count": s.tool_names.len(),
                "allowed_for": allowed_for,
            })
        })
        .collect();
    json!({
        "core_tools": CORE_TOOLS,
        "skills": items,
        "total": items.len(),
    })
}

/// Каталог инструментов для UI/обзора: полное определение каждого инструмента
/// (name, description, JSON-схема параметров) + производные поля (category, is_core,
/// список навыков, в которых он используется). Дедуп по имени.
///
/// NB: бандлы перечислены заново парами `(category, defs)` — при добавлении нового
/// бандла в `tool_universe()` продублируй его и здесь с меткой категории.
pub fn tools_catalog() -> Value {
    let bundles: Vec<(&str, Vec<ToolDefinition>)> = vec![
        ("data", super::data_tools::tool_definitions()),
        ("shared", super::tool_executor::shared_tool_definitions()),
        ("analyst", super::tool_executor::analyst_tool_definitions()),
        ("admin", super::admin_tools::admin_tool_definitions()),
        ("kb", super::kb_admin_tools::kb_admin_tool_definitions()),
        ("plugin", super::plugin_tools::plugin_tool_definitions()),
        ("chart", super::chart_tools::chart_tool_definitions()),
        ("table", super::table_tools::table_tool_definitions()),
        ("mail", super::mail_tools::mail_tool_definitions()),
        ("meta", meta_tool_definitions()),
    ];

    let mut seen: HashSet<String> = HashSet::new();
    let mut tools: Vec<Value> = Vec::new();
    for (category, defs) in bundles {
        for def in defs {
            if !seen.insert(def.name.clone()) {
                continue;
            }
            let is_core = CORE_TOOLS.contains(&def.name.as_str());
            let skills: Vec<&str> = SKILLS
                .iter()
                .filter(|s| s.tool_names.contains(&def.name.as_str()))
                .map(|s| s.id)
                .collect();
            tools.push(json!({
                "name": def.name,
                "description": def.description,
                "parameters": def.parameters,
                "category": category,
                "is_core": is_core,
                "skills": skills,
            }));
        }
    }

    let total = tools.len();
    json!({ "tools": tools, "total": total })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_only_has_meta_and_base() {
        let names = active_tool_names(&[]);
        assert!(names.contains("get_architecture_overview"));
        assert!(names.contains("list_skills"));
        assert!(names.contains("use_skill"));
        assert!(!names.contains("plugin_upsert"));
        assert!(!names.contains("execute_query"));
    }

    #[test]
    fn plugin_skill_brings_plugin_tools() {
        let tools = assemble_tools(&["plugin-authoring"]);
        let names: HashSet<_> = tools.iter().map(|t| t.name.clone()).collect();
        assert!(names.contains("plugin_upsert"));
        assert!(names.contains("execute_query")); // из b_sql
        assert!(names.contains("use_skill")); // core всегда
    }

    #[test]
    fn assemble_dedups_shared_bundle() {
        // execute_query есть и в data-analytics, и в plugin-authoring — не должно дублироваться.
        let tools = assemble_tools(&["data-analytics", "plugin-authoring"]);
        let eq = tools.iter().filter(|t| t.name == "execute_query").count();
        assert_eq!(eq, 1);
    }

    #[test]
    fn data_skill_exposes_semantic_queries_and_raw_fallback() {
        let names: HashSet<_> = assemble_tools(&["data-analytics"])
            .into_iter()
            .map(|tool| tool.name)
            .collect();
        assert!(names.contains("list_data_sources"));
        assert!(names.contains("query_data_schema"));
        assert!(names.contains("run_data_view_scalar"));
        assert!(names.contains("run_data_view_drilldown"));
        assert!(names.contains("execute_query"));
    }

    #[test]
    fn allow_list_blocks_escalation() {
        let allowed = allowed_skills_for(&AgentType::BusinessAnalyst);
        assert!(!allowed.contains(&"system-admin"));
        assert!(!allowed.contains(&"plugin-authoring"));
        assert!(allowed.contains(&"data-analytics"));
    }

    #[test]
    fn intent_maps_to_skill() {
        assert_eq!(
            skill_for_intent("plugin_dev").unwrap().id,
            "plugin-authoring"
        );
        assert_eq!(skill_for_intent("data_query").unwrap().id, "data-analytics");
        assert_eq!(skill_for_intent("chart_build").unwrap().id, "chart-builder");
        assert_eq!(skill_for_intent("table_build").unwrap().id, "table-builder");
        assert!(skill_for_intent("meta_smalltalk").is_none());
    }

    #[test]
    fn builder_skills_expose_only_the_canonical_three_tools() {
        let chart: HashSet<_> = assemble_tools(&["chart-builder"])
            .into_iter()
            .map(|tool| tool.name)
            .collect();
        assert_eq!(
            chart,
            ["find_data_sources", "preview_data", "build_chart"]
                .into_iter()
                .map(str::to_string)
                .collect()
        );
        let table: HashSet<_> = assemble_tools(&["table-builder"])
            .into_iter()
            .map(|tool| tool.name)
            .collect();
        assert_eq!(
            table,
            ["find_data_sources", "preview_data", "build_table"]
                .into_iter()
                .map(str::to_string)
                .collect()
        );
        let preview = assemble_tools(&["chart-builder"])
            .into_iter()
            .find(|tool| tool.name == "preview_data")
            .expect("chart preview definition");
        assert_eq!(
            preview.parameters["required"],
            serde_json::json!(["source", "chart"])
        );
    }
}
