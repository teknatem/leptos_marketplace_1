/// Метаданные UseCase для идентификации и документирования
pub trait UseCaseMetadata {
    /// Индекс UseCase (например, "u501")
    fn usecase_index() -> &'static str;

    /// Техническое имя (например, "import_from_ut")
    fn usecase_name() -> &'static str;

    /// Отображаемое имя для UI (например, "Импорт из УТ 11")
    fn display_name() -> &'static str;

    /// Описание UseCase
    fn description() -> &'static str {
        ""
    }

    /// Полное имя вида "u501_import_from_ut"
    fn full_name() -> String {
        format!("{}_{}", Self::usecase_index(), Self::usecase_name())
    }
}
