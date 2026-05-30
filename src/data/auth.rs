use crate::data::api::client::AHUClient;
use anyhow::{Context, Result, anyhow};
use regex::Regex;
use urlencoding::decode;

pub struct AuthManager {
    client: AHUClient,
}

impl AuthManager {
    pub fn new(client: AHUClient) -> Self {
        Self { client }
    }

    pub async fn refresh_token(&self) -> Result<String> {
        let response = self
            .client
            .ycard_login_redirect(None)
            .await
            .context("Failed to request ycard login redirect")?;

        let final_url = response.url().as_str();

        // Regex("[?&]ticket=([^&]+)")
        let re = Regex::new(r"[?&]ticket=([^&]+)").unwrap();
        let ticket = re
            .captures(final_url)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str())
            .ok_or_else(|| anyhow!("Failed to extract ticket from URL: {}", final_url))?;

        // URLDecoder.decode(URLDecoder.decode(ticket, "UTF-8"), "UTF-8")
        let decoded_once = decode(ticket)?.into_owned();
        let decoded_username = decode(&decoded_once)?.into_owned();

        // 解码后的 username 作为 username 和 password 获取 Token
        let token_res = self
            .client
            .get_token(&decoded_username, &decoded_username)
            .await
            .context("Failed to exchange ticket for token")?;

        let access_token = token_res["access_token"]
            .as_str()
            .ok_or_else(|| anyhow!("Token response missing access_token"))?
            .to_string();

        // get_token 方法内部已经写入了 client.ycard_token，这里返回它仅供参考
        Ok(access_token)
    }
}
