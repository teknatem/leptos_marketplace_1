//! DataView — Семантический слой
//!
//! Реестр именованных DataView: каждый view в своём подкаталоге dvNNN/.
//! На входе — ViewContext, на выходе — IndicatorValue (scalar) или DrilldownResponse.
//!
//! Кеш: `compute_scalar` сохраняет результаты в глобальном TTL-кеше (30 сек).
//! Несколько индикаторов дашборда с одинаковым DataView + контекстом не делают
//! повторных запросов к БД.

pub mod dv001;
pub mod filters;

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::Result;
use contracts::shared::data_view::{DataViewMeta, FilterDef, ViewContext};
use contracts::shared::drilldown::DrilldownResponse;
use contracts::shared::indicators::IndicatorValue;

// ---------------------------------------------------------------------------
// Global scalar result cache
// ---------------------------------------------------------------------------

const CACHE_TTL: Duration = Duration::from_secs(30);

struct CacheEntry {
    value: IndicatorValue,
    created_at: Instant,
}

static SCALAR_CACHE: Mutex<Option<HashMap<String, CacheEntry>>> = Mutex::new(None);

fn cache_key(view_id: &str, ctx: &ViewContext) -> String {
    let refs = {
        let mut r = ctx.connection_mp_refs.clone();
        r.sort();
        r.join(",")
    };
    let metric = ctx.params.get("metric").map(|s| s.as_str()).unwrap_or("");
    format!(
        "{view_id}|{}|{}|{}|{}|{refs}|{metric}",
        ctx.date_from,
        ctx.date_to,
        ctx.period2_from.as_deref().unwrap_or(""),
        ctx.period2_to.as_deref().unwrap_or(""),
    )
}

fn cache_get(key: &str) -> Option<IndicatorValue> {
    let mut guard = SCALAR_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    if let Some(entry) = map.get(key) {
        if entry.created_at.elapsed() < CACHE_TTL {
            return Some(entry.value.clone());
        }
        map.remove(key);
    }
    None
}

fn cache_put(key: String, value: IndicatorValue) {
    let mut guard = SCALAR_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    // Evict expired entries periodically (when map grows).
    if map.len() > 256 {
        map.retain(|_, e| e.created_at.elapsed() < CACHE_TTL);
    }
    map.insert(key, CacheEntry { value, created_at: Instant::now() });
}

type ScalarFn =
    fn(&ViewContext) -> Pin<Box<dyn Future<Output = Result<IndicatorValue>> + Send + '_>>;

type DrillFn = for<'a> fn(
    &'a ViewContext,
    &'a str,
) -> Pin<Box<dyn Future<Output = Result<DrilldownResponse>> + Send + 'a>>;

struct ViewEntry {
    scalar: ScalarFn,
    drilldown: DrillFn,
    meta: DataViewMeta,
}

/// Реестр DataView: отображает view_id → функции вычисления + метаданные.
pub struct DataViewRegistry {
    views: HashMap<String, ViewEntry>,
}

impl DataViewRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            views: HashMap::new(),
        };

        registry.register(
            dv001::meta(),
            |ctx| Box::pin(dv001::compute_scalar(ctx)),
            |ctx, g| Box::pin(dv001::compute_drilldown(ctx, g)),
        );

        registry
    }

    fn register(&mut self, meta: DataViewMeta, scalar: ScalarFn, drilldown: DrillFn) {
        let id = meta.id.clone();
        self.views
            .insert(id, ViewEntry { scalar, drilldown, meta });
    }

    /// Вычислить скалярное значение индикатора через DataView.
    ///
    /// Результат кешируется на [`CACHE_TTL`] по ключу `(view_id, context)`.
    /// Повторные вызовы с теми же параметрами (например, из нескольких индикаторов
    /// одного дашборда) не делают дополнительных запросов к БД.
    pub async fn compute_scalar(
        &self,
        view_id: &str,
        ctx: &ViewContext,
    ) -> Result<IndicatorValue> {
        let key = cache_key(view_id, ctx);

        if let Some(cached) = cache_get(&key) {
            tracing::debug!("DataView cache hit: {key}");
            return Ok(cached);
        }

        let entry = self
            .views
            .get(view_id)
            .ok_or_else(|| anyhow::anyhow!("DataView not found: {}", view_id))?;
        let value = (entry.scalar)(ctx).await?;
        cache_put(key, value.clone());
        Ok(value)
    }

    /// Вычислить drilldown через DataView.
    pub async fn compute_drilldown(
        &self,
        view_id: &str,
        ctx: &ViewContext,
        group_by: &str,
    ) -> Result<DrilldownResponse> {
        let entry = self
            .views
            .get(view_id)
            .ok_or_else(|| anyhow::anyhow!("DataView not found: {}", view_id))?;
        (entry.drilldown)(ctx, group_by).await
    }

    /// Список метаданных всех зарегистрированных DataView.
    pub fn list_meta(&self) -> Vec<&DataViewMeta> {
        let mut list: Vec<&DataViewMeta> = self.views.values().map(|e| &e.meta).collect();
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }

    /// Метаданные конкретного DataView по id.
    pub fn get_meta(&self, id: &str) -> Option<&DataViewMeta> {
        self.views.get(id).map(|e| &e.meta)
    }

    /// Проверить, зарегистрирован ли view.
    pub fn has_view(&self, view_id: &str) -> bool {
        self.views.contains_key(view_id)
    }

    /// Резолвить фильтры конкретного DataView из глобального реестра.
    ///
    /// Берёт FilterRef[] из метаданных view, находит соответствующие FilterDef
    /// в глобальном реестре и возвращает их в порядке `order`.
    /// Применяет `label_override` если задан в FilterRef.
    pub fn resolve_filters(&self, view_id: &str) -> Vec<FilterDef> {
        let Some(meta) = self.get_meta(view_id) else {
            return vec![];
        };

        let registry = filters::global_filter_registry();
        let mut refs = meta.filters.clone();
        refs.sort_by_key(|r| r.order);

        refs.into_iter()
            .filter_map(|r| {
                let mut def = registry.get(&r.filter_id)?.clone();
                if let Some(label) = r.label_override {
                    def.label = label;
                }
                Some(def)
            })
            .collect()
    }
}

impl Default for DataViewRegistry {
    fn default() -> Self {
        Self::new()
    }
}
