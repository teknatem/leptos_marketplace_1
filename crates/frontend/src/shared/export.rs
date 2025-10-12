/// Универсальный модуль для экспорта данных в Excel/CSV формат
use wasm_bindgen::JsCast;
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

/// Trait для типов, которые могут быть экспортированы в Excel
pub trait ExcelExportable {
    /// Возвращает массив заголовков колонок
    fn headers() -> Vec<&'static str>;

    /// Преобразует объект в массив значений для CSV
    fn to_csv_row(&self) -> Vec<String>;
}

/// Экспортирует список данных в CSV файл и инициирует скачивание
pub fn export_to_excel<T: ExcelExportable>(data: &[T], filename: &str) -> Result<(), String> {
    if data.is_empty() {
        return Err("Нет данных для экспорта".to_string());
    }

    // Создаем CSV контент
    let mut csv_content = String::new();

    // Добавляем UTF-8 BOM для корректного отображения кириллицы в Excel
    csv_content.push('\u{FEFF}');

    // Добавляем заголовки
    let headers = T::headers();
    csv_content.push_str(&headers.join(";"));
    csv_content.push('\n');

    // Добавляем данные
    for item in data {
        let row = item.to_csv_row();
        // Экранируем значения, содержащие разделители или кавычки
        let escaped_row: Vec<String> = row
            .iter()
            .map(|cell| escape_csv_cell(cell))
            .collect();
        csv_content.push_str(&escaped_row.join(";"));
        csv_content.push('\n');
    }

    // Создаем Blob с CSV данными
    let blob = create_csv_blob(&csv_content)?;

    // Скачиваем файл
    download_blob(&blob, filename)?;

    Ok(())
}

/// Экранирует CSV ячейку если необходимо
fn escape_csv_cell(cell: &str) -> String {
    // Если ячейка содержит разделитель (;), кавычки (") или перевод строки, оборачиваем в кавычки
    if cell.contains(';') || cell.contains('"') || cell.contains('\n') || cell.contains('\r') {
        // Удваиваем кавычки внутри значения
        let escaped = cell.replace('"', "\"\"");
        format!("\"{}\"", escaped)
    } else {
        cell.to_string()
    }
}

/// Создает Blob объект с CSV данными
fn create_csv_blob(content: &str) -> Result<Blob, String> {
    let array = js_sys::Array::new();
    array.push(&wasm_bindgen::JsValue::from_str(content));

    let properties = BlobPropertyBag::new();
    properties.set_type("text/csv;charset=utf-8;");

    Blob::new_with_str_sequence_and_options(&array, &properties)
        .map_err(|e| format!("Failed to create blob: {:?}", e))
}

/// Инициирует скачивание Blob через браузер
fn download_blob(blob: &Blob, filename: &str) -> Result<(), String> {
    let window = web_sys::window().ok_or("No window object")?;
    let document = window.document().ok_or("No document object")?;

    // Создаем URL для blob
    let url = Url::create_object_url_with_blob(blob)
        .map_err(|e| format!("Failed to create object URL: {:?}", e))?;

    // Создаем временную ссылку для скачивания
    let anchor = document
        .create_element("a")
        .map_err(|e| format!("Failed to create anchor: {:?}", e))?
        .dyn_into::<HtmlAnchorElement>()
        .map_err(|e| format!("Failed to cast to anchor: {:?}", e))?;

    anchor.set_href(&url);
    anchor.set_download(filename);
    anchor.style().set_property("display", "none")
        .map_err(|e| format!("Failed to set style: {:?}", e))?;

    // Добавляем в DOM, кликаем и удаляем
    document
        .body()
        .ok_or("No body element")?
        .append_child(&anchor)
        .map_err(|e| format!("Failed to append anchor: {:?}", e))?;

    anchor.click();

    document
        .body()
        .ok_or("No body element")?
        .remove_child(&anchor)
        .map_err(|e| format!("Failed to remove anchor: {:?}", e))?;

    // Освобождаем URL
    Url::revoke_object_url(&url)
        .map_err(|e| format!("Failed to revoke URL: {:?}", e))?;

    Ok(())
}
