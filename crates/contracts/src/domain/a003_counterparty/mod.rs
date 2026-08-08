pub mod aggregate;
mod metadata_gen;

pub use aggregate::{Counterparty, CounterpartyDto, CounterpartyId};

pub use metadata_gen::{ENTITY_METADATA, FIELDS};
