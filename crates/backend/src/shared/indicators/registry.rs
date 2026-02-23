use anyhow::Result;
use contracts::shared::indicators::*;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

use super::compute::sales;
use super::metadata::ids;

type ComputeFn =
    fn(&IndicatorContext) -> Pin<Box<dyn Future<Output = Result<IndicatorValue>> + Send + '_>>;

/// Central registry: maps `IndicatorId` to its async compute function.
pub struct IndicatorRegistry {
    fns: HashMap<String, ComputeFn>,
}

impl IndicatorRegistry {
    pub fn new() -> Self {
        let mut fns: HashMap<String, ComputeFn> = HashMap::new();

        fns.insert(ids::sales_revenue().0, |ctx| {
            Box::pin(sales::compute_sales_revenue(ctx))
        });
        fns.insert(ids::sales_order_count().0, |ctx| {
            Box::pin(sales::compute_sales_order_count(ctx))
        });
        fns.insert(ids::sales_avg_check().0, |ctx| {
            Box::pin(sales::compute_sales_avg_check(ctx))
        });
        fns.insert(ids::sales_returns_sum().0, |ctx| {
            Box::pin(sales::compute_sales_returns_sum(ctx))
        });

        Self { fns }
    }

    /// Compute a batch of indicators concurrently.
    pub async fn compute(
        &self,
        ids: &[IndicatorId],
        ctx: &IndicatorContext,
    ) -> Vec<IndicatorValue> {
        let mut results = Vec::with_capacity(ids.len());

        for id in ids {
            if let Some(compute_fn) = self.fns.get(&id.0) {
                match compute_fn(ctx).await {
                    Ok(val) => results.push(val),
                    Err(e) => {
                        tracing::warn!("indicator {} compute error: {e:#}", id.0);
                        results.push(IndicatorValue {
                            id: id.clone(),
                            value: None,
                            previous_value: None,
                            change_percent: None,
                            status: IndicatorStatus::Neutral,
                            subtitle: Some(format!("Ошибка: {e}")),
                        });
                    }
                }
            } else {
                tracing::warn!("indicator {} not found in registry", id.0);
            }
        }

        results
    }
}
