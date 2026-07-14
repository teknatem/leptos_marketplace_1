//! Политика авторизации почтового конвейера: какие типы агентов может запускать
//! письмом данный пользователь.
//!
//! Личность отправителя не доходит до инструментов во время прогона — фактические
//! права определяются `agent_type` выбранного подключения. Поэтому гейт делается ЗДЕСЬ,
//! на этапе диспетчеризации: роль пользователя ограничивает допустимые типы агентов.

use contracts::domain::a038_llm_connection::aggregate::AgentType;
use contracts::system::users::User;
use std::collections::HashSet;

/// Множество типов агентов, которые пользователь вправе запустить через почту.
///
/// Базовый уровень для любого активного пользователя — аналитик и общий агент
/// (аналитика/данные read-only). Привилегированные типы (KB, плагины, sys-admin)
/// требуют соответствующей роли.
pub fn allowed_agent_types_for_user(user: &User) -> HashSet<AgentType> {
    let mut set = HashSet::new();
    set.insert(AgentType::BusinessAnalyst);
    set.insert(AgentType::General);

    if user.is_admin {
        set.insert(AgentType::KbAdmin);
        set.insert(AgentType::PluginAdmin);
        set.insert(AgentType::SystemAdmin);
        return set;
    }

    // Менеджер ведёт базу знаний и плагины; sys-admin остаётся только за админом.
    if user.primary_role_code == "manager" {
        set.insert(AgentType::KbAdmin);
        set.insert(AgentType::PluginAdmin);
    }

    set
}
