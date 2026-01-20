//! General Tab - Return header and state information

use super::super::model::YmReturnDetailDto;
use crate::shared::date_utils::format_datetime;
use leptos::prelude::*;

#[component]
pub fn GeneralTab(data: YmReturnDetailDto) -> impl IntoView {
    let return_type = data.header.return_type.clone();
    let return_type_label = match return_type.as_str() {
        "UNREDEEMED" => "Невыкуп".to_string(),
        "RETURN" => "Возврат".to_string(),
        _ => return_type.clone(),
    };
    
    let (return_type_class, return_type_extra_style) = match data.header.return_type.as_str() {
        "UNREDEEMED" => ("badge", "background: #fff3e0; color: #e65100;"),
        "RETURN" => ("badge", "background: #e3f2fd; color: #1565c0;"),
        _ => ("badge", "background: #f5f5f5; color: #666;"),
    };
    
    let (refund_status_class, refund_extra_style) = match data.state.refund_status.as_str() {
        "REFUNDED" => ("badge badge-success", ""),
        "NOT_REFUNDED" => ("badge badge-error", ""),
        "REFUND_IN_PROGRESS" => ("badge", "background: #fff3e0; color: #e65100;"),
        _ => ("badge", "background: #f5f5f5; color: #666;"),
    };

    view! {
        <div class="general-info" style="max-width: 1400px;">
            <div style="background: var(--color-bg-body); padding: var(--space-xl); border-radius: var(--radius-md); border: 1px solid var(--color-border-lighter);">
                <div style="display: grid; grid-template-columns: 180px 1fr; gap: var(--space-md); align-items: start; font-size: var(--font-size-sm);">
                    <div class="field-label">"Return №:"</div>
                    <div style="font-family: monospace; font-size: var(--font-size-base); font-weight: var(--font-weight-semibold); color: #1976d2;">
                        {data.header.return_id}
                    </div>

                    <div class="field-label">"Order №:"</div>
                    <div style="font-family: monospace;">{data.header.order_id}</div>

                    <div class="field-label">"Type:"</div>
                    <div>
                        <span class={return_type_class} style={return_type_extra_style}>
                            {return_type_label}
                        </span>
                    </div>

                    <div class="field-label">"Refund Status:"</div>
                    <div>
                        <span class={refund_status_class} style={refund_extra_style}>
                            {data.state.refund_status.clone()}
                        </span>
                    </div>

                    <div class="field-label">"Amount:"</div>
                    <div style="font-size: var(--font-size-base); font-weight: var(--font-weight-semibold); color: #c62828;">
                        {data.header.amount.map(|a| format!("{:.2}", a)).unwrap_or("—".to_string())}
                        {data.header.currency.clone().map(|c| format!(" {}", c)).unwrap_or_default()}
                    </div>

                    <div class="field-label">"Campaign ID:"</div>
                    <div style="font-family: monospace;">{data.header.campaign_id.clone()}</div>

                    <div class="field-label">"Created At Source:"</div>
                    <div class="field-value">
                        {data
                            .state
                            .created_at_source
                            .as_ref()
                            .map(|d| format_datetime(d))
                            .unwrap_or("—".to_string())}

                    </div>

                    <div class="field-label">"Updated At Source:"</div>
                    <div class="field-value">
                        {data
                            .state
                            .updated_at_source
                            .as_ref()
                            .map(|d| format_datetime(d))
                            .unwrap_or("—".to_string())}

                    </div>

                    <div class="field-label">"Refund Date:"</div>
                    <div class="field-value">
                        {data
                            .state
                            .refund_date
                            .as_ref()
                            .map(|d| format_datetime(d))
                            .unwrap_or("—".to_string())}

                    </div>

                    <div class="field-label">"Fetched At:"</div>
                    <div class="field-value">{format_datetime(&data.source_meta.fetched_at)}</div>

                    <div class="field-label">"Document Version:"</div>
                    <div class="field-value">{data.source_meta.document_version}</div>

                    <div class="field-label">"Is Posted:"</div>
                    <div>
                        {if data.is_posted {
                            view! {
                                <span style="color: var(--color-success); font-weight: var(--font-weight-medium);">
                                    "✓ Yes"
                                </span>
                            }
                                .into_any()
                        } else {
                            view! { <span style="color: var(--color-text-muted);">"No"</span> }
                                .into_any()
                        }}

                    </div>
                </div>
            </div>
        </div>
    }
}
