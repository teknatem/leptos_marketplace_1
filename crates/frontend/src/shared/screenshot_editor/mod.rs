//! Редактор скриншотов: модальное окно с аннотациями (рамки, стрелки, комментарии).
//! Переиспользуется в LLM-чате, тикетах и других местах, где нужна разметка изображений.

mod clipboard;
mod editor;

pub use clipboard::{first_image_from_data_transfer, read_image_file_from_clipboard};
pub use editor::ScreenshotEditor;

/// Черновик изображения, открытый в редакторе: исходный файл + object-URL для превью.
#[derive(Clone)]
pub struct PendingScreenshot {
    pub file: web_sys::File,
    pub preview_url: String,
}

impl PendingScreenshot {
    /// Создать черновик из `File` (object-URL для канвы).
    pub fn open(file: web_sys::File) -> Result<Self, ()> {
        let preview_url = web_sys::Url::create_object_url_with_blob(&file).map_err(|_| ())?;
        Ok(Self { file, preview_url })
    }

    /// Освободить object-URL превью.
    pub fn revoke(&self) {
        let _ = web_sys::Url::revoke_object_url(&self.preview_url);
    }
}

/// Проверка, что файл — изображение, пригодное для редактора.
pub fn is_editable_image(file: &web_sys::File) -> bool {
    is_editable_image_type(&file.type_())
}

/// Проверка MIME-типа изображения для редактора.
pub fn is_editable_image_type(mime: &str) -> bool {
    mime.starts_with("image/")
}
