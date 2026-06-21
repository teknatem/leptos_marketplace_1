pub mod details;
pub mod header_button;
pub mod list;

pub use header_button::AiChatHeaderButton;

/// Ключ для `AppGlobalContext.form_states`: первое сообщение, которое нужно
/// автоматически отправить при открытии только что созданного чата.
///
/// Создание чата на странице списка сохраняет вопрос пользователя под этим ключом,
/// а страница деталей чата при загрузке забирает его и отправляет как первое сообщение.
pub fn pending_first_message_key(chat_id: &str) -> String {
    format!("a018_pending_first_msg_{}", chat_id)
}
