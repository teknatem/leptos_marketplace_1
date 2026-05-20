use contracts::domain::a033_wb_day_close::{ProblemSeverity, WbDayCloseProblem};

/// Статическое описание детектора проблем.
pub struct ProblemDetectorDef {
    pub code: &'static str,
    pub name: &'static str,
    pub severity: ProblemSeverity,
    pub explanation: &'static str,
}

/// Реестр всех детекторов проблем (аналогично turnover_registry.rs).
pub const PROBLEM_DETECTORS: &[ProblemDetectorDef] = &[
    ProblemDetectorDef {
        code: "advert_clicks_order_accrual_without_expense",
        name: "Резерв рекламы без списания",
        severity: ProblemSeverity::Block,
        explanation: "По заказу есть p913.advert_clicks_order_accrual, но нет p913.advert_clicks_order_expense. \
                      Рекламный расход зарезервирован, но не списан. \
                      Необходимо перепровести соответствующий a012_wb_sales.",
    },
    ProblemDetectorDef {
        code: "a012_unposted_for_p903_row",
        name: "Непроведённый a012 при наличии p903",
        severity: ProblemSeverity::Warn,
        explanation: "По заказу есть строки в p903_wb_finance_report за день, \
                      но соответствующий a012_wb_sales не проведён или не найден. \
                      Перепроведение рекомендовано.",
    },
    ProblemDetectorDef {
        code: "advert_attributed_to_cancelled_order",
        name: "Реклама на отменённый заказ",
        severity: ProblemSeverity::Warn,
        explanation: "Заказ помечен как отменённый в a015_wb_orders, \
                      но p913 содержит advert_clicks_order_accrual для этого srid. \
                      Рекламный резерв должен быть закрыт.",
    },
    ProblemDetectorDef {
        code: "dealer_price_missing",
        name: "Отсутствует дилерская цена",
        severity: ProblemSeverity::Warn,
        explanation: "Не найдена dealer_price ни в a012_wb_sales, ни в p912_nomenclature_costs. \
                      Колонка «ЦенаДилер» пустая для этой строки.",
    },
    ProblemDetectorDef {
        code: "column_invariant_mismatch",
        name: "Нарушение инварианта колонок",
        severity: ProblemSeverity::Block,
        explanation: "Сумма колонок 1..7 не равна полю result. \
                      Это внутренняя ошибка формул — обратитесь к разработчикам.",
    },
    // ── Новые детекторы: связи a015/a012 и классификатор ────────────────────
    ProblemDetectorDef {
        code: "multiple_a012_for_srid",
        name: "Несколько a012 для одного srid",
        severity: ProblemSeverity::Block,
        explanation: "Для данного srid найдено более одного документа a012_wb_sales \
                      одного типа (sale или return). \
                      Допустима ровно одна реализация или один возврат на srid.",
    },
    ProblemDetectorDef {
        code: "a015_order_missing",
        name: "Заказ a015 не найден",
        severity: ProblemSeverity::Warn,
        explanation: "Строка типа «Продажа» или «Возврат», но в a015_wb_orders \
                      нет документа с соответствующим srid. \
                      Возможно, импорт заказов не завершён.",
    },
    ProblemDetectorDef {
        code: "a012_sale_missing",
        name: "Нет реализации a012 (продажа)",
        severity: ProblemSeverity::Warn,
        explanation: "Строка классифицирована как «Продажа», но в a012_wb_sales \
                      не найден документ с event_type=sale для данного srid. \
                      Перепроведение a012 рекомендовано.",
    },
    ProblemDetectorDef {
        code: "a012_return_missing",
        name: "Нет реализации a012 (возврат)",
        severity: ProblemSeverity::Warn,
        explanation: "Строка классифицирована как «Возврат», но в a012_wb_sales \
                      не найден документ с event_type=return для данного srid. \
                      Перепроведение a012 рекомендовано.",
    },
    ProblemDetectorDef {
        code: "mixed_sale_and_return_for_srid",
        name: "Смешанная строка: продажа и возврат",
        severity: ProblemSeverity::Block,
        explanation: "Строка содержит одновременно qty_sold > 0 и qty_returned > 0. \
                      Это некорректная ситуация: один srid не может быть одновременно \
                      продажей и возвратом. Проверьте данные p903.",
    },
    ProblemDetectorDef {
        code: "unknown_line_type",
        name: "Неизвестный тип строки",
        severity: ProblemSeverity::Warn,
        explanation: "Не удалось классифицировать строку по полям supplier_oper_name \
                      и суммам. Строка помечена как «Прочее». \
                      Возможно, WB добавил новый тип операции.",
    },
];

/// Найти описание детектора по коду.
pub fn get_detector(code: &str) -> Option<&'static ProblemDetectorDef> {
    PROBLEM_DETECTORS.iter().find(|d| d.code == code)
}

/// Создать проблему из кода детектора.
pub fn make_problem(
    code: &'static str,
    srid: Option<String>,
    nomenclature_ref: Option<String>,
    a012_ids: Vec<String>,
    extra_message: Option<String>,
) -> WbDayCloseProblem {
    let def =
        get_detector(code).unwrap_or_else(|| panic!("Unknown problem detector code: {}", code));

    let message = match extra_message {
        Some(msg) => msg,
        None => def.explanation.to_string(),
    };

    WbDayCloseProblem {
        code: code.to_string(),
        severity: def.severity.clone(),
        srid,
        nomenclature_ref,
        a012_ids,
        message,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn problem_codes_are_unique() {
        let mut seen = HashSet::new();
        for def in PROBLEM_DETECTORS {
            assert!(
                seen.insert(def.code),
                "duplicate problem detector code: {}",
                def.code
            );
        }
    }

    #[test]
    fn all_detectors_have_non_empty_fields() {
        for def in PROBLEM_DETECTORS {
            assert!(!def.code.is_empty(), "code must not be empty");
            assert!(
                !def.name.is_empty(),
                "name must not be empty for code={}",
                def.code
            );
            assert!(
                !def.explanation.is_empty(),
                "explanation must not be empty for code={}",
                def.code
            );
        }
    }

    #[test]
    fn get_detector_finds_known_code() {
        let def = get_detector("advert_clicks_order_accrual_without_expense");
        assert!(
            def.is_some(),
            "should find advert_clicks_order_accrual_without_expense"
        );
        assert_eq!(def.unwrap().severity, ProblemSeverity::Block);
    }

    #[test]
    fn get_detector_returns_none_for_unknown() {
        assert!(get_detector("totally_unknown_code").is_none());
    }

    #[test]
    fn make_problem_uses_detector_severity() {
        let p = make_problem(
            "advert_clicks_order_accrual_without_expense",
            Some("SRID-001".to_string()),
            None,
            vec![],
            None,
        );
        assert_eq!(p.code, "advert_clicks_order_accrual_without_expense");
        assert_eq!(p.severity, ProblemSeverity::Block);
        assert_eq!(p.srid.as_deref(), Some("SRID-001"));
        assert!(!p.message.is_empty());
    }
}
