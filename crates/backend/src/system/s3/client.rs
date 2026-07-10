use bytes::Bytes;
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE, HOST};
use reqwest::Method;
use sha2::{Digest, Sha256};

use crate::shared::config::S3Config;

type HmacSha256 = Hmac<Sha256>;

pub struct S3Object {
    pub bytes: Bytes,
    pub content_type: Option<String>,
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn hmac_sha256(key: &[u8], data: &str) -> anyhow::Result<Vec<u8>> {
    let mut mac = HmacSha256::new_from_slice(key)
        .map_err(|err| anyhow::anyhow!("Failed to initialize HMAC: {}", err))?;
    mac.update(data.as_bytes());
    Ok(mac.finalize().into_bytes().to_vec())
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    hex_lower(&Sha256::digest(bytes))
}

fn encode_key(key: &str) -> String {
    key.split('/')
        .map(|part| urlencoding::encode(part).into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn endpoint_base(config: &S3Config) -> String {
    config.endpoint.trim_end_matches('/').to_string()
}

fn request_url(config: &S3Config, key: &str) -> String {
    format!(
        "{}/{}/{}",
        endpoint_base(config),
        urlencoding::encode(&config.bucket),
        encode_key(key)
    )
}

fn signed_headers(
    config: &S3Config,
    method: &Method,
    key: &str,
    payload_hash: &str,
    content_type: Option<&str>,
) -> anyhow::Result<HeaderMap> {
    let endpoint = reqwest::Url::parse(&endpoint_base(config))?;
    let host = endpoint
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("S3 endpoint must include host"))?;

    let now = Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    let date_stamp = now.format("%Y%m%d").to_string();
    let canonical_uri = format!(
        "/{}/{}",
        urlencoding::encode(&config.bucket),
        encode_key(key)
    );

    let mut canonical_headers = format!(
        "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
        host, payload_hash, amz_date
    );
    let mut signed_headers = "host;x-amz-content-sha256;x-amz-date".to_string();

    if let Some(content_type) = content_type.filter(|value| !value.trim().is_empty()) {
        canonical_headers.push_str(&format!("content-type:{}\n", content_type));
        signed_headers.push_str(";content-type");
    }

    let canonical_request = format!(
        "{}\n{}\n\n{}\n{}\n{}",
        method.as_str(),
        canonical_uri,
        canonical_headers,
        signed_headers,
        payload_hash
    );
    let credential_scope = format!("{}/{}/s3/aws4_request", date_stamp, config.region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{}\n{}\n{}",
        amz_date,
        credential_scope,
        sha256_hex(canonical_request.as_bytes())
    );

    let date_key = hmac_sha256(
        format!("AWS4{}", config.secret_access_key).as_bytes(),
        &date_stamp,
    )?;
    let region_key = hmac_sha256(&date_key, &config.region)?;
    let service_key = hmac_sha256(&region_key, "s3")?;
    let signing_key = hmac_sha256(&service_key, "aws4_request")?;
    let signature = hex_lower(&hmac_sha256(&signing_key, &string_to_sign)?);

    let authorization = format!(
        "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
        config.access_key_id, credential_scope, signed_headers, signature
    );

    let mut headers = HeaderMap::new();
    headers.insert(HOST, HeaderValue::from_str(host)?);
    headers.insert("x-amz-date", HeaderValue::from_str(&amz_date)?);
    headers.insert("x-amz-content-sha256", HeaderValue::from_str(payload_hash)?);
    headers.insert(AUTHORIZATION, HeaderValue::from_str(&authorization)?);
    if let Some(content_type) = content_type.filter(|value| !value.trim().is_empty()) {
        headers.insert(CONTENT_TYPE, HeaderValue::from_str(content_type)?);
    }
    Ok(headers)
}

pub async fn put_object(
    config: &S3Config,
    key: &str,
    content_type: Option<&str>,
    bytes: Bytes,
) -> anyhow::Result<Option<String>> {
    let payload_hash = sha256_hex(&bytes);
    let headers = signed_headers(config, &Method::PUT, key, &payload_hash, content_type)?;
    let response = reqwest::Client::new()
        .put(request_url(config, key))
        .headers(headers)
        .body(bytes)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("S3 PUT failed with {}: {}", status, body));
    }

    Ok(response
        .headers()
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string))
}

async fn get_object_response(config: &S3Config, key: &str) -> anyhow::Result<reqwest::Response> {
    let payload_hash = sha256_hex(&[]);
    let headers = signed_headers(config, &Method::GET, key, &payload_hash, None)?;
    Ok(reqwest::Client::new()
        .get(request_url(config, key))
        .headers(headers)
        .send()
        .await?)
}

async fn read_object_body(response: reqwest::Response) -> anyhow::Result<S3Object> {
    let content_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(ToString::to_string);
    let bytes = response.bytes().await?;
    Ok(S3Object {
        bytes,
        content_type,
    })
}

pub async fn get_object(config: &S3Config, key: &str) -> anyhow::Result<S3Object> {
    let response = get_object_response(config, key).await?;
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("S3 GET failed with {}: {}", status, body));
    }
    read_object_body(response).await
}

/// Как `get_object`, но возвращает `None` вместо ошибки, если объекта ещё не существует (404) —
/// нужно для чтения `catalog.json` до самой первой публикации какого-либо плагина.
pub(crate) async fn get_object_opt(
    config: &S3Config,
    key: &str,
) -> anyhow::Result<Option<S3Object>> {
    let response = get_object_response(config, key).await?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!("S3 GET failed with {}: {}", status, body));
    }
    Ok(Some(read_object_body(response).await?))
}

pub async fn delete_object(config: &S3Config, key: &str) -> anyhow::Result<()> {
    let payload_hash = sha256_hex(&[]);
    let headers = signed_headers(config, &Method::DELETE, key, &payload_hash, None)?;
    let response = reqwest::Client::new()
        .delete(request_url(config, key))
        .headers(headers)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(anyhow::anyhow!(
            "S3 DELETE failed with {}: {}",
            status,
            body
        ));
    }

    Ok(())
}
