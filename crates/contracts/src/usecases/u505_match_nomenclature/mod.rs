pub mod events;
pub mod progress;
pub mod request;
pub mod response;

pub use events::MatchEvent;
pub use progress::MatchProgress;
pub use request::MatchRequest;
pub use response::MatchResponse;

use crate::usecases::common::UseCaseMetadata;

pub struct MatchNomenclature;

impl UseCaseMetadata for MatchNomenclature {
    fn usecase_index() -> &'static str {
        "u505"
    }

    fn usecase_name() -> &'static str {
        "match_nomenclature"
    }

    fn display_name() -> &'static str {
        "Сопоставление номенклатуры"
    }

    fn description() -> &'static str {
        "Автоматическое сопоставление товаров маркетплейсов с номенклатурой 1С по артикулу"
    }
}
