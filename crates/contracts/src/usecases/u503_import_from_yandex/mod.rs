pub mod events;
pub mod progress;
pub mod request;
pub mod response;

pub use events::ImportEvent;
pub use progress::{AggregateProgress, ImportProgress};
pub use request::ImportRequest;
pub use response::ImportResponse;

use crate::usecases::common::UseCaseMetadata;

pub struct ImportFromYandex;

impl UseCaseMetadata for ImportFromYandex {
    fn usecase_index() -> &'static str {
        "u503"
    }

    fn usecase_name() -> &'static str {
        "import_from_yandex"
    }

    fn display_name() -> &'static str {
        "Импорт из Yandex Market"
    }

    fn description() -> &'static str {
        "Загрузка товаров из маркетплейса Yandex Market через API"
    }
}
