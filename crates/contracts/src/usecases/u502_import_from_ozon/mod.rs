pub mod request;
pub mod response;
pub mod events;
pub mod progress;

pub use request::ImportRequest;
pub use response::ImportResponse;
pub use events::ImportEvent;
pub use progress::{ImportProgress, AggregateProgress};

use crate::usecases::common::UseCaseMetadata;

pub struct ImportFromOzon;

impl UseCaseMetadata for ImportFromOzon {
    fn usecase_index() -> &'static str {
        "u502"
    }

    fn usecase_name() -> &'static str {
        "import_from_ozon"
    }

    fn display_name() -> &'static str {
        "Импорт из OZON"
    }

    fn description() -> &'static str {
        "Загрузка товаров из маркетплейса OZON через API"
    }
}
