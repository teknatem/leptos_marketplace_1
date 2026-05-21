//! Доп. информационные колонки drilldown, зависящие от выбранного измерения.
//!
//! Идея: некоторые измерения несут полезную справочную информацию помимо
//! наименования. Например, для измерения «Номенклатура» удобно показывать
//! «Артикул» рядом с наименованием товара.
//!
//! Механизм намеренно отделён от тяжёлого SQL drilldown:
//! [`resolve`] получает уже посчитанный список `group_key` строк отчёта и
//! догружает справочные значения одним батч-запросом по первичному ключу.
//! Это не утяжеляет горячий путь агрегации и легко расширяется — чтобы
//! добавить новую колонку/измерение, дописываем ветку в [`resolve`].

use std::collections::HashMap;

use anyhow::Result;
use sea_orm::{ConnectionTrait, Statement, Value};

use crate::shared::data::db::get_connection;
use contracts::shared::drilldown::ExtraColumnDef;

/// Результат резолва доп. колонок для одного measurement (`group_by`).
#[derive(Debug, Default)]
pub struct DrilldownExtra {
    /// Определения колонок (id + label), по порядку отображения.
    pub columns: Vec<ExtraColumnDef>,
    /// Значения: `group_key` строки → (`id` колонки → значение).
    pub values: HashMap<String, HashMap<String, String>>,
}

/// Резолвит доп. колонки для измерения `group_by` по списку ключей строк.
///
/// `group_keys` — `group_key` строк отчёта (для номенклатуры это id из
/// `a004_nomenclature`). Неизвестные/служебные ключи просто не попадают в map.
///
/// Если для измерения доп. колонок нет — возвращается пустой [`DrilldownExtra`]
/// (фронтенд при этом ничего лишнего не рисует).
pub async fn resolve(group_by: &str, group_keys: &[String]) -> Result<DrilldownExtra> {
    match group_by {
        "nomenclature" => resolve_nomenclature(group_keys).await,
        _ => Ok(DrilldownExtra::default()),
    }
}

/// Номенклатура → колонка «Артикул» (из `a004_nomenclature.article`).
async fn resolve_nomenclature(group_keys: &[String]) -> Result<DrilldownExtra> {
    let ids: Vec<&String> = group_keys
        .iter()
        .filter(|key| !key.is_empty() && !key.starts_with("__") && !key.starts_with('('))
        .collect();

    let columns = vec![ExtraColumnDef {
        id: "article".to_string(),
        label: "Артикул".to_string(),
    }];

    if ids.is_empty() {
        return Ok(DrilldownExtra {
            columns,
            values: HashMap::new(),
        });
    }

    let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let sql = format!(
        "SELECT id, COALESCE(article, '') AS article \
         FROM a004_nomenclature WHERE id IN ({placeholders})"
    );
    let params: Vec<Value> = ids
        .iter()
        .map(|id| Value::String(Some(Box::new((*id).clone()))))
        .collect();

    let db = get_connection();
    let stmt = Statement::from_sql_and_values(db.get_database_backend(), &sql, params);
    let rows = db.query_all(stmt).await?;

    let mut values: HashMap<String, HashMap<String, String>> = HashMap::new();
    for row in rows {
        let id: String = row.try_get("", "id").unwrap_or_default();
        let article: String = row.try_get("", "article").unwrap_or_default();
        if id.is_empty() || article.is_empty() {
            continue;
        }
        values.insert(id, HashMap::from([("article".to_string(), article)]));
    }

    Ok(DrilldownExtra { columns, values })
}
