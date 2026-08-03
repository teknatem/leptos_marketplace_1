//! Роутер интентов (Фаза 0).
//!
//! Классифицирует сообщение пользователя в один из известных интентов. На Фазе 0
//! результат только записывается в метаданные сообщения и логируется — поведение
//! пайплайна (набор tools, промпт) пока не меняется.
//!
//! Основной путь — дешёвый LLM-вызов без инструментов (`chat_completion`), который
//! просят вернуть строгий JSON `{ "intent": "...", "confidence": 0.0 }`.
//! Если вызов не удался или ответ не распарсился — fallback на правила/ключевые слова.

use super::types::{ChatMessage, LlmProvider};
use contracts::domain::a017_llm_agent::aggregate::AgentType;

/// Известные интенты уровня сообщения (см. план, §1).
pub const KNOWN_INTENTS: &[&str] = &[
    "func_help",                   // вопрос по функционалу приложения
    "data_query",                  // аналитика по данным (SQL/drilldown/индикаторы)
    "sales_query",                 // продажи/выручка/заказы/маржа (аналитик продаж)
    "marketing_query",             // реклама/воронка/поисковая аналитика/промо (маркетолог)
    "marketplace_funnel_analysis", // сквозная воронка маркетплейса и сравнение этапов
    "finance_query",               // главная книга/сверка выручки/взаиморасчёты (финансист)
    "bi_authoring",                // создание индикатора/дашборда
    "chart_build",                 // построить график/диаграмму по данным
    "table_build",                 // построить таблицу данных (плагин-таблица)
    "plugin_dev",                  // создание/доработка плагина
    "sys_admin",                   // системная диагностика
    "kb_curation",                 // работа с базой знаний
    "mailbox",                     // чтение/отправка почты
    "support",                     // обращение в поддержку: сбой, пожелание, тикет
    "meta_smalltalk",              // приветствие/уточнение/«что ты умеешь»
];

/// Результат классификации.
#[derive(Debug, Clone)]
pub struct IntentResult {
    pub intent: String,
    pub confidence: f64,
    /// Откуда получен результат — для аналитики/отладки ("llm" | "rules").
    pub source: &'static str,
    pub tokens_used: i32,
}

impl IntentResult {
    fn new(intent: impl Into<String>, confidence: f64, source: &'static str) -> Self {
        Self {
            intent: intent.into(),
            confidence,
            source,
            tokens_used: 0,
        }
    }
}

/// Системный промпт классификатора. Просим строгий JSON без пояснений.
fn classifier_system_prompt() -> String {
    format!(
        "Ты — классификатор запросов пользователя в системе управления маркетплейсами \
         (Wildberries, OZON, Яндекс.Маркет). Определи ЕДИНСТВЕННЫЙ интент сообщения.\n\n\
         Возможные интенты:\n\
         - func_help: как пользоваться приложением, где найти функцию, что делает фича.\n\
         - data_query: общая аналитика по данным — остатки, отчёты, SQL, drilldown, индикаторы (без явной темы продаж/маркетинга/финансов).\n\
         - sales_query: продажи, выручка, заказы, средний чек, маржа/прибыль, динамика продаж.\n\
         - marketing_query: реклама, ДРР, CTR, ставки, воронка продаж, конверсии, выкуп, поисковая аналитика, промо/акции.\n\
         - marketplace_funnel_analysis: рассчитать, проверить или сравнить этапы воронки маркетплейса по данным OZON, Wildberries или Яндекс.Маркета.\n\
         - finance_query: главная книга, обороты по счетам, сверка выручки (fina/ybuh), взаиморасчёты, комиссии.\n\
         - bi_authoring: просьба СОЗДАТЬ индикатор/дашборд/KPI.\n\
         - chart_build: построить ГРАФИК/диаграмму/визуализацию по данным (линия, столбцы, доли).\n\
         - table_build: построить ТАБЛИЦУ данных по данным (колонки/строки, фильтры, сортировка, итоги).\n\
         - plugin_dev: создать/доработать/протестировать плагин (JS).\n\
         - sys_admin: состояние системы, производительность, фоновые задачи, целостность данных.\n\
         - kb_curation: работа с базой знаний — прочитать/исправить статью, тикет правки.\n\
         - mailbox: почта — прочитать входящие письма, найти письмо, ответить или отправить письмо.\n\
         - support: обращение по работе самой программы — что-то не работает/ошибка/сломалось, просьба доработать или идея по улучшению, просьба завести тикет/заявку.\n\
         - meta_smalltalk: приветствие, благодарность, «что ты умеешь», уточнение без конкретной задачи.\n\n\
         Ответь СТРОГО валидным JSON без пояснений и без markdown:\n\
         {{\"intent\": \"<один из: {}>\", \"confidence\": <число 0.0..1.0>}}",
        KNOWN_INTENTS.join(", ")
    )
}

/// Классифицировать сообщение. Никогда не паникует и всегда возвращает результат
/// (в худшем случае — fallback по правилам).
pub async fn classify_intent(
    provider: &dyn LlmProvider,
    user_message: &str,
    recent_summary: &str,
    seed_agent_type: &AgentType,
) -> IntentResult {
    // Очень короткие/пустые реплики — болтовня, без LLM-вызова.
    let trimmed = user_message.trim();
    if trimmed.chars().count() < 3 {
        return IntentResult::new("meta_smalltalk", 0.5, "rules");
    }

    let mut user_block = String::new();
    if !recent_summary.trim().is_empty() {
        user_block.push_str("Краткий контекст последних ходов:\n");
        user_block.push_str(recent_summary.trim());
        user_block.push_str("\n\n");
    }
    user_block.push_str("Сообщение пользователя:\n");
    user_block.push_str(trimmed);

    let messages = vec![
        ChatMessage::system(classifier_system_prompt()),
        ChatMessage::user(user_block),
    ];

    match provider.chat_completion(&messages).await {
        Ok(resp) => match parse_intent_json(&resp.content) {
            Some(mut result) => {
                result.tokens_used = resp.tokens_used.unwrap_or(0);
                result
            }
            None => {
                tracing::warn!(
                    "[router] не удалось распарсить ответ классификатора, fallback на правила: {}",
                    preview(&resp.content)
                );
                rule_based(trimmed, seed_agent_type)
            }
        },
        Err(e) => {
            tracing::warn!(
                "[router] ошибка LLM-классификатора ({:?}), fallback на правила",
                e
            );
            rule_based(trimmed, seed_agent_type)
        }
    }
}

fn preview(s: &str) -> String {
    s.chars().take(120).collect()
}

/// Маппинг интента (см. `KNOWN_INTENTS`) в тип агента-исполнителя.
/// Обратное к seed-таблице `AgentType → intent`. Используется почтовым конвейером
/// для выбора специалиста по содержимому письма.
pub fn intent_to_agent_type(intent: &str) -> AgentType {
    match intent {
        "kb_curation" => AgentType::KbAdmin,
        // Разработчик один: и плагины, и поддержка пользователей (навык `support`).
        "plugin_dev" | "support" => AgentType::PluginAdmin,
        "sys_admin" => AgentType::SystemAdmin,
        "sales_query" => AgentType::SalesAnalyst,
        "marketing_query" => AgentType::Marketer,
        "marketplace_funnel_analysis" => AgentType::Marketer,
        "finance_query" => AgentType::Financier,
        // data_query | chart_build | table_build | bi_authoring | func_help — аналитик.
        "data_query" | "chart_build" | "table_build" | "bi_authoring" | "func_help" => {
            AgentType::BusinessAnalyst
        }
        // meta_smalltalk и всё прочее — общий агент.
        _ => AgentType::CoordinatorAdmin,
    }
}

/// Быстрая (rule-based, без LLM) классификация интента для синхронной предактивации
/// навыков перед основным циклом. Полный LLM-роутер по-прежнему идёт конкурентно.
pub fn quick_intent(message: &str, seed_agent_type: &AgentType) -> String {
    quick_intent_result(message, seed_agent_type).intent
}

/// Быстрый результат вместе с confidence — chat selector использует низкую
/// уверенность как признак follow-up, не меняющего текущую специализацию задачи.
pub fn quick_intent_result(message: &str, seed_agent_type: &AgentType) -> IntentResult {
    let trimmed = message.trim();
    if trimmed.chars().count() < 3 {
        return IntentResult::new("meta_smalltalk", 0.5, "rules");
    }
    rule_based(trimmed, seed_agent_type)
}

/// Распарсить `{ "intent": "...", "confidence": ... }` из ответа модели,
/// допуская обрамление markdown-кодом и лишний текст вокруг.
fn parse_intent_json(content: &str) -> Option<IntentResult> {
    // Найти первый '{' и последний '}' — грубое извлечение JSON-объекта.
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    if end <= start {
        return None;
    }
    let json_slice = &content[start..=end];
    let value: serde_json::Value = serde_json::from_str(json_slice).ok()?;

    let intent = value.get("intent")?.as_str()?.trim().to_lowercase();
    if !KNOWN_INTENTS.contains(&intent.as_str()) {
        return None;
    }
    let confidence = value
        .get("confidence")
        .and_then(|c| c.as_f64())
        .unwrap_or(0.6)
        .clamp(0.0, 1.0);

    Some(IntentResult::new(intent, confidence, "llm"))
}

/// Резервная классификация по ключевым словам. Низкая уверенность, чтобы на Фазе 1
/// такие случаи можно было отличать и при необходимости уточнять у пользователя.
fn rule_based(message: &str, seed_agent_type: &AgentType) -> IntentResult {
    let m = message.to_lowercase();

    let any = |needles: &[&str]| needles.iter().any(|n| m.contains(n));

    // Поддержка — раньше остальных правил: «не работает график» это обращение о сбое,
    // а не просьба построить график. Исключение — разработка плагинов (свой интент).
    if !any(&["плагин", "plugin"])
        && any(&[
            "тикет",
            "обращени",
            "заявк",
            "поддержк",
            "баг",
            "не работает",
            "не работают",
            "не открывается",
            "не грузит",
            "не загружается",
            "не сохраня",
            "ошибк",
            "сломал",
            "глючит",
            "зависает",
            "доработ",
            "пожелани",
            "неудобно",
        ])
    {
        return IntentResult::new("support", 0.5, "rules");
    }
    if any(&["график", "графік", "диаграмм", "chart", "чарт", "визуализ"])
    {
        return IntentResult::new("chart_build", 0.5, "rules");
    }
    if any(&["таблиц", "table", "грид", "grid", "data-grid"]) {
        return IntentResult::new("table_build", 0.5, "rules");
    }
    if any(&["плагин", "plugin", "виджет"]) {
        return IntentResult::new("plugin_dev", 0.45, "rules");
    }
    if any(&["индикатор", "дашборд", "kpi", "дашбоард", "показател"])
        && any(&["созда", "добав", "сдела", "построй"])
    {
        return IntentResult::new("bi_authoring", 0.45, "rules");
    }
    if any(&[
        "здоров",
        "производительн",
        "фонов",
        "задач",
        "целостност",
        "диагност",
        "health",
    ]) {
        return IntentResult::new("sys_admin", 0.4, "rules");
    }
    if any(&["база знаний", "статья", "статью", "knowledge", "kb"]) {
        return IntentResult::new("kb_curation", 0.4, "rules");
    }
    if any(&[
        "письм",
        "почт",
        "email",
        "e-mail",
        "mail",
        "входящ",
        "отправь письмо",
        "напиши письмо",
    ]) {
        return IntentResult::new("mailbox", 0.45, "rules");
    }
    // Маркетинг раньше продаж: «реклама/воронка/промо» — маркетолог, не общий data_query.
    // Маркетплейс-слово: включая разговорные «wb»/«вб», как пишут продажники (кабинет
    // «WB - SANSTAR» → lowercase содержит "wb"). Латинский "wb" в русских словах не встречается.
    let marketplace_word = any(&[
        "ozon",
        "озон",
        "wildberries",
        "вайлдберриз",
        "вайлдберис",
        "wb",
        "вб",
        "яндекс маркет",
        "маркетплейс",
    ]);
    // «Воронка + маркетплейс», а также WB-вопросы по низу воронки (выкуп/отмены/возвраты/топ
    // товаров/конверсии), которые продажник задаёт БЕЗ слова «воронка», но их авторитет — навык
    // воронки (иначе уходят в data-analytics и теряют гардрейлы: funnel_order_count≠order_count,
    // фильтр кабинета, лаг выкупа).
    if any(&[
        "воронка маркетплейс",
        "воронку маркетплейс",
        "funnel marketplace",
        "ozon funnel",
        "wildberries funnel",
        "яндекс маркет funnel",
    ]) || (any(&["воронк", "funnel"]) && marketplace_word)
        || (marketplace_word
            && any(&[
                "выкуп",
                "отмен",
                "возврат",
                "топ товаров",
                "топ-5",
                "топ 5",
                "конверси",
            ]))
    {
        return IntentResult::new("marketplace_funnel_analysis", 0.55, "rules");
    }
    if any(&[
        "реклам",
        "дрр",
        "ctr",
        "воронк",
        "конверс",
        "выкуп",
        "ставк",
        "промо",
        "акци",
        "поисков",
        "джем",
    ]) {
        return IntentResult::new("marketing_query", 0.45, "rules");
    }
    if any(&[
        "главная книга",
        "главную книгу",
        "сверк",
        "взаиморасч",
        "комисс",
        "оборот",
        "дебет",
        "кредит",
        "проводк",
        "реализац",
    ]) {
        return IntentResult::new("finance_query", 0.45, "rules");
    }
    if any(&[
        "выручк",
        "продаж",
        "заказ",
        "маржинальн",
        "маржа",
        "прибыл",
        "средний чек",
    ]) {
        return IntentResult::new("sales_query", 0.45, "rules");
    }
    if any(&[
        "отчёт",
        "отчет",
        "остат",
        "sql",
        "сколько",
        "сумм",
        "возврат",
    ]) {
        return IntentResult::new("data_query", 0.45, "rules");
    }
    if any(&[
        "как ",
        "где ",
        "что такое",
        "что делает",
        "помоги найти",
        "инструкц",
    ]) {
        return IntentResult::new("func_help", 0.4, "rules");
    }

    // Иначе — seed по типу агента (back-compat), низкая уверенность.
    let seeded = match seed_agent_type {
        AgentType::SystemAdmin => "sys_admin",
        AgentType::KbAdmin => "kb_curation",
        // Расплывчатая реплика разработчику — это почти всегда вопрос по системе:
        // просьбы про плагины ловятся явным правилом по слову «плагин» выше.
        AgentType::PluginAdmin => "support",
        AgentType::SalesAnalyst => "sales_query",
        AgentType::Marketer => "marketing_query",
        AgentType::Financier => "finance_query",
        _ => "data_query",
    };
    IntentResult::new(seeded, 0.25, "rules")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marketplace_funnel_rule_precedes_generic_marketing() {
        let result =
            quick_intent_result("Сравни воронку OZON за два периода", &AgentType::Marketer);
        assert_eq!(result.intent, "marketplace_funnel_analysis");
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn wb_cabinet_funnel_questions_route_to_funnel_skill() {
        // Продажник пишет «WB», а не «Wildberries»; кабинет — «WB - SANSTAR».
        for q in [
            "Посчитай воронку продаж по кабинету WB - SANSTAR за июль",
            "Какой процент выкупа по WB - SANSTAR за июль 2026?",
            "Топ-5 товаров WB - SANSTAR по заказам за июль",
            "Сколько отмен и возвратов по WB за июль?",
        ] {
            let result = quick_intent_result(q, &AgentType::BusinessAnalyst);
            assert_eq!(
                result.intent, "marketplace_funnel_analysis",
                "ожидали funnel для: {q}"
            );
        }
    }

    #[test]
    fn generic_buyout_without_marketplace_stays_marketing() {
        // Без маркетплейс-слова «выкуп» остаётся маркетингом (funnel-правило не срабатывает).
        let result = quick_intent_result("Какой у нас процент выкупа?", &AgentType::BusinessAnalyst);
        assert_eq!(result.intent, "marketing_query");
    }
}
