pub mod events;
pub mod progress;
pub mod request;
pub mod response;

pub use events::ImportEvent;
pub use progress::{AggregateProgress, ImportProgress};
pub use request::ImportRequest;
pub use response::ImportResponse;

use crate::usecases::common::UseCaseMetadata;

pub struct ImportFromLemanaPro;

impl UseCaseMetadata for ImportFromLemanaPro {
    fn usecase_index() -> &'static str {
        "u506"
    }

    fn usecase_name() -> &'static str {
        "import_from_lemanapro"
    }

    fn display_name() -> &'static str {
        "Импорт из ЛеманаПро"
    }

    fn description() -> &'static str {
        "Загрузка товаров из маркетплейса ЛеманаПро через B2B API"
    }
}
