//! # Подсистема контроля качества данных
//!
//! Вертикальная подсистема для проверки состояния данных и выявления потенциальных
//! проблем в агрегатах и проекциях.
//!
//! ## Архитектура
//!
//! ```text
//! quality/
//! ├── mod.rs                          ← реестр проверок + диспетчер run_check()
//! └── checks/
//!     ├── mod.rs
//!     └── nomenclature_in_projections.rs   ← первая проверка
//! ```
//!
//! ## Концепция
//!
//! Каждая **проверка** (check) отвечает за одну конкретную потенциальную проблему
//! или группу однородных проблем. Проверка:
//!
//! - имеет уникальный строковый `id` и читаемое название;
//! - запускается вручную через `POST /api/quality/checks/{id}/run`;
//! - возвращает [`CheckResult`] с набором метрик и итоговым счётчиком проблем;
//! - в будущем может быть запущена по расписанию или при событии.
//!
//! ## Зарегистрированные проверки
//!
//! | ID | Название | Что проверяет |
//! |----|----------|---------------|
//! | `nomenclature_in_projections` | Заполненность номенклатуры в проекциях | Строки p909/p911/p913, где `nomenclature_ref IS NULL` |
//! | `gl_projection_integrity` | Целостность GL ↔ ProjectionLinked-проекции | orphan_gl / orphan_projection / amount_mismatch для p909/p910/p911/p913 |
//! | `p903_gl_integrity` | Целостность GL ↔ p903 (ExternalLinked) | orphan_gl / amount_mismatch для p903_wb_finance_report |
//!
//! ## Добавление новой проверки
//!
//! 1. Создать файл `checks/my_check.rs`, объявить `CHECK_ID`, `info()`, `run()`.
//! 2. Добавить `pub mod my_check;` в `checks/mod.rs`.
//! 3. В этом модуле добавить `checks::my_check::info()` в [`list_checks`]
//!    и ветку в [`run_check`].

pub mod checks;

use contracts::quality::{
    CheckDetails, CheckResult, NipCleanupRequest, NipCleanupResult, NipGroupsResponse,
    NipProjectionRow, NipRepostRequest, NipRepostResult, QualityCheckInfo, QualityCheckSource,
};

/// Возвращает список всех зарегистрированных проверок.
pub fn list_checks() -> Vec<QualityCheckInfo> {
    let mut checks = vec![
        checks::nomenclature_in_projections::info(),
        checks::projection_orphan_registrators::info(),
        checks::marketplace_product_ref_required::info(),
        checks::gl_projection_integrity::info(),
        checks::p903_gl_integrity::info(),
    ];

    for (idx, check) in checks.iter_mut().enumerate() {
        check.code = format!("QC-{:03}", idx + 1);
    }

    checks
}

/// Запускает проверку по её ID и возвращает результат.
///
/// Возвращает `Err` с маркером `"NOT_FOUND"` в сообщении, если ID не зарегистрирован.
pub async fn run_check(id: &str) -> anyhow::Result<CheckResult> {
    match id {
        checks::nomenclature_in_projections::CHECK_ID => {
            checks::nomenclature_in_projections::run().await
        }
        checks::projection_orphan_registrators::CHECK_ID => {
            checks::projection_orphan_registrators::run().await
        }
        checks::marketplace_product_ref_required::CHECK_ID => {
            checks::marketplace_product_ref_required::run().await
        }
        checks::gl_projection_integrity::CHECK_ID => {
            checks::gl_projection_integrity::run().await
        }
        checks::p903_gl_integrity::CHECK_ID => {
            checks::p903_gl_integrity::run().await
        }
        other => Err(anyhow::anyhow!("NOT_FOUND: Unknown check id: {}", other)),
    }
}

/// Собирает полный пакет детализации правила для страницы `quality_check_details`:
/// метаданные + прогон (с популяцией и нарушениями) + разрезы + источники drill-down.
pub async fn check_details(id: &str) -> anyhow::Result<CheckDetails> {
    let info = list_checks()
        .into_iter()
        .find(|c| c.id == id)
        .ok_or_else(|| anyhow::anyhow!("NOT_FOUND: Unknown check id: {}", id))?;

    let result = run_check(id).await?;

    let breakdowns = match id {
        checks::nomenclature_in_projections::CHECK_ID => {
            checks::nomenclature_in_projections::breakdowns().await?
        }
        checks::projection_orphan_registrators::CHECK_ID => {
            checks::projection_orphan_registrators::breakdowns().await?
        }
        _ => Vec::new(),
    };

    let sources = list_check_sources(id).unwrap_or_default();

    Ok(CheckDetails {
        info,
        result,
        breakdowns,
        sources,
    })
}

/// Возвращает список источников (проекционных таблиц) для указанной проверки.
pub fn list_check_sources(check_id: &str) -> anyhow::Result<Vec<QualityCheckSource>> {
    match check_id {
        checks::nomenclature_in_projections::CHECK_ID => {
            Ok(checks::nomenclature_in_projections::list_sources())
        }
        checks::projection_orphan_registrators::CHECK_ID => {
            Ok(checks::projection_orphan_registrators::list_sources())
        }
        other => Err(anyhow::anyhow!("NOT_FOUND: Unknown check id: {}", other)),
    }
}

/// Возвращает страницу групп регистраторов для drill-down по проекционной таблице.
pub async fn list_check_groups(
    check_id: &str,
    projection_table: &str,
    page: i64,
    page_size: i64,
    sort_by: &str,
    sort_desc: bool,
) -> anyhow::Result<NipGroupsResponse> {
    match check_id {
        checks::nomenclature_in_projections::CHECK_ID => {
            checks::nomenclature_in_projections::list_groups(
                projection_table,
                page,
                page_size,
                sort_by,
                sort_desc,
            )
            .await
        }
        checks::projection_orphan_registrators::CHECK_ID => {
            checks::projection_orphan_registrators::list_groups(
                projection_table,
                page,
                page_size,
                sort_by,
                sort_desc,
            )
            .await
        }
        other => Err(anyhow::anyhow!("NOT_FOUND: Unknown check id: {}", other)),
    }
}

/// Возвращает строки проекции с пустым `nomenclature_ref` для одного регистратора.
pub async fn list_check_rows(
    check_id: &str,
    projection_table: &str,
    registrator_ref: &str,
) -> anyhow::Result<Vec<NipProjectionRow>> {
    match check_id {
        checks::nomenclature_in_projections::CHECK_ID => {
            checks::nomenclature_in_projections::list_rows(projection_table, registrator_ref).await
        }
        checks::projection_orphan_registrators::CHECK_ID => {
            checks::projection_orphan_registrators::list_rows(projection_table, registrator_ref)
                .await
        }
        other => Err(anyhow::anyhow!("NOT_FOUND: Unknown check id: {}", other)),
    }
}

/// Удаляет осиротевшие строки проекций по указанным регистраторам.
pub async fn cleanup_orphans(
    check_id: &str,
    request: &NipCleanupRequest,
) -> anyhow::Result<NipCleanupResult> {
    match check_id {
        checks::projection_orphan_registrators::CHECK_ID => {
            checks::projection_orphan_registrators::cleanup(
                &request.projection_table,
                &request.registrator_refs,
            )
            .await
        }
        other => Err(anyhow::anyhow!("NOT_FOUND: Unknown check id: {}", other)),
    }
}

/// Массово перепроводит указанные документы.
pub async fn bulk_repost(
    check_id: &str,
    request: &NipRepostRequest,
) -> anyhow::Result<NipRepostResult> {
    match check_id {
        checks::nomenclature_in_projections::CHECK_ID => {
            checks::nomenclature_in_projections::bulk_repost(
                &request.registrator_type,
                &request.registrator_refs,
            )
            .await
        }
        other => Err(anyhow::anyhow!("NOT_FOUND: Unknown check id: {}", other)),
    }
}
