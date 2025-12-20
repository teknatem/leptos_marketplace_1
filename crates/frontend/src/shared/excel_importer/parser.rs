use wasm_bindgen::prelude::*;

/// JS binding для парсинга Excel файлов через SheetJS
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = parseExcelFile, catch)]
    pub fn parse_excel_file(data: &[u8]) -> Result<JsValue, JsValue>;
}

/// Парсит Excel файл и возвращает данные как Vec<Vec<String>>
pub async fn read_excel_from_file(file: web_sys::File) -> Result<Vec<Vec<String>>, String> {
    use wasm_bindgen_futures::JsFuture;

    // Читаем файл как ArrayBuffer
    let array_buffer = JsFuture::from(file.array_buffer())
        .await
        .map_err(|e| format!("Ошибка чтения файла: {:?}", e))?;

    // Конвертируем ArrayBuffer в Uint8Array
    let uint8_array = js_sys::Uint8Array::new(&array_buffer);
    let mut bytes = vec![0; uint8_array.length() as usize];
    uint8_array.copy_to(&mut bytes);

    // Парсим через JS функцию
    let result = parse_excel_file(&bytes).map_err(|e| format!("Ошибка парсинга Excel: {:?}", e))?;

    // Конвертируем JsValue в Vec<Vec<String>>
    parse_js_array_to_vec(result)
}

/// Конвертирует JS Array в Rust Vec<Vec<String>>
fn parse_js_array_to_vec(js_value: JsValue) -> Result<Vec<Vec<String>>, String> {
    if !js_value.is_array() {
        return Err("Результат парсинга не является массивом".to_string());
    }

    let array = js_sys::Array::from(&js_value);
    let mut result = Vec::new();

    for i in 0..array.length() {
        let row_value = array.get(i);

        if !row_value.is_array() {
            continue;
        }

        let row_array = js_sys::Array::from(&row_value);
        let mut row = Vec::new();

        for j in 0..row_array.length() {
            let cell = row_array.get(j);
            let cell_str = if cell.is_null() || cell.is_undefined() {
                String::new()
            } else {
                cell.as_string().unwrap_or_else(|| {
                    // Пытаемся преобразовать в строку
                    format!("{:?}", cell).trim_matches('"').to_string()
                })
            };
            row.push(cell_str);
        }

        result.push(row);
    }

    Ok(result)
}
