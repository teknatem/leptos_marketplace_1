/// Базовый трейт для элементов, которые можно выбирать через пикер
pub trait AggregatePickerResult {
    fn id(&self) -> String;
    fn display_name(&self) -> String;
}

/// Трейт для элементов, которые можно отобразить в виде таблицы
pub trait TableDisplayable: AggregatePickerResult {
    fn code(&self) -> String;
    fn description(&self) -> String;
}
