// use crate::aggregates::customers::schema::CustomersSchema;
// use crate::aggregates::products::schema::ProductsSchema;
use crate::shared::data::schema::TableDefinition;
use std::borrow::Cow;

pub fn all_tables() -> Vec<TableDefinition> {
    let mut tables: Vec<TableDefinition> = Vec::new();
    // let customers_schema = CustomersSchema;
    // registry.insert(customers_schema.get_schema().name.clone(), Box::new(customers_schema));

    // let products_schema = ProductsSchema;
    // registry.insert(products_schema.get_schema().name.clone(), Box::new(products_schema));
    // connection_1c_database table
    tables.push(TableDefinition {
        name: Cow::Borrowed("connection_1c_database"),
        columns: vec![
            crate::shared::data::schema::ColumnDefinition {
                name: Cow::Borrowed("id"),
                column_type: crate::shared::data::schema::SimpleColumnType::Integer,
                nullable: false,
                primary_key: true,
                unique: true,
            },
            crate::shared::data::schema::ColumnDefinition {
                name: Cow::Borrowed("description"),
                column_type: crate::shared::data::schema::SimpleColumnType::String {
                    length: Some(255),
                },
                nullable: false,
                primary_key: false,
                unique: false,
            },
            crate::shared::data::schema::ColumnDefinition {
                name: Cow::Borrowed("url"),
                column_type: crate::shared::data::schema::SimpleColumnType::String {
                    length: Some(500),
                },
                nullable: false,
                primary_key: false,
                unique: false,
            },
            crate::shared::data::schema::ColumnDefinition {
                name: Cow::Borrowed("comment"),
                column_type: crate::shared::data::schema::SimpleColumnType::Text,
                nullable: true,
                primary_key: false,
                unique: false,
            },
            crate::shared::data::schema::ColumnDefinition {
                name: Cow::Borrowed("login"),
                column_type: crate::shared::data::schema::SimpleColumnType::String {
                    length: Some(100),
                },
                nullable: false,
                primary_key: false,
                unique: false,
            },
            crate::shared::data::schema::ColumnDefinition {
                name: Cow::Borrowed("password"),
                column_type: crate::shared::data::schema::SimpleColumnType::String {
                    length: Some(100),
                },
                nullable: false,
                primary_key: false,
                unique: false,
            },
            crate::shared::data::schema::ColumnDefinition {
                name: Cow::Borrowed("is_primary"),
                column_type: crate::shared::data::schema::SimpleColumnType::Boolean,
                nullable: false,
                primary_key: false,
                unique: false,
            },
            crate::shared::data::schema::ColumnDefinition {
                name: Cow::Borrowed("is_deleted"),
                column_type: crate::shared::data::schema::SimpleColumnType::Boolean,
                nullable: false,
                primary_key: false,
                unique: false,
            },
            crate::shared::data::schema::ColumnDefinition {
                name: Cow::Borrowed("created_at"),
                column_type: crate::shared::data::schema::SimpleColumnType::DateTime,
                nullable: true,
                primary_key: false,
                unique: false,
            },
            crate::shared::data::schema::ColumnDefinition {
                name: Cow::Borrowed("updated_at"),
                column_type: crate::shared::data::schema::SimpleColumnType::DateTime,
                nullable: true,
                primary_key: false,
                unique: false,
            },
        ],
    });
    tables
}
