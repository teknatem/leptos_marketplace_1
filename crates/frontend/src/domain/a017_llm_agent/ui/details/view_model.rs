//! AI-сотрудник (a017) — ViewModel карточки.
//!
//! Персона поверх технического подключения a038: имя, аватар, почта, специализация,
//! обязанности, расписание. Техника (провайдер/креды/тюнинг) — в связанном подключении.

use leptos::prelude::*;

/// Опция подключения для селекта «Подключение».
#[derive(Clone)]
pub struct ConnOption {
    pub id: String,
    pub name: String,
    /// Курируемый список моделей подключения (для выбора закреплённой модели сотрудника).
    pub allowed_models: Vec<String>,
    /// Дефолтная модель подключения (используется, если сотрудник не закрепил свою).
    pub default_model: String,
}

/// Навык для read-only блока «Навыки».
#[derive(Clone)]
pub struct SkillItem {
    pub id: String,
    pub title: String,
    pub description: String,
}

/// ViewModel карточки AI-сотрудника.
#[derive(Clone, Copy)]
pub struct LlmAgentDetailsVm {
    pub code: RwSignal<String>,
    /// Имя сотрудника (base.description).
    pub description: RwSignal<String>,
    pub comment: RwSignal<String>,

    /// Специализация (agent_type) — определяет навыки.
    pub agent_type: RwSignal<String>,
    /// Должностные обязанности (system_prompt).
    pub system_prompt: RwSignal<String>,

    /// Техническое подключение a038 (UUID).
    pub connection_id: RwSignal<String>,
    /// Закреплённая сотрудником модель (пусто → дефолт подключения).
    pub model_name: RwSignal<String>,

    pub avatar: RwSignal<String>,
    pub email: RwSignal<String>,
    pub schedule_cron: RwSignal<String>,
    pub is_active: RwSignal<bool>,
    pub is_primary: RwSignal<bool>,

    /// Список доступных подключений a038.
    pub connections: RwSignal<Vec<ConnOption>>,
    /// Навыки core (активны по умолчанию) для выбранной специализации.
    pub skills_core: RwSignal<Vec<SkillItem>>,
    /// Навыки extended (доступны по запросу через use_skill).
    pub skills_extended: RwSignal<Vec<SkillItem>>,

    pub error: Signal<Option<String>>,
    pub set_error: WriteSignal<Option<String>>,
}

impl LlmAgentDetailsVm {
    pub fn new() -> Self {
        let (error, set_error) = signal::<Option<String>>(None);
        Self {
            code: RwSignal::new(String::new()),
            description: RwSignal::new(String::new()),
            comment: RwSignal::new(String::new()),
            agent_type: RwSignal::new("business_analyst".to_string()),
            system_prompt: RwSignal::new(String::new()),
            connection_id: RwSignal::new(String::new()),
            model_name: RwSignal::new(String::new()),
            avatar: RwSignal::new(String::new()),
            email: RwSignal::new(String::new()),
            schedule_cron: RwSignal::new(String::new()),
            is_active: RwSignal::new(true),
            is_primary: RwSignal::new(false),
            connections: RwSignal::new(Vec::new()),
            skills_core: RwSignal::new(Vec::new()),
            skills_extended: RwSignal::new(Vec::new()),
            error: error.into(),
            set_error,
        }
    }

    /// Список моделей для селекта закреплённой модели — allowed_models выбранного подключения.
    pub fn models_for_selected_connection(&self) -> Vec<String> {
        let cid = self.connection_id.get();
        self.connections
            .get()
            .into_iter()
            .find(|c| c.id == cid)
            .map(|c| c.allowed_models)
            .unwrap_or_default()
    }

    /// Собрать DTO для сохранения. Технические поля (провайдер/креды/тюнинг) не отправляем —
    /// у сотрудника они вестигиальны, бэкенд подставит дефолты / сохранит прежние.
    pub fn build_save_dto(&self, id: Option<String>) -> serde_json::Value {
        let opt = |s: String| if s.trim().is_empty() { None } else { Some(s) };
        serde_json::json!({
            "id": id,
            "code": self.code.get(),
            "description": self.description.get(),
            "comment": opt(self.comment.get()),
            "agent_type": self.agent_type.get(),
            "system_prompt": opt(self.system_prompt.get()),
            "connection_id": opt(self.connection_id.get()),
            "model_name": self.model_name.get(),
            "avatar": opt(self.avatar.get()),
            "email": opt(self.email.get()),
            "schedule_cron": opt(self.schedule_cron.get()),
            "is_active": self.is_active.get(),
            "is_primary": self.is_primary.get(),
        })
    }
}

impl Default for LlmAgentDetailsVm {
    fn default() -> Self {
        Self::new()
    }
}
