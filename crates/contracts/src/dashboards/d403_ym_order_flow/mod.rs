use serde::{Deserialize, Serialize};

/// Полная история YM-заказа: строки реализации (a034) + платёжные транзакции (p907),
/// собранные по номеру заказа (`order_id`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderFlowResponse {
    /// Номер заказа YM (a013_ym_order.document_no).
    pub order_no: String,
    /// Сам заказ a013 (если найден) — первое событие ленты.
    #[serde(default)]
    pub order: Option<YmOrderFlowItem>,
    /// Строки официальной реализации a034 по этому заказу.
    pub realizations: Vec<YmRealizationFlowItem>,
    /// Платёжные транзакции p907 по этому заказу.
    pub payments: Vec<YmPaymentFlowItem>,
    /// Возвраты a016_ym_returns по этому заказу (если есть).
    #[serde(default)]
    pub returns: Vec<YmReturnFlowItem>,
}

/// Сам заказ a013 по номеру — для строки «Заказ» в ленте истории.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmOrderFlowItem {
    /// UUID документа a013 (для открытия деталки).
    pub id: String,
    pub document_no: String,
    /// Дата создания заказа `DD.MM.YYYY`.
    pub order_date: Option<String>,
    /// Нормализованный статус заказа (DELIVERED/RECEIVED/…).
    pub status: Option<String>,
    /// Дата доставки `DD.MM.YYYY`.
    pub delivery_date: Option<String>,
    /// Суммарное количество по строкам заказа.
    pub qty: f64,
    /// Платёж покупателя (itemsTotal).
    pub items_total: Option<f64>,
    /// Общая сумма заказа (total).
    pub total_amount: Option<f64>,
    pub is_posted: bool,
}

/// Возврат a016_ym_returns по заказу — строка «Возврат YM» в ленте истории.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmReturnFlowItem {
    /// UUID документа a016 (для открытия деталки).
    pub id: String,
    pub return_id: i64,
    /// Тип возврата: RETURN / UNREDEEMED.
    pub return_type: String,
    /// Статус возврата денег (REFUNDED/REJECTED/…).
    pub refund_status: String,
    /// Дата создания возврата в источнике `DD.MM.YYYY`.
    pub created_at_source: Option<String>,
    /// Общая сумма возврата (header.amount).
    pub amount: f64,
    /// Суммарное количество товаров в возврате.
    pub qty: i32,
}

/// Одна строка реализации a034 (продажа или возврат) по заказу.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmRealizationFlowItem {
    /// UUID документа a034 (для открытия деталки).
    pub doc_id: String,
    /// Номер документа реализации.
    pub document_no: String,
    /// Дата документа `YYYY-MM-DD`.
    pub document_date: String,
    pub shop_sku: String,
    pub offer_name: String,
    pub quantity: f64,
    /// Сумма выручки (положительная и для продаж, и для возвратов).
    pub revenue_amount: f64,
    /// true — возврат (уменьшает выручку), false — продажа.
    pub is_return: bool,
}

/// Одна платёжная транзакция p907 по заказу.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YmPaymentFlowItem {
    /// UUID записи p907 (для открытия деталки).
    pub id: String,
    pub transaction_date: Option<String>,
    pub transaction_type: Option<String>,
    pub transaction_id: Option<String>,
    pub transaction_sum: Option<f64>,
    pub bank_sum: Option<f64>,
    pub payment_status: Option<String>,
    pub transaction_source: Option<String>,
    pub shop_sku: Option<String>,
    pub offer_or_service_name: Option<String>,
    pub count: Option<i32>,
    pub comments: Option<String>,
}
