pub mod aggregate;

#[allow(dead_code)]
mod metadata_gen;
pub use metadata_gen::{ENTITY_METADATA, FIELDS};

pub use aggregate::{WbDocument, WbDocumentHeader, WbDocumentId, WbDocumentSourceMeta};
