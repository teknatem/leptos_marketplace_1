#!/usr/bin/env python3
"""
Script to generate WB Orders Details UI component based on WB Sales Details template
"""

import re

# Read the sales details template
with open('crates/frontend/src/domain/a012_wb_sales/ui/details/mod.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Replace type names
content = content.replace('WbSalesDetailDto', 'WbOrderDetailDto')
content = content.replace('WbSalesDetail', 'WbOrdersDetail')
content = content.replace('sale', 'order')
content = content.replace('Sale', 'Order')
content = content.replace('sales', 'orders')
content = content.replace('Sales', 'Orders')

# Update API endpoints
content = content.replace('/api/a012/wb-orders', '/api/a015/wb-orders')
content = content.replace('/api/a012/raw/', '/api/a015/raw/')

# Update title
content = content.replace('"Wildberries Orders Details"', '"Wildberries Orders Details"')

# Update LineDto structure - this requires manual adjustment
# Find and replace the LineDto definition
line_dto_pattern = r'#\[derive\(Debug, Clone, Serialize, Deserialize\)\]\s*pub struct LineDto \{[^}]*\}'
line_dto_replacement = '''#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineDto {
    pub line_id: String,
    pub supplier_article: String,
    pub nm_id: i64,
    pub barcode: String,
    pub category: Option<String>,
    pub subject: Option<String>,
    pub brand: Option<String>,
    pub tech_size: Option<String>,
    pub qty: f64,
    pub total_price: Option<f64>,
    pub discount_percent: Option<f64>,
    pub spp: Option<f64>,
    pub finished_price: Option<f64>,
    pub price_with_disc: Option<f64>,
}'''
content = re.sub(line_dto_pattern, line_dto_replacement, content, flags=re.DOTALL)

# Update StateDto structure
state_dto_pattern = r'#\[derive\(Debug, Clone, Serialize, Deserialize\)\]\s*pub struct StateDto \{[^}]*\}'
state_dto_replacement = '''#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDto {
    pub order_dt: String,
    pub last_change_dt: Option<String>,
    pub is_cancel: bool,
    pub cancel_dt: Option<String>,
    pub is_supply: Option<bool>,
    pub is_realization: Option<bool>,
}'''
content = re.sub(state_dto_pattern, state_dto_replacement, content, flags=re.DOTALL)

# Add GeographyDto after WarehouseDto
warehouse_dto_end = content.find('pub struct WarehouseDto {')
if warehouse_dto_end != -1:
    next_derive = content.find('#[derive(Debug, Clone, Serialize, Deserialize)]', warehouse_dto_end + 50)
    if next_derive != -1:
        geography_dto = '''
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeographyDto {
    pub country_name: Option<String>,
    pub oblast_okrug_name: Option<String>,
    pub region_name: Option<String>,
}

'''
        content = content[:next_derive] + geography_dto + content[next_derive:]

# Update WbOrderDetailDto to include geography
order_detail_pattern = r'(pub struct WbOrderDetailDto \{[^}]*pub warehouse: WarehouseDto,)'
order_detail_replacement = r'\1\n    pub geography: GeographyDto,'
content = re.sub(order_detail_pattern, order_detail_replacement, content, flags=re.DOTALL)

# Update SourceMetaDto structure
source_meta_pattern = r'(pub struct SourceMetaDto \{\s*)'
source_meta_replacement = r'\1pub income_id: Option<i64>,\n    pub sticker: Option<String>,\n    pub g_number: Option<String>,\n    '
content = re.sub(source_meta_pattern, source_meta_replacement, content, flags=re.DOTALL)

# Update field references in view code
content = content.replace('order_data.state.order_dt', 'order_data.state.order_dt')
content = content.replace('order_data.state.event_type', 'if order_data.state.is_cancel { "Отменён" } else { "Активен" }')
content = content.replace('order_data.state.status_norm', '""')

# Write output
with open('crates/frontend/src/domain/a015_wb_orders/ui/details/mod.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print("Generated WB Orders Details component successfully!")

