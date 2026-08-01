use crate::data::api::client::AHUClient;
use anyhow::{Result, anyhow};
use regex::Regex;
use urlencoding::decode;

pub struct AuthManager {
    client: AHUClient,
}

fn extract_ticket(final_url: &str) -> Result<&str> {
    let re = Regex::new(r"[?&]ticket=([^&]+)").expect("valid ticket regex");
    re.captures(final_url)
        .and_then(|captures| captures.get(1))
        .map(|value| value.as_str())
        .ok_or_else(|| anyhow!("ycard_ticket_missing"))
}

impl AuthManager {
    pub fn new(client: AHUClient) -> Self {
        Self { client }
    }

    pub async fn refresh_token(&self) -> Result<String> {
        let final_url = self.client.ycard_login_redirect(None).await?;

        let ticket = extract_ticket(final_url.as_str())?;

        // URLDecoder.decode(URLDecoder.decode(ticket, "UTF-8"), "UTF-8")
        let decoded_once = decode(ticket)
            .map_err(|_| anyhow!("ycard_ticket_decode_failed"))?
            .into_owned();
        let decoded_username = decode(&decoded_once)
            .map_err(|_| anyhow!("ycard_ticket_decode_failed"))?
            .into_owned();

        // 解码后的 username 作为 username 和 password 获取 Token
        let token_res = self
            .client
            .get_token(&decoded_username, &decoded_username)
            .await?;

        let access_token = token_res["access_token"]
            .as_str()
            .ok_or_else(|| anyhow!("ycard_token_missing"))?
            .to_string();

        // get_token 方法内部已经写入了 client.ycard_token，这里返回它仅供参考
        Ok(access_token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ticket_error_never_contains_the_credential_url() {
        let final_url =
            "https://ycard.ahu.edu.cn/redirect?synjones-auth=secret&next=%2526ticket%253DST-secret";
        let error = extract_ticket(final_url).unwrap_err().to_string();

        assert_eq!(error, "ycard_ticket_missing");
        assert!(!error.contains("secret"));
        assert!(!error.contains("ticket="));
        assert!(!error.contains("ycard.ahu.edu.cn"));
    }
}
