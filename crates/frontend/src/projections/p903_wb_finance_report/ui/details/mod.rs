use crate::domain::a012_wb_sales::ui::details::WbSalesDetail;
use crate::shared::icons::icon;
use crate::shared::json_viewer::widget::JsonViewer;
use crate::shared::list_utils::{format_number, get_sort_class, get_sort_indicator};
use crate::shared::page_frame::PageFrame;
use contracts::general_ledger::GeneralLedgerEntryDto;
use contracts::projections::p903_wb_finance_report::dto::{
    WbFinanceReportDetailResponse, WbFinanceReportDto,
};
use leptos::logging::log;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thaw::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

#[derive(Debug, Clone)]
struct FieldRow {
    description: String,
    field_id: String,
    value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GlFieldRole {
    Condition,
    Resource,
    ResourceAndCondition,
}

const EXCLUDED_PAYMENT_PROCESSING_VALUE: &str = "Комиссия за организацию платежа с НДС";

fn extra_string_field(item: &WbFinanceReportDto, field: &str) -> Option<String> {
    item.extra
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .and_then(|json| {
            json.get(field)
                .and_then(|value| value.as_str())
                .map(|value| value.trim().to_string())
        })
        .filter(|value| !value.is_empty())
}

fn extra_f64_field(item: &WbFinanceReportDto, field: &str) -> Option<f64> {
    item.extra
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .and_then(|json| {
            json.get(field).and_then(|value| {
                value.as_f64().or_else(|| {
                    value
                        .as_str()
                        .and_then(|raw| raw.trim().parse::<f64>().ok())
                })
            })
        })
}

fn field_note(field_id: &str) -> Option<&'static str> {
    match field_id {
        "supplier_oper_name" => Some(
            "Правило GL: поле выбирает ветку posting для customer_revenue/customer_return, mp_commission/mp_commission_adjustment, mp_storage, mp_penalty/mp_penalty_storno, voluntary_return_compensation и mp_ppvz_reward.",
        ),
        "srid" => Some(
            "Правило GL: непустой SRID делает строку linked. Это влияет на customer_revenue/customer_return, знаки расходов и posting acceptance только для строк без SRID.",
        ),
        "retail_amount" => Some(
            "Правило GL: ресурс для customer_revenue по linked sale-строке; если у возврата пустой return_amount, retail_amount используется как fallback для customer_return. Также участвует в определении sale-строки.",
        ),
        "return_amount" => Some(
            "Правило GL: ресурс для customer_return, если поле заполнено. Также участвует в определении return-строки и знака для mp_acquiring, mp_commission и mp_ppvz_reward.",
        ),
        "ppvz_vw" => Some(
            "Правило GL: часть комиссии WB. Для sale/return используется сумма ppvz_vw + ppvz_vw_nds в turnover mp_commission; для прочих операций вместе с ppvz_sales_commission формирует mp_commission_adjustment.",
        ),
        "ppvz_vw_nds" => Some(
            "Правило GL: часть комиссии WB. Для sale/return используется сумма ppvz_vw + ppvz_vw_nds в turnover mp_commission; для прочих операций вместе с ppvz_sales_commission формирует mp_commission_adjustment.",
        ),
        "ppvz_sales_commission" => Some(
            "Правило GL: участвует только в turnover mp_commission_adjustment для не sale/non-return операций; в sale/return ветке это поле в ресурс не входит.",
        ),
        "acquiring_fee" => Some(
            "Правило GL: ресурс для mp_acquiring. Для linked-возвратов сумма разворачивается в минус; строки с payment_processing = 'Комиссия за организацию платежа с НДС' исключаются.",
        ),
        "rebill_logistic_cost" => Some(
            "Правило GL: проводка формируется только по rebill_logistic_cost в turnover mp_rebill_logistic_cost.",
        ),
        "ppvz_reward" => Some(
            "Правило GL: источник — raw WB JSON field extra.ppvz_reward. Знак: Продажа = плюс, Возврат = минус, операция 'Возмещение за выдачу и возврат товаров на ПВЗ' = плюс. Turnover: mp_ppvz_reward.",
        ),
        "storage_fee" => Some(
            "Правило GL: ресурс для mp_storage только при supplier_oper_name = 'Хранение'. Знак зависит от linked/unlinked ветки.",
        ),
        "penalty" => Some(
            "Правило GL: ресурс для mp_penalty / mp_penalty_storno только при supplier_oper_name = 'Штраф'. Положительная сумма идет в mp_penalty, отрицательная — в mp_penalty_storno.",
        ),
        "ppvz_for_pay" => Some(
            "Правило GL: ресурс для voluntary_return_compensation только при supplier_oper_name = 'Добровольная компенсация при возврате'.",
        ),
        "delivery_amount" => {
            Some("Правило GL: ресурс для acceptance только для unlinked-строк без SRID.")
        }
        "payment_processing" => Some(
            "Значение показывается из raw WB JSON. Для GL mp_acquiring значение 'Комиссия за организацию платежа с НДС' исключается.",
        ),
        _ => None,
    }
}

fn is_emphasized_string_field(field_id: &str) -> bool {
    matches!(field_id, "payment_processing")
}

fn display_field_note(field_id: &str) -> Option<&'static str> {
    match field_id {
        _ => field_note(field_id),
    }
}

fn display_field_description(row: &FieldRow) -> String {
    match row.field_id.as_str() {
        "payment_processing" => "Тип обработки платежа".to_string(),
        "ppvz_reward" => "Возмещение за выдачу и возврат товаров на ПВЗ".to_string(),
        _ => row.description.clone(),
    }
}

fn gl_resource_turnovers(field_id: &str) -> &'static [&'static str] {
    match field_id {
        "retail_amount" => &["customer_revenue"],
        "return_amount" => &["customer_return"],
        "ppvz_vw" | "ppvz_vw_nds" => &["mp_commission", "mp_commission_adjustment"],
        "ppvz_sales_commission" => &["mp_commission_adjustment"],
        "acquiring_fee" => &["mp_acquiring"],
        "rebill_logistic_cost" => &["mp_rebill_logistic_cost"],
        "ppvz_reward" => &["mp_ppvz_reward"],
        "storage_fee" => &["mp_storage"],
        "penalty" => &["mp_penalty", "mp_penalty_storno"],
        "ppvz_for_pay" => &["voluntary_return_compensation"],
        "delivery_amount" => &["acceptance"],
        _ => &[],
    }
}

fn gl_condition_turnovers(field_id: &str) -> &'static [&'static str] {
    match field_id {
        "supplier_oper_name" => &[
            "customer_revenue",
            "customer_return",
            "mp_commission",
            "mp_commission_adjustment",
            "mp_acquiring",
            "mp_storage",
            "mp_penalty",
            "mp_penalty_storno",
            "voluntary_return_compensation",
            "mp_ppvz_reward",
        ],
        "srid" => &[
            "customer_revenue",
            "customer_return",
            "mp_commission",
            "mp_commission_adjustment",
            "mp_acquiring",
            "mp_rebill_logistic_cost",
            "mp_storage",
            "acceptance",
        ],
        "payment_processing" => &["mp_acquiring"],
        "retail_amount" => &[
            "customer_revenue",
            "customer_return",
            "mp_commission",
            "mp_ppvz_reward",
        ],
        "return_amount" => &[
            "customer_return",
            "mp_acquiring",
            "mp_commission",
            "mp_ppvz_reward",
        ],
        _ => &[],
    }
}

fn has_turnover(entries: &[GeneralLedgerEntryDto], turnover_code: &str) -> bool {
    entries
        .iter()
        .any(|entry| entry.turnover_code == turnover_code)
}

fn field_gl_role(field_id: &str, entries: &[GeneralLedgerEntryDto]) -> Option<GlFieldRole> {
    let is_resource = gl_resource_turnovers(field_id)
        .iter()
        .any(|turnover_code| has_turnover(entries, turnover_code));
    let is_condition = gl_condition_turnovers(field_id)
        .iter()
        .any(|turnover_code| has_turnover(entries, turnover_code));

    match (is_resource, is_condition) {
        (true, true) => Some(GlFieldRole::ResourceAndCondition),
        (true, false) => Some(GlFieldRole::Resource),
        (false, true) => Some(GlFieldRole::Condition),
        (false, false) => None,
    }
}

fn gl_role_badge_label(role: GlFieldRole) -> &'static str {
    match role {
        GlFieldRole::Condition => "Условие",
        GlFieldRole::Resource => "Ресурс",
        GlFieldRole::ResourceAndCondition => "Ресурс + условие",
    }
}

fn gl_role_badge_style(role: GlFieldRole) -> &'static str {
    match role {
        GlFieldRole::Condition => {
            "display: inline-flex; align-items: center; justify-content: center; padding: 3px 10px; border-radius: 999px; background: #0f766e; color: #f0fdfa; border: 1px solid rgba(255,255,255,0.14); font-size: var(--font-size-sm); font-weight: 700; line-height: 1.2;"
        }
        GlFieldRole::Resource => {
            "display: inline-flex; align-items: center; justify-content: center; padding: 3px 10px; border-radius: 999px; background: #9a3412; color: #fff7ed; border: 1px solid rgba(255,255,255,0.14); font-size: var(--font-size-sm); font-weight: 700; line-height: 1.2;"
        }
        GlFieldRole::ResourceAndCondition => {
            "display: inline-flex; align-items: center; justify-content: center; padding: 3px 10px; border-radius: 999px; background: #1d4ed8; color: #eff6ff; border: 1px solid rgba(255,255,255,0.14); font-size: var(--font-size-sm); font-weight: 700; line-height: 1.2;"
        }
    }
}

fn gl_role_row_style(role: GlFieldRole) -> &'static str {
    match role {
        GlFieldRole::Condition => {
            "background: rgba(15, 118, 110, 0.08); box-shadow: inset 3px 0 0 #0f766e;"
        }
        GlFieldRole::Resource => {
            "background: rgba(154, 52, 18, 0.08); box-shadow: inset 3px 0 0 #9a3412;"
        }
        GlFieldRole::ResourceAndCondition => {
            "background: rgba(29, 78, 216, 0.08); box-shadow: inset 3px 0 0 #1d4ed8;"
        }
    }
}

// Simplified WbSales structure for links display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesLink {
    pub id: String,
    pub header: WbSalesHeaderLink,
    pub line: WbSalesLineLink,
    pub state: WbSalesStateLink,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesHeaderLink {
    pub document_no: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesLineLink {
    pub nm_id: i64,
    pub supplier_article: String,
    pub name: String,
    pub qty: f64,
    pub total_price: Option<f64>,
    pub payment_sale_amount: Option<f64>,
    pub price_effective: Option<f64>,
    pub amount_line: Option<f64>,
    pub finished_price: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WbSalesStateLink {
    pub sale_dt: String,
}

#[component]
pub fn WbFinanceReportDetail(id: String, #[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let (data, set_data) = signal::<Option<WbFinanceReportDto>>(None);
    let (general_ledger_entries, set_general_ledger_entries) =
        signal::<Vec<GeneralLedgerEntryDto>>(Vec::new());
    let (loading, set_loading) = signal(true);
    let (posting, set_posting) = signal(false);
    let (error, set_error) = signal(None::<String>);
    let (action_message, set_action_message) = signal(None::<String>);
    let (active_tab, set_active_tab) = signal("fields");
    let (sort_by, set_sort_by) = signal("description".to_string());
    let (sort_desc, set_sort_desc) = signal(false);

    // Linked sales documents
    let (linked_sales, set_linked_sales) = signal::<Vec<WbSalesLink>>(Vec::new());
    let (links_loading, set_links_loading) = signal(false);
    let (links_error, set_links_error) = signal(None::<String>);
    let (selected_sale_id, set_selected_sale_id) = signal::<Option<String>>(None);

    // Загрузка данных
    let id_clone = id.clone();
    Effect::new(move || {
        let id = id_clone.clone();

        spawn_local(async move {
            match fetch_detail(&id).await {
                Ok(response) => {
                    set_action_message.set(None);
                    set_general_ledger_entries.set(response.general_ledger_entries);
                    set_data.set(Some(response.item));
                    set_loading.set(false);
                }
                Err(e) => {
                    log!("Failed to fetch finance report detail: {:?}", e);
                    set_error.set(Some(e));
                    set_loading.set(false);
                }
            }
        });
    });

    // Загрузка связанных документов продаж при активации вкладки Links
    Effect::new(move || {
        let tab = active_tab.get();
        if tab == "links" {
            if let Some(item) = data.get() {
                if let Some(srid_val) = item.srid {
                    if !srid_val.is_empty() {
                        set_links_loading.set(true);
                        set_links_error.set(None);

                        spawn_local(async move {
                            match fetch_linked_sales(&srid_val).await {
                                Ok(sales) => {
                                    set_linked_sales.set(sales);
                                    set_links_loading.set(false);
                                }
                                Err(e) => {
                                    log!("Failed to fetch linked sales: {:?}", e);
                                    set_links_error.set(Some(e));
                                    set_links_loading.set(false);
                                }
                            }
                        });
                    }
                }
            }
        }
    });

    // Преобразование данных в таблицу полей
    let get_field_rows = move || -> Vec<FieldRow> {
        let Some(item) = data.get() else {
            return Vec::new();
        };

        let mut rows = vec![
            FieldRow {
                description: "Эквайринг/Комиссии за организацию платежей".to_string(),
                field_id: "acquiring_fee".to_string(),
                value: item
                    .acquiring_fee
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Размер комиссии за эквайринг/Комиссии за организацию платежей, %"
                    .to_string(),
                field_id: "acquiring_percent".to_string(),
                value: item
                    .acquiring_percent
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Корректировка Вознаграждения Вайлдберриз (ВВ)".to_string(),
                field_id: "additional_payment".to_string(),
                value: item
                    .additional_payment
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Виды логистики, штрафов и корректировок ВВ".to_string(),
                field_id: "bonus_type_name".to_string(),
                value: item
                    .bonus_type_name
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Размер кВВ, %".to_string(),
                field_id: "commission_percent".to_string(),
                value: item
                    .commission_percent
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Количество доставок".to_string(),
                field_id: "delivery_amount".to_string(),
                value: item
                    .delivery_amount
                    .map(|v| format!("{:.0}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Услуги по доставке товара покупателю".to_string(),
                field_id: "delivery_rub".to_string(),
                value: item
                    .delivery_rub
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Артикул WB".to_string(),
                field_id: "nm_id".to_string(),
                value: item
                    .nm_id
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Общая сумма штрафов".to_string(),
                field_id: "penalty".to_string(),
                value: item
                    .penalty
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Вознаграждение Вайлдберриз (ВВ), без НДС".to_string(),
                field_id: "ppvz_vw".to_string(),
                value: item
                    .ppvz_vw
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "НДС с вознаграждения Вайлдберриз".to_string(),
                field_id: "ppvz_vw_nds".to_string(),
                value: item
                    .ppvz_vw_nds
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Вознаграждение с продаж до вычета услуг поверенного, без НДС"
                    .to_string(),
                field_id: "ppvz_sales_commission".to_string(),
                value: item
                    .ppvz_sales_commission
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Возмещение за выдачу и возврат товаров на ПВЗ (raw JSON)".to_string(),
                field_id: "ppvz_reward".to_string(),
                value: extra_f64_field(&item, "ppvz_reward")
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Количество".to_string(),
                field_id: "quantity".to_string(),
                value: item
                    .quantity
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Возмещение издержек по перевозке/по складским операциям с товаром"
                    .to_string(),
                field_id: "rebill_logistic_cost".to_string(),
                value: item
                    .rebill_logistic_cost
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Вайлдберриз реализовал Товар (Пр)".to_string(),
                field_id: "retail_amount".to_string(),
                value: item
                    .retail_amount
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Цена розничная".to_string(),
                field_id: "retail_price".to_string(),
                value: item
                    .retail_price
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Цена розничная с учётом согласованной скидки".to_string(),
                field_id: "retail_price_withdisc_rub".to_string(),
                value: item
                    .retail_price_withdisc_rub
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Количество возврата".to_string(),
                field_id: "return_amount".to_string(),
                value: item
                    .return_amount
                    .map(|v| format!("{:.0}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Дата операции".to_string(),
                field_id: "rr_dt".to_string(),
                value: item.rr_dt.clone(),
            },
            FieldRow {
                description: "Артикул продавца".to_string(),
                field_id: "sa_name".to_string(),
                value: item.sa_name.clone().unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Хранение".to_string(),
                field_id: "storage_fee".to_string(),
                value: item
                    .storage_fee
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Предмет".to_string(),
                field_id: "subject_name".to_string(),
                value: item.subject_name.clone().unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Обоснование для оплаты".to_string(),
                field_id: "supplier_oper_name".to_string(),
                value: item
                    .supplier_oper_name
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Сумма, удержанная за начисленные баллы программы лояльности"
                    .to_string(),
                field_id: "cashback_amount".to_string(),
                value: item
                    .cashback_amount
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "К перечислению продавцу за реализованный товар".to_string(),
                field_id: "ppvz_for_pay".to_string(),
                value: item
                    .ppvz_for_pay
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Итоговый кВВ без НДС, %".to_string(),
                field_id: "ppvz_kvw_prc".to_string(),
                value: item
                    .ppvz_kvw_prc
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Размер кВВ без НДС, % базовый".to_string(),
                field_id: "ppvz_kvw_prc_base".to_string(),
                value: item
                    .ppvz_kvw_prc_base
                    .map(|v| format!("{:.2}", v))
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "Признак услуги платной доставки".to_string(),
                field_id: "srv_dbs".to_string(),
                value: item
                    .srv_dbs
                    .map(|v| {
                        if v == 1 {
                            "Да".to_string()
                        } else {
                            "Нет".to_string()
                        }
                    })
                    .unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "SRID (Уникальный идентификатор строки)".to_string(),
                field_id: "srid".to_string(),
                value: item.srid.clone().unwrap_or_else(|| "-".to_string()),
            },
            FieldRow {
                description: "ID".to_string(),
                field_id: "id".to_string(),
                value: item.id.clone(),
            },
            FieldRow {
                description: "Source row reference".to_string(),
                field_id: "source_row_ref".to_string(),
                value: item.source_row_ref.clone(),
            },
            FieldRow {
                description: "General ledger entries count".to_string(),
                field_id: "general_ledger_entries_count".to_string(),
                value: item.general_ledger_entries_count.to_string(),
            },
        ];

        // Сортировка
        rows.push(FieldRow {
            description: format!(
                "Тип обработки платежа. Примечание: значение '{}' исключается из GL-проводки mp_acquiring.",
                EXCLUDED_PAYMENT_PROCESSING_VALUE
            ),
            field_id: "payment_processing".to_string(),
            value: extra_string_field(&item, "payment_processing")
                .unwrap_or_else(|| "-".to_string()),
        });

        let sort_field = sort_by.get();
        let is_desc = sort_desc.get();

        rows.sort_by(|a, b| {
            let cmp = match &*sort_field {
                "field_id" => a.field_id.cmp(&b.field_id),
                "value" => a.value.cmp(&b.value),
                _ => a.description.cmp(&b.description),
            };
            if is_desc {
                cmp.reverse()
            } else {
                cmp
            }
        });

        rows
    };

    let handle_column_sort = move |column: &'static str| {
        let current_sort = sort_by.get();
        if current_sort == column {
            set_sort_desc.set(!sort_desc.get());
        } else {
            set_sort_by.set(column.to_string());
            set_sort_desc.set(false);
        }
    };

    // Экспорт в Excel
    let export_to_excel = move || {
        let field_rows = get_field_rows();
        if field_rows.is_empty() {
            log!("No data to export");
            return;
        }

        // UTF-8 BOM для правильного отображения кириллицы в Excel
        let mut csv = String::from("\u{FEFF}");

        // Заголовок с точкой с запятой как разделитель
        csv.push_str("Описание;Идентификатор;Значение\n");

        for row in field_rows {
            csv.push_str(&format!(
                "\"{}\";\"{}\";\"{}\"\n",
                row.description.replace('\"', "\"\""),
                row.field_id.replace('\"', "\"\""),
                row.value.replace('\"', "\"\"")
            ));
        }

        // Создаем Blob с CSV данными
        use js_sys::Array;
        use wasm_bindgen::JsValue;

        let array = Array::new();
        array.push(&JsValue::from_str(&csv));

        let blob_props = web_sys::BlobPropertyBag::new();
        blob_props.set_type("text/csv;charset=utf-8;");

        if let Ok(blob) = web_sys::Blob::new_with_str_sequence_and_options(&array, &blob_props) {
            if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                if let Some(window) = web_sys::window() {
                    if let Some(document) = window.document() {
                        if let Ok(a) = document.create_element("a") {
                            let a: web_sys::HtmlAnchorElement = a.unchecked_into();
                            a.set_href(&url);
                            let filename = format!(
                                "wb_finance_report_detail_{}.csv",
                                chrono::Utc::now().format("%Y%m%d_%H%M%S")
                            );
                            a.set_download(&filename);
                            let _ = a.click();
                            let _ = web_sys::Url::revoke_object_url(&url);
                        }
                    }
                }
            }
        }
    };

    let post_click = move |_| {
        let id = id.clone();
        set_posting.set(true);
        set_action_message.set(None);

        spawn_local(async move {
            match post_detail(&id).await {
                Ok(response) => {
                    set_general_ledger_entries.set(response.general_ledger_entries);
                    set_data.set(Some(response.item));
                    set_active_tab.set("general_ledger");
                    set_action_message.set(Some("General Ledger rebuilt.".to_string()));
                }
                Err(e) => {
                    log!("Failed to rebuild p903 general ledger: {:?}", e);
                    set_action_message.set(Some(format!("Post failed: {e}")));
                }
            }
            set_posting.set(false);
        });
    };

    view! {
        <PageFrame page_id="p903_wb_finance_report--detail" category="detail">
            <div class="modal-header">
                <h3 class="modal-title">"WB Finance Report Details"</h3>
                <Button
                    appearance=ButtonAppearance::Primary
                    on_click=post_click
                    disabled=Signal::derive(move || loading.get() || posting.get())
                >
                    <span>{vec![icon("refresh").into_view()]}</span>
                    <span>{move || if posting.get() { " Проведение..." } else { " Post" }}</span>
                </Button>
                <Button
                    appearance=ButtonAppearance::Secondary
                    on_click=move |_| on_close.run(())
                >
                    <span>{vec![icon("x").into_view()]}</span>
                    <span>" Закрыть"</span>
                </Button>
            </div>

            <div class="page__content">
            {move || {
                action_message.get().map(|message| {
                    view! {
                        <div class="warning-box" style="margin-bottom: var(--spacing-md);">
                            <span class="warning-box__text">{message}</span>
                        </div>
                    }
                })
            }}
            {move || {
                if loading.get() {
                    view! { <p class="text-muted">"Загрузка..."</p> }.into_any()
                } else if let Some(err) = error.get() {
                    view! {
                        <div class="warning-box warning-box--error">
                            <span class="warning-box__icon">"⚠"</span>
                            <span class="warning-box__text">{err}</span>
                        </div>
                    }
                        .into_any()
                } else if data.get().is_some() {
                    view! {
                        <div>
                            // Tabs and Export button
                            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: var(--spacing-md);">
                                <div class="detail-tabs">
                                    <button
                                        class=move || if active_tab.get() == "fields" {
                                            "detail-tabs__item detail-tabs__item--active"
                                        } else {
                                            "detail-tabs__item"
                                        }
                                        on:click=move |_| set_active_tab.set("fields")
                                    >
                                        "Fields"
                                    </button>
                                    <button
                                        class=move || if active_tab.get() == "json" {
                                            "detail-tabs__item detail-tabs__item--active"
                                        } else {
                                            "detail-tabs__item"
                                        }
                                        on:click=move |_| set_active_tab.set("json")
                                    >
                                        "Raw JSON"
                                    </button>
                                    <button
                                        class=move || if active_tab.get() == "links" {
                                            "detail-tabs__item detail-tabs__item--active"
                                        } else {
                                            "detail-tabs__item"
                                        }
                                        on:click=move |_| set_active_tab.set("links")
                                    >
                                        "Links"
                                    </button>
                                    <button
                                        class=move || if active_tab.get() == "general_ledger" {
                                            "detail-tabs__item detail-tabs__item--active"
                                        } else {
                                            "detail-tabs__item"
                                        }
                                        on:click=move |_| set_active_tab.set("general_ledger")
                                    >
                                        "General Ledger"
                                    </button>
                                </div>
                                {
                                    let export_excel = export_to_excel.clone();
                                    view! {
                                        <Button
                                            appearance=ButtonAppearance::Primary
                                            on_click=move |_| export_excel()
                                        >
                                            <span>{vec![icon("download").into_view()]}</span>
                                            <span>" Export Excel"</span>
                                        </Button>
                                    }
                                }
                            </div>

                            // Tab Content
                            {move || {
                                if active_tab.get() == "fields" {
                                    let field_rows = get_field_rows();
                                    let gl_entries = general_ledger_entries.get();
                                    view! {
                                        <div style="width: 100%; overflow-x: hidden;">
                                            <Table attr:style="width: 100%; table-layout: fixed;">
                                                <TableHeader>
                                                    <TableRow>
                                                        <TableHeaderCell resizable=true min_width=240.0>
                                                            "Описание"
                                                            <span
                                                                class={move || get_sort_class("description", &sort_by.get())}
                                                                on:click=move |_| handle_column_sort("description")
                                                            >
                                                                {move || get_sort_indicator("description", &sort_by.get(), !sort_desc.get())}
                                                            </span>
                                                        </TableHeaderCell>
                                                        <TableHeaderCell resizable=true min_width=120.0>
                                                            "Роль"
                                                        </TableHeaderCell>
                                                        <TableHeaderCell resizable=true min_width=150.0>
                                                            "Идентификатор"
                                                            <span
                                                                class={move || get_sort_class("field_id", &sort_by.get())}
                                                                on:click=move |_| handle_column_sort("field_id")
                                                            >
                                                                {move || get_sort_indicator("field_id", &sort_by.get(), !sort_desc.get())}
                                                            </span>
                                                        </TableHeaderCell>
                                                        <TableHeaderCell resizable=true min_width=160.0>
                                                            "Значение"
                                                            <span
                                                                class={move || get_sort_class("value", &sort_by.get())}
                                                                on:click=move |_| handle_column_sort("value")
                                                            >
                                                                {move || get_sort_indicator("value", &sort_by.get(), !sort_desc.get())}
                                                            </span>
                                                        </TableHeaderCell>
                                                    </TableRow>
                                                </TableHeader>
                                                <TableBody>
                                                    {field_rows
                                                        .into_iter()
                                                        .map(|row| {
                                                            let description = display_field_description(&row);
                                                            let note = display_field_note(&row.field_id).map(str::to_string);
                                                            let gl_role = field_gl_role(&row.field_id, &gl_entries);
                                                            let emphasized_value =
                                                                is_emphasized_string_field(&row.field_id);
                                                            view! {
                                                                <TableRow attr:style={gl_role.map(gl_role_row_style).unwrap_or("")}>
                                                                    <TableCell>
                                                                        <TableCellLayout>
                                                                            <div style="color: var(--color-text-primary); font-weight: 600; line-height: 1.35; overflow-wrap: anywhere;">
                                                                                {description}
                                                                            </div>
                                                                            {note.clone().map(|note_text| view! {
                                                                                <div style="margin-top: 6px; font-size: var(--font-size-sm); color: var(--color-text-secondary); line-height: 1.4; overflow-wrap: anywhere;">
                                                                                    {note_text}
                                                                                </div>
                                                                            })}
                                                                        </TableCellLayout>
                                                                    </TableCell>
                                                                    <TableCell>
                                                                        <TableCellLayout>
                                                                            <div style="display: flex; justify-content: center; align-items: center; min-height: 100%;">
                                                                                {if let Some(role) = gl_role {
                                                                                    view! {
                                                                                        <span style={gl_role_badge_style(role)}>
                                                                                            {gl_role_badge_label(role)}
                                                                                        </span>
                                                                                    }.into_any()
                                                                                } else {
                                                                                    view! {
                                                                                        <span style="font-size: var(--font-size-sm); color: var(--color-text-secondary); font-weight: 600;">
                                                                                            "—"
                                                                                        </span>
                                                                                    }.into_any()
                                                                                }}
                                                                            </div>
                                                                        </TableCellLayout>
                                                                    </TableCell>
                                                                    <TableCell>
                                                                        <TableCellLayout>
                                                                            <span style="font-size: var(--font-size-sm); color: var(--color-text-primary); font-weight: 600; overflow-wrap: anywhere; word-break: break-word;">
                                                                                {row.field_id.clone()}
                                                                            </span>
                                                                        </TableCellLayout>
                                                                    </TableCell>
                                                                    <TableCell>
                                                                        <TableCellLayout>
                                                                            {if emphasized_value {
                                                                                view! {
                                                                                    <code style="display: inline-block; padding: 2px 6px; border-radius: 6px; background: var(--color-bg-secondary); color: var(--color-text-primary); border: 1px solid var(--color-border); font-size: var(--font-size-sm); white-space: normal; overflow-wrap: anywhere; word-break: break-word;">
                                                                                        {row.value.clone()}
                                                                                    </code>
                                                                                }.into_any()
                                                                            } else {
                                                                                view! {
                                                                                    <span style="color: var(--color-text-primary); font-weight: 600; white-space: normal; overflow-wrap: anywhere; word-break: break-word;">
                                                                                        {row.value.clone()}
                                                                                    </span>
                                                                                }.into_any()
                                                                            }}
                                                                        </TableCellLayout>
                                                                    </TableCell>
                                                                </TableRow>
                                                            }
                                                            .into_view()
                                                        })
                                                        .collect_view()}
                                                </TableBody>
                                            </Table>
                                        </div>
                                    }
                                        .into_any()
                                } else if active_tab.get() == "json" {
                                    let json_content = data
                                        .get()
                                        .and_then(|d| d.extra)
                                        .unwrap_or_else(|| "{}".to_string());
                                    view! {
                                        <JsonViewer
                                            json_content=json_content
                                            title="Raw JSON from WB".to_string()
                                        />
                                    }
                                        .into_any()
                                } else if active_tab.get() == "general_ledger" {
                                    let entries = general_ledger_entries.get();
                                    if entries.is_empty() {
                                        view! { <p class="text-muted">"Нет связанных записей general ledger."</p> }.into_any()
                                    } else {
                                        view! {
                                            <div>
                                                <div style="padding: 10px; margin-bottom: 10px; background: var(--color-bg-secondary); border: 1px solid var(--color-border); border-radius: var(--radius-md); display: flex; gap: 20px; flex-wrap: wrap; font-size: var(--font-size-sm); font-weight: 600;">
                                                    <span>"Entries: " {entries.len()}</span>
                                                    <span>"ID: " {data.get().map(|item| item.id).unwrap_or_default()}</span>
                                                    <span>"Source: " {data.get().map(|item| item.source_row_ref).unwrap_or_default()}</span>
                                                </div>

                                                <div style="width: 100%; overflow-x: auto;">
                                                    <Table attr:style="width: 100%;">
                                                        <TableHeader>
                                                            <TableRow>
                                                                <TableHeaderCell resizable=true min_width=120.0>"Entry Date"</TableHeaderCell>
                                                                <TableHeaderCell resizable=true min_width=140.0>"Turnover"</TableHeaderCell>
                                                                <TableHeaderCell resizable=true min_width=90.0>"Amount"</TableHeaderCell>
                                                                <TableHeaderCell resizable=true min_width=120.0>"Resource"</TableHeaderCell>
                                                                <TableHeaderCell resizable=true min_width=80.0>"Sign"</TableHeaderCell>
                                                                <TableHeaderCell resizable=true min_width=120.0>"Debit"</TableHeaderCell>
                                                                <TableHeaderCell resizable=true min_width=120.0>"Credit"</TableHeaderCell>
                                                            </TableRow>
                                                        </TableHeader>
                                                        <TableBody>
                                                            {entries
                                                                .into_iter()
                                                                .map(|entry| {
                                                                    view! {
                                                                        <TableRow>
                                                                            <TableCell><TableCellLayout>{entry.entry_date}</TableCellLayout></TableCell>
                                                                            <TableCell><TableCellLayout truncate=true>{entry.turnover_code}</TableCellLayout></TableCell>
                                                                            <TableCell class="table__cell--right"><TableCellLayout>{format_number(entry.amount)}</TableCellLayout></TableCell>
                                                                            <TableCell><TableCellLayout truncate=true>{entry.resource_field}</TableCellLayout></TableCell>
                                                                            <TableCell class="table__cell--right"><TableCellLayout>{entry.resource_sign}</TableCellLayout></TableCell>
                                                                            <TableCell><TableCellLayout>{entry.debit_account}</TableCellLayout></TableCell>
                                                                            <TableCell><TableCellLayout>{entry.credit_account}</TableCellLayout></TableCell>
                                                                        </TableRow>
                                                                    }
                                                                    .into_view()
                                                                })
                                                                .collect_view()}
                                                        </TableBody>
                                                    </Table>
                                                </div>
                                            </div>
                                        }
                                        .into_any()
                                    }
                                } else if active_tab.get() == "links" {
                                    if links_loading.get() {
                                        view! { <p class="text-muted">"Загрузка связанных документов..."</p> }.into_any()
                                    } else if let Some(err) = links_error.get() {
                                        view! {
                                            <div class="warning-box warning-box--error">
                                                <span class="warning-box__icon">"⚠"</span>
                                                <span class="warning-box__text">"Error loading links: " {err}</span>
                                            </div>
                                        }
                                            .into_any()
                                    } else {
                                        let sales = linked_sales.get();
                                        if sales.is_empty() {
                                            view! { <p class="text-muted">"Нет связанных документов продаж для данного SRID."</p> }.into_any()
                                        } else {
                                            let total_qty: f64 = sales.iter().map(|s| s.line.qty).sum();
                                            let total_total_price: f64 = sales.iter().filter_map(|s| s.line.total_price).sum();
                                            let total_payment: f64 = sales.iter().filter_map(|s| s.line.payment_sale_amount).sum();
                                            let total_amount: f64 = sales.iter().filter_map(|s| s.line.amount_line).sum();
                                            let total_finished: f64 = sales.iter().filter_map(|s| s.line.finished_price).sum();

                                            view! {
                                                <div>
                                                    <div style="padding: 10px; margin-bottom: 10px; background: var(--color-bg-secondary); border: 1px solid var(--color-border); border-radius: var(--radius-md); display: flex; gap: 20px; flex-wrap: wrap; font-size: var(--font-size-sm); font-weight: 600;">
                                                        <span>"Найдено: " {sales.len()} " документов"</span>
                                                        <span>"Total Qty: " {format_number(total_qty)}</span>
                                                        <span>"Total Price: " {format_number(total_total_price)}</span>
                                                        <span>"Payment: " {format_number(total_payment)}</span>
                                                        <span>"Amount: " {format_number(total_amount)}</span>
                                                        <span>"Finished: " {format_number(total_finished)}</span>
                                                    </div>

                                                    <div style="width: 100%; overflow-x: auto;">
                                                        <Table attr:style="width: 100%;">
                                                            <TableHeader>
                                                                <TableRow>
                                                                    <TableHeaderCell resizable=true min_width=100.0>"Date"</TableHeaderCell>
                                                                    <TableHeaderCell resizable=true min_width=120.0>"Document No"</TableHeaderCell>
                                                                    <TableHeaderCell resizable=true min_width=80.0>"NM ID"</TableHeaderCell>
                                                                    <TableHeaderCell resizable=true min_width=120.0>"Supplier Article"</TableHeaderCell>
                                                                    <TableHeaderCell resizable=true min_width=200.0>"Name"</TableHeaderCell>
                                                                    <TableHeaderCell resizable=true min_width=70.0>"Qty"</TableHeaderCell>
                                                                    <TableHeaderCell resizable=true min_width=100.0>"Total Price"</TableHeaderCell>
                                                                    <TableHeaderCell resizable=true min_width=100.0>"Payment"</TableHeaderCell>
                                                                    <TableHeaderCell resizable=true min_width=100.0>"Price Effective"</TableHeaderCell>
                                                                    <TableHeaderCell resizable=true min_width=100.0>"Amount Line"</TableHeaderCell>
                                                                    <TableHeaderCell resizable=true min_width=100.0>"Finished Price"</TableHeaderCell>
                                                                </TableRow>
                                                            </TableHeader>
                                                            <TableBody>
                                                                {sales
                                                                    .into_iter()
                                                                    .map(|sale| {
                                                                        let sale_id = sale.id.clone();
                                                                        view! {
                                                                            <TableRow on:click=move |_| set_selected_sale_id.set(Some(sale_id.clone()))>
                                                                                <TableCell><TableCellLayout>{sale.state.sale_dt}</TableCellLayout></TableCell>
                                                                                <TableCell><TableCellLayout>{sale.header.document_no}</TableCellLayout></TableCell>
                                                                                <TableCell><TableCellLayout>{sale.line.nm_id}</TableCellLayout></TableCell>
                                                                                <TableCell><TableCellLayout truncate=true>{sale.line.supplier_article}</TableCellLayout></TableCell>
                                                                                <TableCell><TableCellLayout truncate=true>{sale.line.name}</TableCellLayout></TableCell>
                                                                                <TableCell class="table__cell--right"><TableCellLayout>{format_number(sale.line.qty)}</TableCellLayout></TableCell>
                                                                                <TableCell class="table__cell--right"><TableCellLayout>{sale.line.total_price.map(|v| format_number(v)).unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                                                <TableCell class="table__cell--right"><TableCellLayout>{sale.line.payment_sale_amount.map(|v| format_number(v)).unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                                                <TableCell class="table__cell--right"><TableCellLayout>{sale.line.price_effective.map(|v| format_number(v)).unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                                                <TableCell class="table__cell--right"><TableCellLayout>{sale.line.amount_line.map(|v| format_number(v)).unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                                                <TableCell class="table__cell--right"><TableCellLayout>{sale.line.finished_price.map(|v| format_number(v)).unwrap_or_else(|| "-".to_string())}</TableCellLayout></TableCell>
                                                                            </TableRow>
                                                                        }
                                                                        .into_view()
                                                                    })
                                                                    .collect_view()}
                                                            </TableBody>
                                                        </Table>
                                                    </div>
                                                </div>
                                            }.into_any()
                                        }
                                    }
                                } else {
                                    view! { <div></div> }.into_any()
                                }
                            }}

                        </div>
                    }
                        .into_any()
                } else {
                    view! { <p>"No data"</p> }.into_any()
                }
            }}
            </div>

            // Modal for WbSalesDetail when clicking on a linked sale
            {move || {
                if let Some(sale_id) = selected_sale_id.get() {
                    view! {
                        <div class="modal-overlay" style="z-index: 2000;">
                            <div class="modal modal-content-wide">
                                <WbSalesDetail
                                    id=sale_id.clone()
                                    on_close=move || set_selected_sale_id.set(None)
                                />
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! { <div></div> }.into_any()
                }
            }}
        </PageFrame>
    }
}

async fn fetch_detail(id: &str) -> Result<WbFinanceReportDetailResponse, String> {
    let window = web_sys::window().ok_or("No window object")?;
    let url = format!("/api/p903/finance-report/by-id/{}", urlencoding::encode(id));

    let resp_value = JsFuture::from(window.fetch_with_str(&url))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| "Failed to cast to Response")?;

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    let json = JsFuture::from(resp.json().map_err(|_| "Failed to get JSON")?)
        .await
        .map_err(|e| format!("Failed to parse JSON: {:?}", e))?;

    serde_wasm_bindgen::from_value(json).map_err(|e| format!("Failed to deserialize: {:?}", e))
}

async fn post_detail(id: &str) -> Result<WbFinanceReportDetailResponse, String> {
    use web_sys::{Request, RequestInit, RequestMode};

    let window = web_sys::window().ok_or("No window object")?;
    let url = format!(
        "/api/p903/finance-report/by-id/{}/post",
        urlencoding::encode(id)
    );

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("Request init failed: {:?}", e))?;

    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| "Failed to cast to Response")?;

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    let json = JsFuture::from(resp.json().map_err(|_| "Failed to get JSON")?)
        .await
        .map_err(|e| format!("Failed to parse JSON: {:?}", e))?;

    serde_wasm_bindgen::from_value(json).map_err(|e| format!("Failed to deserialize: {:?}", e))
}

async fn fetch_linked_sales(srid: &str) -> Result<Vec<WbSalesLink>, String> {
    let window = web_sys::window().ok_or("No window object")?;
    let url = format!("/api/a012/wb-sales/search-by-srid?srid={}", srid);

    let resp_value = JsFuture::from(window.fetch_with_str(&url))
        .await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;

    let resp: web_sys::Response = resp_value
        .dyn_into()
        .map_err(|_| "Failed to cast to Response")?;

    if !resp.ok() {
        return Err(format!("HTTP error: {}", resp.status()));
    }

    let json = JsFuture::from(resp.json().map_err(|_| "Failed to get JSON")?)
        .await
        .map_err(|e| format!("Failed to parse JSON: {:?}", e))?;

    serde_wasm_bindgen::from_value(json).map_err(|e| format!("Failed to deserialize: {:?}", e))
}
