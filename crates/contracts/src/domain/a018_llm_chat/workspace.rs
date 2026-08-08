//! DTO рабочего каталога чата: задачи (активности) и их файлы.
//!
//! Каталог живёт на диске рядом с базой знаний, а не в БД: анкета и план должны
//! переживать сжатие истории диалога. Эти структуры — то, что видит UI.

use serde::{Deserialize, Serialize};

/// Задача (активность) внутри чата.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatActivity {
    /// Имя каталога, например `002-grafik-voronki-wb`.
    pub name: String,
    pub ordinal: u32,
    /// Часть после номера — человекочитаемое описание задачи.
    pub description: String,
    pub is_active: bool,
}

/// Файл внутри задачи.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChatFile {
    /// Путь относительно каталога задачи (`plan.md`, `steps/003-calc-….json`).
    pub path: String,
    pub bytes: u64,
    /// Живые документы (анкета, план, заметки) правятся из UI; журнал шагов — нет.
    pub is_live_document: bool,
}

/// Уточняющий вопрос из анкеты активной задачи.
///
/// Модель задаёт их, когда не может вывести параметр из запроса. Вопрос с
/// `options` рисуется выбором, без них — полем ввода.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntakeQuestion {
    /// Идентификатор — он же имя поля анкеты, которое уточняется.
    pub id: String,
    pub text: String,
    #[serde(default)]
    pub options: Vec<String>,
    /// Уже данный ответ, если есть.
    #[serde(default)]
    pub answer: Option<String>,
}

/// Ответ на запрос каталога чата.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChatWorkspaceView {
    pub activities: Vec<ChatActivity>,
    /// Файлы активной задачи.
    pub files: Vec<ChatFile>,
    /// Вопросы из анкеты активной задачи (отвеченные и нет).
    #[serde(default)]
    pub questions: Vec<IntakeQuestion>,
}

/// Ответ пользователя на уточняющий вопрос.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerQuestionRequest {
    pub question_id: String,
    pub answer: String,
}

/// Содержимое одного файла.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatFileContent {
    pub path: String,
    pub content: String,
    pub is_live_document: bool,
}

/// Запрос на правку живого документа из UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveChatFileRequest {
    pub content: String,
}
