use serde::{Deserialize, Serialize};

/// DTO для строки финансовой реализации OZON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceRealizationDto {
    // Composite Key (posting_number + sku + operation_type)
    pub posting_number: String,
    pub sku: String,

    // Metadata
    pub document_type: String,
    pub registrator_ref: String, // UUID источника данных

    // References
    pub connection_mp_ref: String,
    pub organization_ref: String,
    pub posting_ref: Option<String>, // Ссылка на a010_ozon_fbs_posting (UUID)

    // Даты
    pub accrual_date: String,           // Дата начисления
    pub sale_date: Option<String>,      // Дата продажи (из p900_sales_register)
    pub operation_date: Option<String>, // Дата операции
    pub delivery_date: Option<String>,  // Дата доставки

    // Информация о доставке
    pub delivery_schema: Option<String>, // Схема доставки (FBS/FBO)
    pub delivery_region: Option<String>, // Регион доставки
    pub delivery_city: Option<String>,   // Город доставки

    // Количество и суммы
    pub quantity: f64,
    pub price: Option<f64>,           // Цена товара
    pub amount: f64,                  // Сумма продажи
    pub commission_amount: Option<f64>, // Сумма комиссии
    pub commission_percent: Option<f64>, // Процент комиссии
    pub services_amount: Option<f64>,   // Сумма доп. услуг
    pub payout_amount: Option<f64>,     // Сумма к выплате

    // Тип операции
    pub operation_type: String,      // Тип операции
    pub operation_type_name: Option<String>, // Название типа операции
    pub is_return: bool,             // Флаг возврата (true если return_commission)

    // Валюта
    pub currency_code: Option<String>,

    // Технические поля
    pub loaded_at_utc: String,
    pub payload_version: i32,
    pub extra: Option<String>, // JSON для дополнительных полей
}

/// Запрос для получения списка финансовых данных с фильтрами
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceRealizationListRequest {
    // Обязательные фильтры
    pub date_from: String, // NaiveDate as string "YYYY-MM-DD"
    pub date_to: String,

    // Опциональные фильтры
    #[serde(default)]
    pub posting_number: Option<String>, // Поиск по posting_number
    #[serde(default)]
    pub sku: Option<String>, // Поиск по SKU
    #[serde(default)]
    pub connection_mp_ref: Option<String>,
    #[serde(default)]
    pub organization_ref: Option<String>,
    #[serde(default)]
    pub operation_type: Option<String>,
    #[serde(default)]
    pub is_return: Option<bool>, // Фильтр: возвраты или продажи
    #[serde(default)]
    pub has_posting_ref: Option<bool>, // Фильтр: есть ли ссылка на posting

    // Сортировка
    #[serde(default = "default_sort_by")]
    pub sort_by: String, // "accrual_date" | "posting_number" | "sku" | "amount"
    #[serde(default = "default_sort_desc")]
    pub sort_desc: bool,

    // Пагинация
    #[serde(default = "default_limit")]
    pub limit: i32,
    #[serde(default)]
    pub offset: i32,
}

fn default_sort_by() -> String {
    "accrual_date".to_string()
}

fn default_sort_desc() -> bool {
    true
}

fn default_limit() -> i32 {
    50
}

/// Ответ для списка финансовых данных
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceRealizationListResponse {
    pub items: Vec<OzonFinanceRealizationDto>,
    pub total_count: i32,
    pub has_more: bool,
}

/// Запрос для получения детальной информации по композитному ключу
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceRealizationByIdRequest {
    pub posting_number: String,
    pub sku: String,
    pub operation_type: String,
}

/// Ответ для детальной информации
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceRealizationByIdResponse {
    pub item: OzonFinanceRealizationDto,
}

/// Запрос для статистики по периоду
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceRealizationStatsRequest {
    pub date_from: String,
    pub date_to: String,
    #[serde(default)]
    pub connection_mp_ref: Option<String>,
}

/// Статистика за период
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OzonFinanceRealizationStatsResponse {
    pub total_rows: i32,
    pub total_quantity: f64,
    pub total_amount: f64,
    pub total_commission: f64,
    pub total_payout: f64,
    pub unique_postings: i32,
    pub linked_postings: i32, // Сколько постингов связано с a010
}
