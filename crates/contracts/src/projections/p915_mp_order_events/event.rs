//! Реестр событий заказа маркетплейса для проекции `p915_mp_order_events`.
//!
//! Каждое событие — точка в таймлайне заказа с датой (MSK), слоем-источником
//! и (опционально) суммой/товаром. Источники событий см. в комментариях ниже.

use serde::{Deserialize, Serialize};

/// Тип события заказа (реестр событий).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderEventType {
    /// Дата заказа — a013_ym_order.state.creation_date
    OrderPlaced,
    /// Дата отгрузки — TODO: источник пока отсутствует, событие не заполняется.
    Shipment,
    /// Дата доставки — a013_ym_order.state.delivery_date
    Delivery,
    /// Дата реализации — a034_ym_realization sales_lines, дата = header.document_date
    Realization,
    /// Дата возврата товара — a034_ym_realization return_lines, дата = header.document_date
    GoodsReturn,
    /// Дата оплаты — p907_ym_payment_report «Платёж покупателя», transaction_date
    Payment,
    /// Дата возврата оплаты — p907_ym_payment_report «Возврат платежа покупателя»
    PaymentReturn,
    /// Дата оплаты поставщику — перечисление YM по банковскому ордеру
    /// (a035_ym_settlement_recon при проведении), дата = bank_order_date.
    SupplierPayment,
    /// Дата возврата оплаты поставщику — YM удержал ранее перечисленную оплату
    /// при возврате товара (a035_ym_settlement_recon, строка «Возврат платежа
    /// покупателя»), дата = bank_order_date ордера-удержания.
    SupplierPaymentReturn,
}

impl OrderEventType {
    /// Каноническая строка события (хранится в БД, поле `event_type`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OrderPlaced => "order_placed",
            Self::Shipment => "shipment",
            Self::Delivery => "delivery",
            Self::Realization => "realization",
            Self::GoodsReturn => "goods_return",
            Self::Payment => "payment",
            Self::PaymentReturn => "payment_return",
            Self::SupplierPayment => "supplier_payment",
            Self::SupplierPaymentReturn => "supplier_payment_return",
        }
    }

    /// Разбор строки события (None — неизвестное значение).
    pub fn from_str(value: &str) -> Option<Self> {
        match value {
            "order_placed" => Some(Self::OrderPlaced),
            "shipment" => Some(Self::Shipment),
            "delivery" => Some(Self::Delivery),
            "realization" => Some(Self::Realization),
            "goods_return" => Some(Self::GoodsReturn),
            "payment" => Some(Self::Payment),
            "payment_return" => Some(Self::PaymentReturn),
            "supplier_payment" => Some(Self::SupplierPayment),
            "supplier_payment_return" => Some(Self::SupplierPaymentReturn),
            _ => None,
        }
    }

    /// Человекочитаемая RU-метка для UI.
    pub fn label_ru(self) -> &'static str {
        match self {
            Self::OrderPlaced => "Дата заказа",
            Self::Shipment => "Дата отгрузки",
            Self::Delivery => "Дата доставки",
            Self::Realization => "Дата реализации",
            Self::GoodsReturn => "Дата возврата товара",
            Self::Payment => "Дата оплаты",
            Self::PaymentReturn => "Дата возврата оплаты",
            Self::SupplierPayment => "Дата оплаты поставщику",
            Self::SupplierPaymentReturn => "Возврат оплаты поставщику",
        }
    }

    /// Все события реестра (для UI-легенды/фильтра).
    pub fn all() -> &'static [OrderEventType] {
        &[
            Self::OrderPlaced,
            Self::Shipment,
            Self::Delivery,
            Self::Realization,
            Self::GoodsReturn,
            Self::Payment,
            Self::PaymentReturn,
            Self::SupplierPayment,
            Self::SupplierPaymentReturn,
        ]
    }
}
