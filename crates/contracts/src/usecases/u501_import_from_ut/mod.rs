pub mod request;
pub mod response;
pub mod events;
pub mod progress;

pub use request::ImportRequest;
pub use response::ImportResponse;
pub use events::ImportEvent;
pub use progress::{ImportProgress, AggregateProgress};

use crate::usecases::common::UseCaseMetadata;

pub struct ImportFromUt;

impl UseCaseMetadata for ImportFromUt {
    fn usecase_index() -> &'static str {
        "u501"
    }

    fn usecase_name() -> &'static str {
        "import_from_ut"
    }

    fn display_name() -> &'static str {
        "Импорт из УТ 11"
    }

    fn description() -> &'static str {
        "Загрузка справочников из 1С:Управление торговлей 11 через OData"
    }
}
