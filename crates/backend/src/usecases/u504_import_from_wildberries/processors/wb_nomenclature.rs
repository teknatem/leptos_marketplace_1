use anyhow::Result;

pub async fn resolve_wb_nomenclature_ref(
    connection_mp_ref: &str,
    nm_id: i64,
    seller_article: Option<&str>,
) -> Result<Option<String>> {
    let sku = nm_id.to_string();

    let mut resolved =
        crate::domain::a007_marketplace_product::repository::get_by_connection_and_sku(
            connection_mp_ref,
            &sku,
        )
        .await?
        .and_then(|item| item.nomenclature_ref);

    if resolved.is_none() {
        if let Some(article) = seller_article
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            resolved =
                crate::domain::a007_marketplace_product::repository::get_unique_by_connection_and_article(
                    connection_mp_ref,
                    article,
                )
                .await?
                .and_then(|item| item.nomenclature_ref);
        }
    }

    Ok(resolved)
}
