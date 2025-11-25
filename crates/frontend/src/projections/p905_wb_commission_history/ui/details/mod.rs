use chrono::Utc;
use leptos::prelude::*;
use leptos::task::spawn_local;

use crate::projections::p905_wb_commission_history::api;
use crate::layout::global_context::AppGlobalContext;

#[component]
pub fn CommissionHistoryDetails(
    #[prop(into, optional)] id: Option<String>,
) -> impl IntoView {
    let is_new = id.is_none();
    
    // State
    let (loading, set_loading) = signal(false);
    let (error, set_error) = signal(None::<String>);
    let (success_message, set_success_message) = signal(None::<String>);
    let (active_tab, set_active_tab) = signal("fields");

    // Form fields
    let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();
    let (date, set_date) = signal(today);
    let (subject_id, set_subject_id) = signal("".to_string());
    let (subject_name, set_subject_name) = signal("".to_string());
    let (parent_id, set_parent_id) = signal("".to_string());
    let (parent_name, set_parent_name) = signal("".to_string());
    let (kgvp_booking, set_kgvp_booking) = signal("".to_string());
    let (kgvp_marketplace, set_kgvp_marketplace) = signal("".to_string());
    let (kgvp_pickup, set_kgvp_pickup) = signal("".to_string());
    let (kgvp_supplier, set_kgvp_supplier) = signal("".to_string());
    let (kgvp_supplier_express, set_kgvp_supplier_express) = signal("".to_string());
    let (paid_storage_kgvp, set_paid_storage_kgvp) = signal("".to_string());
    let (raw_json, set_raw_json) = signal("".to_string());

    // Load existing data if editing
    if let Some(ref commission_id) = id {
        let id_clone = commission_id.clone();
        Effect::new(move |_| {
            set_loading.set(true);
            let id_for_fetch = id_clone.clone();
            
            spawn_local(async move {
                match api::get_commission(&id_for_fetch).await {
                    Ok(commission) => {
                        set_date.set(commission.date);
                        set_subject_id.set(commission.subject_id.to_string());
                        set_subject_name.set(commission.subject_name);
                        set_parent_id.set(commission.parent_id.to_string());
                        set_parent_name.set(commission.parent_name);
                        set_kgvp_booking.set(commission.kgvp_booking.to_string());
                        set_kgvp_marketplace.set(commission.kgvp_marketplace.to_string());
                        set_kgvp_pickup.set(commission.kgvp_pickup.to_string());
                        set_kgvp_supplier.set(commission.kgvp_supplier.to_string());
                        set_kgvp_supplier_express.set(commission.kgvp_supplier_express.to_string());
                        set_paid_storage_kgvp.set(commission.paid_storage_kgvp.to_string());
                        set_raw_json.set(commission.raw_json);
                        set_loading.set(false);
                    }
                    Err(e) => {
                        set_error.set(Some(format!("Ошибка загрузки: {}", e)));
                        set_loading.set(false);
                    }
                }
            });
        });
    }

    let app_context = leptos::context::use_context::<AppGlobalContext>()
        .expect("AppGlobalContext not found");
    let close_tab = move |_| {
        if let Some(ref commission_id) = id {
            app_context.close_tab(&format!("p905-commission-{}", commission_id));
        } else {
            app_context.close_tab("p905-commission-new");
        }
    };

    view! {
        <div class="commission-details" style="padding: 20px;">
            <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                <h2 style="margin: 0;">
                    {if is_new { "Новая комиссия" } else { "Редактирование комиссии" }}
                </h2>
                <button
                    on:click=close_tab
                    style="padding: 6px 16px; background: #6c757d; color: white; border: none; border-radius: 4px; cursor: pointer;"
                >
                    "Закрыть"
                </button>
            </div>

            <div>
                <div style="padding: 12px; background: #d4edda; border: 1px solid #c3e6cb; border-radius: 4px; color: #155724; margin-bottom: 15px;">
                    "Note: Save functionality will be added soon"
                </div>

                {move || {
                    error.get().map(|msg| {
                        view! {
                            <div style="padding: 12px; background: #f8d7da; border: 1px solid #f5c6cb; border-radius: 4px; color: #721c24; margin-bottom: 15px;">
                                {msg}
                            </div>
                        }
                    })
                }}

                {move || {
                    success_message.get().map(|msg| {
                        view! {
                            <div style="padding: 12px; background: #d4edda; border: 1px solid #c3e6cb; border-radius: 4px; color: #155724; margin-bottom: 15px;">
                                {msg}
                            </div>
                        }
                    })
                }}

                <div style="max-width: 600px;">
                    <div style="margin-bottom: 15px;">
                        <label style="display: block; font-weight: 500; margin-bottom: 5px;">"Дата:"</label>
                        <input
                            type="date"
                            prop:value=move || date.get()
                            on:input=move |ev| set_date.set(event_target_value(&ev))
                            style="width: 100%; padding: 8px; border: 1px solid #ced4da; border-radius: 4px;"
                        />
                    </div>

                    <div style="margin-bottom: 15px;">
                        <label style="display: block; font-weight: 500; margin-bottom: 5px;">"Subject ID:"</label>
                        <input
                            type="number"
                            prop:value=move || subject_id.get()
                            on:input=move |ev| set_subject_id.set(event_target_value(&ev))
                            style="width: 100%; padding: 8px; border: 1px solid #ced4da; border-radius: 4px;"
                        />
                    </div>

                    <div style="margin-bottom: 15px;">
                        <label style="display: block; font-weight: 500; margin-bottom: 5px;">"Название категории:"</label>
                        <input
                            type="text"
                            prop:value=move || subject_name.get()
                            on:input=move |ev| set_subject_name.set(event_target_value(&ev))
                            style="width: 100%; padding: 8px; border: 1px solid #ced4da; border-radius: 4px;"
                        />
                    </div>

                    <div style="margin-bottom: 15px;">
                        <label style="display: block; font-weight: 500; margin-bottom: 5px;">"KGVP Booking (%):"</label>
                        <input
                            type="number"
                            step="0.01"
                            prop:value=move || kgvp_booking.get()
                            on:input=move |ev| set_kgvp_booking.set(event_target_value(&ev))
                            style="width: 100%; padding: 8px; border: 1px solid #ced4da; border-radius: 4px;"
                        />
                    </div>

                    <div style="margin-bottom: 15px;">
                        <label style="display: block; font-weight: 500; margin-bottom: 5px;">"KGVP Marketplace (%):"</label>
                        <input
                            type="number"
                            step="0.01"
                            prop:value=move || kgvp_marketplace.get()
                            on:input=move |ev| set_kgvp_marketplace.set(event_target_value(&ev))
                            style="width: 100%; padding: 8px; border: 1px solid #ced4da; border-radius: 4px;"
                        />
                    </div>
                </div>
            </div>
        </div>
    }
}
