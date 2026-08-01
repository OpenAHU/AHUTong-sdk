use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use log::{error, info};
use reqwest::header::{ACCEPT, CONNECTION, HOST, USER_AGENT};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant};

// Raw public key bytes derived from the SPKI:
// MCowBQYDK2VwAyEAsQ2Fz04RzJgfvt/dsExlo44l3RFQ4JAMHGRrAn9IXNk=
const PUBLIC_KEY_BYTES: [u8; 32] = [
    0xB1, 0x0D, 0x85, 0xCF, 0x4E, 0x11, 0xCC, 0x98, 0x1F, 0xBE, 0xDF, 0xDD, 0xB0, 0x4C, 0x65, 0xA3,
    0x8E, 0x25, 0xDD, 0x11, 0x50, 0xE0, 0x90, 0x0C, 0x1C, 0x64, 0x6B, 0x02, 0x7F, 0x48, 0x5C, 0xD9,
];

pub async fn download_and_verify_update(
    url: &str,
    save_path: &str,
    expected_sha256_hex: &str,
    signature_base64: &str,
) -> Result<()> {
    download_and_verify_update_with_progress(
        url,
        save_path,
        expected_sha256_hex,
        signature_base64,
        |_downloaded, _total| {},
    )
    .await
}

pub async fn download_and_verify_update_with_progress<F>(
    url: &str,
    save_path: &str,
    expected_sha256_hex: &str,
    signature_base64: &str,
    mut on_progress: F,
) -> Result<()>
where
    F: FnMut(u64, i64) + Send,
{
    let server_ip = "118.25.8.226";
    info!("update_download_started");

    let parsed = reqwest::Url::parse(url).map_err(|_| anyhow!("update_url_invalid"))?;
    let original_host = parsed
        .host_str()
        .ok_or_else(|| anyhow!("update_url_host_missing"))?
        .to_string();
    let ip_url = url.replace(&original_host, server_ip);

    // 1. Download file
    let client = reqwest::Client::builder()
        .user_agent("RustSdkHotUpdate/1.0 (Android)")
        .danger_accept_invalid_hostnames(true)
        // .danger_accept_invalid_certs(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(600)) // 10 minutes for large files (APK)
        .build()
        .map_err(|_| anyhow!("update_http_client_failed"))?;

    let mut resp = client
        .get(&ip_url)
        .header(USER_AGENT, "RustSdkHotUpdate/1.0 (Android)")
        .header(ACCEPT, "application/octet-stream")
        .header(CONNECTION, "close")
        .header(HOST, original_host.clone()) // ✅ 关键：让 nginx 按 server_name 路由
        .send()
        .await
        .map_err(|_| anyhow!("update_download_request_failed"))?;
    let response_status = resp.status();
    if !response_status.is_success() {
        error!(
            "update_download_http_failed status={}",
            response_status.as_u16()
        );
        return Err(anyhow!("update_download_http_failed"));
    }

    let total_len: i64 = resp.content_length().map(|v| v as i64).unwrap_or(-1);

    on_progress(0, total_len);

    // 2. Prepare file and hasher
    let mut file = File::create(save_path).map_err(|_| anyhow!("update_file_create_failed"))?;
    let mut hasher = Sha256::new();

    // 3. Stream download
    info!("Starting stream download...");
    let mut downloaded: u64 = 0;

    let mut last_emit_bytes: u64 = 0;
    let mut last_emit_time = Instant::now();

    while let Some(chunk) = resp
        .chunk()
        .await
        .map_err(|_| anyhow!("update_download_stream_failed"))?
    {
        file.write_all(&chunk)
            .map_err(|_| anyhow!("update_file_write_failed"))?;
        hasher.update(&chunk);
        downloaded += chunk.len() as u64;

        let bytes_delta = downloaded - last_emit_bytes;
        let time_delta = last_emit_time.elapsed();

        if bytes_delta >= 256 * 1024 || time_delta >= Duration::from_millis(200) {
            on_progress(downloaded, total_len);
            last_emit_bytes = downloaded;
            last_emit_time = Instant::now();
        }
    }

    file.flush()
        .map_err(|_| anyhow!("update_file_flush_failed"))?;
    file.sync_all().ok();
    drop(file);

    // 完成回调
    on_progress(downloaded, total_len);
    info!("Download complete. Total size: {} bytes", downloaded);

    // 4. Verify SHA256
    info!("Verifying SHA256...");
    let digest = hasher.finalize();
    let calculated_sha256_hex = hex::encode(&digest);

    if !calculated_sha256_hex.eq_ignore_ascii_case(expected_sha256_hex) {
        error!("update_sha256_mismatch");
        let _ = std::fs::remove_file(save_path);
        return Err(anyhow!("update_sha256_mismatch"));
    }

    // 3. Verify Signature
    info!("Verifying Signature...");
    // The Python server signs the SHA256 digest (raw bytes).
    let digest_bytes = digest.as_slice();

    let signature_bytes = BASE64
        .decode(signature_base64)
        .map_err(|_| anyhow!("update_signature_decode_failed"))?;

    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|_| anyhow!("update_signature_format_invalid"))?;

    let verifying_key = VerifyingKey::from_bytes(&PUBLIC_KEY_BYTES)
        .map_err(|_| anyhow!("update_public_key_invalid"))?;

    if verifying_key.verify(digest_bytes, &signature).is_err() {
        error!("update_signature_invalid");
        let _ = std::fs::remove_file(save_path);
        return Err(anyhow!("update_signature_invalid"));
    }
    info!("Signature verified.");
    info!("File saved and verified successfully.");

    Ok(())
}

// /api/check_apk_update
#[derive(Debug, Deserialize)]
pub struct ApkUpdateConfig {
    #[serde(rename = "versionCode")]
    pub version_code: i64,

    #[serde(rename = "versionName", default)]
    pub version_name: String,

    #[serde(default)]
    pub force: bool,

    pub url: String,
    pub sha256: String,
    pub signature: String,

    #[serde(default)]
    pub alg: String,

    #[serde(default)]
    pub changelog: String,
}

#[derive(Debug, Serialize)]
pub struct ApkUpdateInfo {
    pub update: bool,
    pub force: bool,

    #[serde(rename = "versionCode")]
    pub version_code: i64,

    #[serde(rename = "versionName")]
    pub version_name: String,

    pub changelog: String,

    pub url: Option<String>,
    pub sha256: Option<String>,
    pub signature: Option<String>,
}

pub async fn check_apk_update(current_version_code: i64) -> Result<ApkUpdateInfo> {
    info!(
        "Checking APK update. current_version_code: {}",
        current_version_code
    );
    let original_url = "https://openahu.org/api/check_apk_update";
    let server_ip = "118.25.8.226";
    let original_host = reqwest::Url::parse(original_url)
        .map_err(|_| anyhow!("update_config_url_invalid"))?
        .host_str()
        .unwrap_or("openahu.org")
        .to_string();

    let ip_url = original_url.replace(&original_host, server_ip);

    let client = reqwest::Client::builder()
        .user_agent("AHUTong/ApkUpdateCheck (Android)")
        .danger_accept_invalid_hostnames(true)
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|_| anyhow!("update_http_client_failed"))?;

    let resp = client
        .get(&ip_url)
        .header(USER_AGENT, "AHUTong/ApkUpdateCheck (Android)")
        .header(ACCEPT, "application/json")
        .header(CONNECTION, "close")
        .header(HOST, original_host.clone())
        .send()
        .await
        .map_err(|_| anyhow!("update_config_request_failed"))?;
    let response_status = resp.status();
    if !response_status.is_success() {
        error!(
            "update_config_http_failed status={}",
            response_status.as_u16()
        );
        return Err(anyhow!("update_config_http_failed"));
    }

    let text = resp
        .text()
        .await
        .map_err(|_| anyhow!("update_config_read_failed"))?;
    info!("update_config_received bytes={}", text.len());

    let cfg: ApkUpdateConfig =
        serde_json::from_str(&text).map_err(|_| anyhow!("update_config_parse_failed"))?;

    let update = cfg.version_code > current_version_code;
    info!(
        "update_config_evaluated remote_version_code={} available={} forced={}",
        cfg.version_code,
        update,
        cfg.force && update
    );

    Ok(ApkUpdateInfo {
        update,
        force: cfg.force && update,
        version_code: cfg.version_code,
        version_name: if cfg.version_name.is_empty() {
            cfg.version_code.to_string()
        } else {
            cfg.version_name
        },
        changelog: cfg.changelog,
        url: update.then_some(cfg.url),
        sha256: update.then_some(cfg.sha256),
        signature: update.then_some(cfg.signature),
    })
}
