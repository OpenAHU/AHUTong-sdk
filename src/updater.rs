use anyhow::{Context, Result};
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
    info!(
        "Starting download_and_verify_update. url={}, save_path={}",
        url, save_path
    );

    let parsed = reqwest::Url::parse(url).context("Failed to parse download url")?;
    let original_host = parsed
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Download url has no host"))?
        .to_string();
    let ip_url = url.replace(&original_host, server_ip);

    // 1. Download file
    info!("Downloading file from ip_url: {}", ip_url);
    let client = reqwest::Client::builder()
        .user_agent("RustSdkHotUpdate/1.0 (Android)")
        .danger_accept_invalid_hostnames(true)
        // .danger_accept_invalid_certs(true)
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(600)) // 10 minutes for large files (APK)
        .build()
        .context("Failed to build reqwest client")?;

    let mut resp = client
        .get(&ip_url)
        .header(USER_AGENT, "RustSdkHotUpdate/1.0 (Android)")
        .header(ACCEPT, "application/octet-stream")
        .header(CONNECTION, "close")
        .header(HOST, original_host.clone()) // ✅ 关键：让 nginx 按 server_name 路由
        .send()
        .await
        .with_context(|| format!("Failed to download file. ipUrl={}", ip_url))?
        .error_for_status()
        .context("HTTP status is not success")?;

    let total_len: i64 = resp.content_length().map(|v| v as i64).unwrap_or(-1);

    on_progress(0, total_len);

    // 2. Prepare file and hasher
    info!("Creating file at {}", save_path);
    let mut file = File::create(save_path).context("Failed to create file")?;
    let mut hasher = Sha256::new();

    // 3. Stream download
    info!("Starting stream download...");
    let mut downloaded: u64 = 0;

    let mut last_emit_bytes: u64 = 0;
    let mut last_emit_time = Instant::now();

    while let Some(chunk) = resp.chunk().await.context("Failed to read chunk")? {
        file.write_all(&chunk)
            .context("Failed to write chunk to file")?;
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

    file.flush().context("Failed to flush file")?;
    file.sync_all().ok();
    drop(file);

    // 完成回调
    on_progress(downloaded, total_len);
    info!("Download complete. Total size: {} bytes", downloaded);

    // 4. Verify SHA256
    info!("Verifying SHA256...");
    let digest = hasher.finalize();
    let calculated_sha256_hex = hex::encode(&digest);
    info!(
        "SHA256 calculated: {}, expected: {}",
        calculated_sha256_hex, expected_sha256_hex
    );

    if !calculated_sha256_hex.eq_ignore_ascii_case(expected_sha256_hex) {
        error!("SHA256 mismatch! Deleting invalid file.");
        let _ = std::fs::remove_file(save_path);
        return Err(anyhow::anyhow!(
            "SHA256 mismatch. Expected: {}, Calculated: {}",
            expected_sha256_hex,
            calculated_sha256_hex
        ));
    }

    // 3. Verify Signature
    info!("Verifying Signature...");
    // The Python server signs the SHA256 digest (raw bytes).
    let digest_bytes = digest.as_slice();

    let signature_bytes = BASE64
        .decode(signature_base64)
        .context("Failed to decode base64 signature")?;

    let signature = Signature::from_slice(&signature_bytes).context("Invalid signature format")?;

    let verifying_key =
        VerifyingKey::from_bytes(&PUBLIC_KEY_BYTES).context("Invalid public key")?;

    if let Err(e) = verifying_key.verify(digest_bytes, &signature) {
        error!("Signature verification failed! Deleting invalid file.");
        let _ = std::fs::remove_file(save_path);
        return Err(anyhow::anyhow!("Signature verification failed: {:?}", e));
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
        .context("parse original apk update url failed")?
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
        .context("Failed to build reqwest client")?;

    let resp = client
        .get(&ip_url)
        .header(USER_AGENT, "AHUTong/ApkUpdateCheck (Android)")
        .header(ACCEPT, "application/json")
        .header(CONNECTION, "close")
        .header(HOST, original_host.clone())
        .send()
        .await
        .with_context(|| format!("Failed to request apk update config, url={}", ip_url))?
        .error_for_status()
        .context("APK update config http status not success")?;

    let text = resp.text().await.context("Failed to read response text")?;
    info!("APK update config response: {}", text);

    let cfg: ApkUpdateConfig =
        serde_json::from_str(&text).context("Failed to parse apk update config JSON")?;

    info!("Parsed config: {:?}", cfg);

    let update = cfg.version_code > current_version_code;
    info!("Update available: {}", update);

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
