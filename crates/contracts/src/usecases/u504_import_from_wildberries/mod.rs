pub mod request;
pub mod response;
pub mod events;
pub mod progress;

pub use request::ImportRequest;
pub use response::ImportResponse;
pub use events::ImportEvent;
pub use progress::{ImportProgress, AggregateProgress};

use crate::usecases::common::UseCaseMetadata;

pub struct ImportFromWildberries;

impl UseCaseMetadata for ImportFromWildberries {
    fn usecase_index() -> &'static str {
        "u504"
    }

    fn usecase_name() -> &'static str {
        "import_from_wildberries"
    }

    fn display_name() -> &'static str {
        "Импорт из Wildberries"
    }

    fn description() -> &'static str {
        "Загрузка товаров из маркетплейса Wildberries через API"
    }
}
