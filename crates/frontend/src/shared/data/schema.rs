use std::borrow::Cow;

#[derive(Clone, Debug)]
pub enum SimpleColumnType {
    Integer,
    BigInteger,
    Float,
    Double,
    Decimal {
        precision: Option<u32>,
        scale: Option<u32>,
    },
    String {
        length: Option<u32>,
    },
    Text,
    Boolean,
    DateTime,
    Json,
}

#[derive(Clone, Debug)]
pub struct ColumnDefinition {
    pub name: Cow<'static, str>,
    pub column_type: SimpleColumnType,
    pub nullable: bool,
    pub primary_key: bool,
    pub unique: bool,
}

#[derive(Clone, Debug)]
pub struct TableDefinition {
    pub name: Cow<'static, str>,
    pub columns: Vec<ColumnDefinition>,
}

pub trait AggregateSchemaProvider {
    fn tables() -> Vec<TableDefinition>;
}
