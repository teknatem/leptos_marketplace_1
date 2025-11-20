#!/usr/bin/env python3
"""
Fix Orders Details component - handle field differences from Sales
"""

with open('crates/frontend/src/domain/a015_wb_orders/ui/details/mod.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix line.name reference - Orders doesn't have name field, show brand/category instead
content = content.replace(
    '''<div style="font-weight: 600; color: #555;">"Название:"</div>
                                                        <div style="font-family: var(--font-family-base); font-size: 14px; font-weight: 500;">{line.name.clone()}</div>''',
    '''<div style="font-weight: 600; color: #555;">"Бренд:"</div>
                                                        <div style="font-family: var(--font-family-base); font-size: 14px; font-weight: 500;">{line.brand.clone().unwrap_or("—".to_string())}</div>
                                                        
                                                        <div style="font-weight: 600; color: #555;">"Категория:"</div>
                                                        <div style="font-family: var(--font-family-base); font-size: 14px;">{line.category.clone().unwrap_or("—".to_string())}</div>
                                                        
                                                        <div style="font-weight: 600; color: #555;">"Предмет:"</div>
                                                        <div style="font-family: var(--font-family-base); font-size: 14px;">{line.subject.clone().unwrap_or("—".to_string())}</div>
                                                        
                                                        <div style="font-weight: 600; color: #555;">"Размер:"</div>
                                                        <div style="font-family: var(--font-family-base); font-size: 14px;">{line.tech_size.clone().unwrap_or("—".to_string())}</div>'''
)

# Remove fields that don't exist in Orders LineDto
fields_to_remove = [
    'price_list',
    'discount_total',
    'price_effective',
    'payment_sale_amount',
    'amount_line',
    'currency_code'
]

# Replace the price table with Orders-specific fields
old_price_table = '''<tr>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px;">"Цена без скидок"</td>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px;"><code style="font-size: 0.85em;">"price_list"</code></td>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px; text-align: right;">{line.price_list.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px;">"rub"</td>
                                                            </tr>
                                                            <tr>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px;">"Сумма скидок"</td>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px;"><code style="font-size: 0.85em;">"discount_total"</code></td>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px; text-align: right;">{line.discount_total.map(|d| format!("{:.2}", d)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px;">"rub"</td>
                                                            </tr>
                                                            <tr>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px;">"Цена после скидок"</td>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px;"><code style="font-size: 0.85em;">"price_effective"</code></td>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px; text-align: right;">{line.price_effective.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px;">"rub"</td>
                                                            </tr>'''

new_price_rows = '''<tr>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px;">"Цена с учетом скидки"</td>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px;"><code style="font-size: 0.85em;">"price_with_disc"</code></td>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px; text-align: right;">{line.price_with_disc.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px;">"rub"</td>
                                                            </tr>'''

content = content.replace(old_price_table, new_price_rows)

# Remove payment_sale_amount and amount_line rows
content = content.replace('''<tr>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px;">"Сумма платежа"</td>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px;"><code style="font-size: 0.85em;">"payment_sale_amount"</code></td>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px; text-align: right;">{line.payment_sale_amount.map(|p| format!("{:.2}", p)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px;">"rub"</td>
                                                            </tr>
                                                            <tr style="background:rgb(138, 227, 254);">
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px; font-weight: 600;">"К выплате"</td>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px;"><code style="font-size: 0.85em;">"amount_line"</code></td>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px; text-align: right; font-weight: 600;">{line.amount_line.map(|a| format!("{:.2}", a)).unwrap_or("—".to_string())}</td>
                                                                <td style="border: 1px solid #ddd; padding: 4px 6px;">"rub"</td>
                                                            </tr>''', '')

# Fix state fields - Orders has order_dt and is_cancel instead of sale_dt, event_type, status_norm
content = content.replace('order_data.state.event_type.clone()', '"Order"')
content = content.replace('order_data.state.status_norm.clone()', 'if order_data.state.is_cancel { "Отменён" } else { "Активен" }')
content = content.replace('order_data.state.order_dt', 'order_data.state.order_dt')
content = content.replace('"Sale Date:"', '"Order Date:"')
content = content.replace('"sale Date:"', '"Order Date:"')

# Simplify general tab - remove overly complex event_type and status_norm display
# Replace the event/status display with cancel status
old_event_status = '''<div style="font-weight: 600; color: #555;">"Event Type:"</div>
                                                            <div>
                                                                <span style="padding: 2px 8px; background: #e3f2fd; color: #1976d2; border-radius: 3px; font-weight: 500;">
                                                                    {order_data.state.event_type.clone()}
                                                                </span>
                                                            </div>

                                                            <div style="font-weight: 600; color: #555;">"Status:"</div>
                                                            <div>
                                                                <span style="padding: 2px 8px; background: #e8f5e9; color: #2e7d32; border-radius: 3px; font-weight: 500;">
                                                                    {order_data.state.status_norm.clone()}
                                                                </span>
                                                            </div>'''

new_cancel_status = '''<div style="font-weight: 600; color: #555;">"Статус заказа:"</div>
                                                            <div>
                                                                {if order_data.state.is_cancel {
                                                                    view! {
                                                                        <span style="padding: 2px 8px; background: #ffebee; color: #c62828; border-radius: 3px; font-weight: 500;">
                                                                            "Отменён"
                                                                        </span>
                                                                    }.into_any()
                                                                } else {
                                                                    view! {
                                                                        <span style="padding: 2px 8px; background: #e8f5e9; color: #2e7d32; border-radius: 3px; font-weight: 500;">
                                                                            "Активен"
                                                                        </span>
                                                                    }.into_any()
                                                                }}
                                                            </div>
                                                            
                                                            {order_data.state.cancel_dt.as_ref().map(|dt| {
                                                                view! {
                                                                    <>
                                                                        <div style="font-weight: 600; color: #555;">"Дата отмены:"</div>
                                                                        <div style="color: #c62828;">{format_datetime(dt)}</div>
                                                                    </>
                                                                }
                                                            })}'''

content = content.replace(old_event_status, new_cancel_status)

# Fix date field name
content = content.replace('"Sale Date:"', '"Order Date:"')
content = content.replace('format_datetime(&order_data.state.order_dt)', 'format_datetime(&order_data.state.order_dt)')

with open('crates/frontend/src/domain/a015_wb_orders/ui/details/mod.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Fixed WB Orders Details component!")

