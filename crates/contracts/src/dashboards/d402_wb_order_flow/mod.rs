use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbOrderFlowResponse {
    pub srid: String,
    pub order: Option<OrderFlowItem>,
    pub supply: Option<SupplyFlowItem>,
    pub sales: Vec<SaleFlowItem>,
    pub advert_campaigns: Vec<AdvertFlowItem>,
    pub total_advert_cost: f64,
    pub p903_rows: Vec<P903FlowItem>,
    pub claims: Vec<ClaimFlowItem>,
    /// Описание базовой номенклатуры (если у текущей заполнено base_nomenclature_ref).
    pub base_nomenclature_description: Option<String>,
    /// Описание номенклатуры заказа.
    pub nomenclature_description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimFlowItem {
    /// UUID агрегата (для открытия деталки).
    pub id: String,
    /// claim_id из WB API.
    pub claim_id: String,
    /// Числовой статус заявки (см. справочник WB).
    pub status: Option<i32>,
    /// Дата создания заявки `DD.MM.YYYY`.
    pub dt: String,
    /// Дата последнего обновления заявки `DD.MM.YYYY`, если есть.
    pub dt_update: Option<String>,
    /// Цена возврата по заявке.
    pub price: Option<f64>,
    /// Комментарий покупателя.
    pub user_comment: Option<String>,
    /// Признак нахождения в архиве.
    pub is_archive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P903FlowItem {
    pub id: String,
    pub rr_dt: String,
    pub supplier_oper_name: Option<String>,
    pub retail_price_withdisc_rub: Option<f64>,
    pub ppvz_for_pay: Option<f64>,
    pub ppvz_sales_commission: Option<f64>,
    pub acquiring_fee: Option<f64>,
    pub commission_percent: Option<f64>,
    pub delivery_rub: Option<f64>,
    pub penalty: Option<f64>,
    pub storage_fee: Option<f64>,
    pub rebill_logistic_cost: Option<f64>,
    pub additional_payment: Option<f64>,
    pub return_amount: Option<f64>,
    pub quantity: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderFlowItem {
    pub id: String,
    pub document_no: String,
    pub document_date: Option<String>,
    pub supplier_article: Option<String>,
    pub brand: Option<String>,
    pub subject: Option<String>,
    pub nm_id: Option<i64>,
    pub qty: Option<f64>,
    pub finished_price: Option<f64>,
    pub total_price: Option<f64>,
    pub price_with_disc: Option<f64>,
    pub spp: Option<f64>,
    pub dealer_price_ut: Option<f64>,
    pub income_id: Option<i64>,
    pub is_cancel: bool,
    pub is_supply: bool,
    pub is_realization: bool,
    pub is_posted: bool,
    pub warehouse_name: Option<String>,
    pub g_number: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupplyFlowItem {
    pub id: String,
    pub supply_id: String,
    pub supply_name: Option<String>,
    pub created_at_wb: Option<String>,
    pub closed_at_wb: Option<String>,
    pub is_done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleFlowItem {
    pub id: String,
    pub document_no: String,
    pub event_type: String,
    pub status_norm: String,
    pub sale_dt: String,
    pub is_posted: bool,
    pub is_customer_return: bool,
    pub warehouse_name: Option<String>,

    pub name: String,
    pub supplier_article: String,

    pub finished_price: Option<f64>,
    pub amount_line: Option<f64>,

    pub sell_out_plan: Option<f64>,
    pub commission_plan: Option<f64>,
    pub acquiring_fee_plan: Option<f64>,
    pub other_fee_plan: Option<f64>,
    pub supplier_payout_plan: Option<f64>,
    pub cost_of_production: Option<f64>,
    pub dealer_price_ut: Option<f64>,
    pub profit_plan: Option<f64>,

    pub sell_out_fact: Option<f64>,
    pub supplier_payout_fact: Option<f64>,
    pub profit_fact: Option<f64>,
    pub is_fact: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvertFlowItem {
    pub advert_id: i64,
    pub registrator_ref: String,
    pub document_date: String,
    pub allocated_cost: f64,
    pub campaign_name: Option<String>,
    pub campaign_status: Option<i32>,
    // Metrics from a026
    pub views: i64,
    pub clicks: i64,
    pub orders_reported: i64,
    pub total_spend: f64,
    pub ctr: f64,
    pub cpc: f64,
}
