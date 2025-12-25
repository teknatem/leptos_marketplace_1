//! Build script for generating metadata.rs files from metadata.json
//!
//! This script scans the domain directory for metadata.json files and generates
//! corresponding metadata_gen.rs files with static Rust constants.

use serde::Deserialize;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=src/domain");

    let domain_dir = Path::new("src/domain");

    if !domain_dir.exists() {
        println!("cargo:warning=Domain directory not found, skipping metadata generation");
        return;
    }

    for entry in fs::read_dir(domain_dir).expect("Failed to read domain directory") {
        let path = entry.expect("Failed to read entry").path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = path.file_name().unwrap().to_str().unwrap();
        if dir_name == "common" {
            continue;
        }

        let metadata_json = path.join("metadata.json");
        if metadata_json.exists() {
            println!("cargo:rerun-if-changed={}", metadata_json.display());

            let output_rs = path.join("metadata_gen.rs");
            match generate_metadata(&metadata_json, &output_rs) {
                Ok(_) => {
                    println!("cargo:warning=Generated: {}", output_rs.display());
                }
                Err(e) => {
                    panic!(
                        "Failed to generate metadata for {}: {}",
                        dir_name, e
                    );
                }
            }
        }
    }
}

// ============================================================================
// JSON Schema Types (owned Strings for serde deserialization)
// ============================================================================

#[derive(Debug, Deserialize)]
struct MetadataJson {
    schema_version: String,
    entity_type: String,
    entity_name: String,
    entity_index: String,
    collection_name: String,
    table_name: Option<String>,
    ui: UiMetadataJson,
    ai: AiMetadataJson,
    fields: Vec<FieldJson>,
}

#[derive(Debug, Deserialize)]
struct UiMetadataJson {
    element_name: String,
    element_name_en: Option<String>,
    list_name: String,
    list_name_en: Option<String>,
    icon: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AiMetadataJson {
    description: String,
    #[serde(default)]
    questions: Vec<String>,
    #[serde(default)]
    related: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FieldJson {
    name: String,
    rust_type: String,
    field_type: String,
    #[serde(default)]
    source: String,
    ui: FieldUiJson,
    #[serde(default)]
    validation: ValidationJson,
    ai_hint: Option<String>,
    #[allow(dead_code)]
    nested_fields: Option<Vec<FieldJson>>,
    ref_aggregate: Option<String>,
    enum_values: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Default)]
struct FieldUiJson {
    #[serde(default)]
    label: String,
    label_en: Option<String>,
    placeholder: Option<String>,
    hint: Option<String>,
    #[serde(default = "default_true")]
    visible_in_list: bool,
    #[serde(default = "default_true")]
    visible_in_form: bool,
    widget: Option<String>,
    column_width: Option<u32>,
}

#[derive(Debug, Deserialize, Default)]
struct ValidationJson {
    #[serde(default)]
    required: bool,
    min: Option<f64>,
    max: Option<f64>,
    min_length: Option<usize>,
    max_length: Option<usize>,
    pattern: Option<String>,
    custom_error: Option<String>,
}

fn default_true() -> bool {
    true
}

// ============================================================================
// Code Generation
// ============================================================================

fn generate_metadata(
    json_path: &Path,
    output_path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let json_content = fs::read_to_string(json_path)?;
    let metadata: MetadataJson = serde_json::from_str(&json_content)?;

    let code = generate_rust_code(&metadata);
    fs::write(output_path, code)?;

    Ok(())
}

fn generate_rust_code(meta: &MetadataJson) -> String {
    let mut code = String::new();

    // Header
    code.push_str(&format!(
        "// ============================================================================\n\
         // AUTO-GENERATED FROM metadata.json - DO NOT EDIT MANUALLY\n\
         // Generated: {}\n\
         // ============================================================================\n\n",
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ")
    ));

    // Imports
    code.push_str(
        "#![allow(dead_code)]\n\n\
         use crate::shared::metadata::{\n\
         \x20   EntityMetadataInfo, EntityType, EntityUiMetadata, EntityAiMetadata,\n\
         \x20   FieldMetadata, FieldType, FieldSource, FieldUiMetadata, ValidationRules\n\
         };\n\n",
    );

    // Entity metadata constant
    code.push_str(&generate_entity_metadata(meta));
    code.push_str("\n\n");

    // Fields array constant
    code.push_str(&generate_fields_array(&meta.fields));

    code
}

fn generate_entity_metadata(meta: &MetadataJson) -> String {
    format!(
        "/// Entity metadata for {} {}\n\
         pub const ENTITY_METADATA: EntityMetadataInfo = EntityMetadataInfo {{\n\
         \x20   schema_version: \"{}\",\n\
         \x20   entity_type: EntityType::{},\n\
         \x20   entity_name: \"{}\",\n\
         \x20   entity_index: \"{}\",\n\
         \x20   collection_name: \"{}\",\n\
         \x20   table_name: {},\n\
         \x20   ui: EntityUiMetadata {{\n\
         \x20       element_name: \"{}\",\n\
         \x20       element_name_en: {},\n\
         \x20       list_name: \"{}\",\n\
         \x20       list_name_en: {},\n\
         \x20       icon: {},\n\
         \x20   }},\n\
         \x20   ai: EntityAiMetadata {{\n\
         \x20       description: \"{}\",\n\
         \x20       questions: &[{}],\n\
         \x20       related: &[{}],\n\
         \x20   }},\n\
         }};",
        meta.entity_name,
        meta.entity_type,
        meta.schema_version,
        to_pascal_case(&meta.entity_type),
        meta.entity_name,
        meta.entity_index,
        meta.collection_name,
        option_str(&meta.table_name),
        escape_string(&meta.ui.element_name),
        option_str(&meta.ui.element_name_en),
        escape_string(&meta.ui.list_name),
        option_str(&meta.ui.list_name_en),
        option_str(&meta.ui.icon),
        escape_string(&meta.ai.description),
        string_array(&meta.ai.questions),
        string_array(&meta.ai.related),
    )
}

fn generate_fields_array(fields: &[FieldJson]) -> String {
    let mut code = String::from("/// Field metadata array\npub const FIELDS: &[FieldMetadata] = &[\n");

    for field in fields {
        code.push_str(&generate_field_metadata(field, 1));
        code.push_str(",\n");
    }

    code.push_str("];\n");
    code
}

fn generate_field_metadata(field: &FieldJson, indent: usize) -> String {
    let i = "    ".repeat(indent);
    format!(
        "{i}FieldMetadata {{\n\
         {i}    name: \"{}\",\n\
         {i}    rust_type: \"{}\",\n\
         {i}    field_type: FieldType::{},\n\
         {i}    source: FieldSource::{},\n\
         {i}    ui: FieldUiMetadata {{\n\
         {i}        label: \"{}\",\n\
         {i}        label_en: {},\n\
         {i}        placeholder: {},\n\
         {i}        hint: {},\n\
         {i}        visible_in_list: {},\n\
         {i}        visible_in_form: {},\n\
         {i}        widget: {},\n\
         {i}        column_width: {},\n\
         {i}    }},\n\
         {i}    validation: ValidationRules {{\n\
         {i}        required: {},\n\
         {i}        min: {},\n\
         {i}        max: {},\n\
         {i}        min_length: {},\n\
         {i}        max_length: {},\n\
         {i}        pattern: {},\n\
         {i}        custom_error: {},\n\
         {i}    }},\n\
         {i}    ai_hint: {},\n\
         {i}    nested_fields: None,\n\
         {i}    ref_aggregate: {},\n\
         {i}    enum_values: {},\n\
         {i}}}",
        field.name,
        field.rust_type,
        to_pascal_case(&field.field_type),
        to_pascal_case(if field.source.is_empty() {
            "specific"
        } else {
            &field.source
        }),
        escape_string(&field.ui.label),
        option_str(&field.ui.label_en),
        option_str(&field.ui.placeholder),
        option_str(&field.ui.hint),
        field.ui.visible_in_list,
        field.ui.visible_in_form,
        option_str(&field.ui.widget),
        option_u32(field.ui.column_width),
        field.validation.required,
        option_f64(field.validation.min),
        option_f64(field.validation.max),
        option_usize(field.validation.min_length),
        option_usize(field.validation.max_length),
        option_str(&field.validation.pattern),
        option_str(&field.validation.custom_error),
        option_str(&field.ai_hint),
        option_str(&field.ref_aggregate),
        option_str_array(&field.enum_values),
        i = i
    )
}

// ============================================================================
// Helper functions
// ============================================================================

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().chain(chars).collect(),
            }
        })
        .collect()
}

fn option_str(opt: &Option<String>) -> String {
    match opt {
        Some(s) => format!("Some(\"{}\")", escape_string(s)),
        None => "None".to_string(),
    }
}

fn option_u32(opt: Option<u32>) -> String {
    match opt {
        Some(v) => format!("Some({})", v),
        None => "None".to_string(),
    }
}

fn option_f64(opt: Option<f64>) -> String {
    match opt {
        Some(v) => format!("Some({:.1})", v),
        None => "None".to_string(),
    }
}

fn option_usize(opt: Option<usize>) -> String {
    match opt {
        Some(v) => format!("Some({})", v),
        None => "None".to_string(),
    }
}

fn option_str_array(opt: &Option<Vec<String>>) -> String {
    match opt {
        Some(arr) if !arr.is_empty() => format!("Some(&[{}])", string_array(arr)),
        _ => "None".to_string(),
    }
}

fn string_array(arr: &[String]) -> String {
    if arr.is_empty() {
        return String::new();
    }
    arr.iter()
        .map(|s| format!("\"{}\"", escape_string(s)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

