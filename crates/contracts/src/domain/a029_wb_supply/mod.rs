pub mod aggregate;
mod metadata_gen;

pub use aggregate::{
    WbSupply, WbSupplyHeader, WbSupplyId, WbSupplyInfo, WbSupplyOrderRow, WbSupplySourceMeta,
};

pub use metadata_gen::{ENTITY_METADATA, FIELDS};
