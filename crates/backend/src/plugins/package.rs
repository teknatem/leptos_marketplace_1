//! Переносимый файловый формат плагина (export/import).
//!
//! Единица переноса — `PluginBundle` (ключ идентичности — `manifest.code`); локальное
//! состояние ([`PluginDefinition`]) не переносится. Архив — zip с детерминированной
//! раскладкой (удобно для diff и ручной/LLM-правки):
//!
//! ```text
//! plugin.json        # конверт: schema_version + manifest + params + data + view_spec
//! client.js          # client_script (если есть)
//! server.js          # server_script (если есть)
//! styles.css         # styles (если есть)
//! sql/<name>.sql     # каждый sql_resource отдельным файлом
//! assets/<name>      # вложения
//! ```
//!
//! Сборка/разбор атомарны: архив собирается целиком в памяти и отдаётся только при
//! успехе; импорт восстанавливает bundle полностью до валидации и записи.

use contracts::plugins::{DataBinding, ParamSpec, PluginBundle, PluginManifest, ViewSpec};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

/// Версия формата конверта (отдельно от `manifest.api_version`, версионирующего API движка).
pub const SCHEMA_VERSION: u32 = 1;

const MANIFEST_FILE: &str = "plugin.json";
const CLIENT_FILE: &str = "client.js";
const SERVER_FILE: &str = "server.js";
const STYLES_FILE: &str = "styles.css";
const SQL_DIR: &str = "sql/";
const ASSETS_DIR: &str = "assets/";

/// Конверт `plugin.json` — метаданные бандла (скрипты/SQL/стили/вложения хранятся файлами).
#[derive(Serialize, Deserialize)]
struct Envelope {
    schema_version: u32,
    manifest: PluginManifest,
    #[serde(default)]
    params: Vec<ParamSpec>,
    #[serde(default)]
    data: DataBinding,
    #[serde(default)]
    view_spec: ViewSpec,
}

/// Собрать zip-архив переносимого бандла. Возвращает байты архива.
pub fn export_bundle(bundle: &PluginBundle) -> anyhow::Result<Vec<u8>> {
    let mut zip = ZipWriter::new(Cursor::new(Vec::<u8>::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let envelope = Envelope {
        schema_version: SCHEMA_VERSION,
        manifest: bundle.manifest.clone(),
        params: bundle.params.clone(),
        data: bundle.data.clone(),
        view_spec: bundle.view_spec.clone(),
    };
    zip.start_file(MANIFEST_FILE, options)?;
    zip.write_all(serde_json::to_string_pretty(&envelope)?.as_bytes())?;

    let mut write_text = |name: &str, content: &str| -> anyhow::Result<()> {
        zip.start_file(name, options)?;
        zip.write_all(content.as_bytes())?;
        Ok(())
    };

    if let Some(script) = bundle.client_script.as_deref().filter(|s| !s.is_empty()) {
        write_text(CLIENT_FILE, script)?;
    }
    if let Some(script) = bundle.server_script.as_deref().filter(|s| !s.is_empty()) {
        write_text(SERVER_FILE, script)?;
    }
    if let Some(styles) = bundle.styles.as_deref().filter(|s| !s.is_empty()) {
        write_text(STYLES_FILE, styles)?;
    }
    for (name, sql) in &bundle.sql_resources {
        write_text(&format!("{SQL_DIR}{name}.sql"), sql)?;
    }
    for (name, content) in &bundle.assets {
        write_text(&format!("{ASSETS_DIR}{name}"), content)?;
    }

    Ok(zip.finish()?.into_inner())
}

/// Разобрать zip-архив в `PluginBundle`. Не валидирует и не сохраняет.
pub fn import_archive(bytes: &[u8]) -> anyhow::Result<PluginBundle> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|e| anyhow::anyhow!("Не удалось открыть архив: {e}"))?;

    let mut files: HashMap<String, String> = HashMap::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index)?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        let mut raw = Vec::new();
        entry.read_to_end(&mut raw)?;
        files.insert(name, String::from_utf8_lossy(&raw).into_owned());
    }

    let envelope_raw = files
        .get(MANIFEST_FILE)
        .ok_or_else(|| anyhow::anyhow!("В архиве отсутствует {MANIFEST_FILE}"))?;
    let envelope: Envelope = serde_json::from_str(envelope_raw)
        .map_err(|e| anyhow::anyhow!("Некорректный {MANIFEST_FILE}: {e}"))?;
    if envelope.schema_version > SCHEMA_VERSION {
        anyhow::bail!(
            "Версия формата {} новее поддерживаемой ({SCHEMA_VERSION})",
            envelope.schema_version
        );
    }

    let mut sql_resources = HashMap::new();
    let mut assets = HashMap::new();
    for (name, content) in &files {
        if let Some(rest) = name.strip_prefix(SQL_DIR) {
            if let Some(key) = rest.strip_suffix(".sql") {
                sql_resources.insert(key.to_string(), content.clone());
            }
        } else if let Some(rest) = name.strip_prefix(ASSETS_DIR) {
            assets.insert(rest.to_string(), content.clone());
        }
    }

    Ok(PluginBundle {
        manifest: envelope.manifest,
        params: envelope.params,
        data: envelope.data,
        client_script: files.get(CLIENT_FILE).cloned(),
        server_script: files.get(SERVER_FILE).cloned(),
        view_spec: envelope.view_spec,
        styles: files.get(STYLES_FILE).cloned(),
        sql_resources,
        assets,
    })
}

/// Безопасное имя файла архива из кода плагина.
pub fn archive_filename(code: &str) -> String {
    let safe: String = code
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let safe = safe.trim_matches('_');
    let stem = if safe.is_empty() { "plugin" } else { safe };
    format!("{stem}.plugin.zip")
}

#[cfg(test)]
mod tests {
    use super::*;
    use contracts::plugins::PluginRuntime;

    fn sample_bundle() -> PluginBundle {
        PluginBundle {
            manifest: PluginManifest {
                code: "PLG-DEMO".into(),
                title: "Демо".into(),
                runtime: PluginRuntime::Hybrid,
                api_version: "2".into(),
                description: Some("desc".into()),
                capabilities: vec!["data:read".into()],
            },
            params: vec![],
            data: DataBinding::default(),
            client_script: Some("export async function mount(r,h){}".into()),
            server_script: Some("export async function loadReport(a,h){return [];}".into()),
            view_spec: ViewSpec::default(),
            styles: Some(".x{color:red}".into()),
            sql_resources: [("report".to_string(), "SELECT 1 AS v".to_string())]
                .into_iter()
                .collect(),
            assets: [("logo.svg".to_string(), "<svg/>".to_string())]
                .into_iter()
                .collect(),
        }
    }

    #[test]
    fn export_import_round_trip() {
        let original = sample_bundle();
        let bytes = export_bundle(&original).unwrap();
        let restored = import_archive(&bytes).unwrap();

        assert_eq!(restored.manifest.code, original.manifest.code);
        assert_eq!(restored.manifest.runtime, original.manifest.runtime);
        assert_eq!(restored.client_script, original.client_script);
        assert_eq!(restored.server_script, original.server_script);
        assert_eq!(restored.styles, original.styles);
        assert_eq!(restored.sql_resources, original.sql_resources);
        assert_eq!(restored.assets, original.assets);
    }

    #[test]
    fn import_rejects_archive_without_manifest() {
        // Пустой zip без plugin.json.
        let empty = ZipWriter::new(Cursor::new(Vec::<u8>::new()))
            .finish()
            .unwrap()
            .into_inner();
        let error = import_archive(&empty).unwrap_err();
        assert!(error.to_string().contains(MANIFEST_FILE));
    }

    #[test]
    fn archive_filename_is_sanitized() {
        assert_eq!(archive_filename("PLG WB/Orders"), "PLG_WB_Orders.plugin.zip");
    }
}
