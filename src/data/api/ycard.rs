use crate::data::api::client::AHUClient;
use reqwest::{RequestBuilder, Response, Result};
use serde_json::Value;

const BASE_URL: &str = "https://ycard.ahu.edu.cn";

impl AHUClient {
    // @Interceptor
    async fn with_auth(&self, builder: RequestBuilder) -> RequestBuilder {
        let token_guard = self.ycard_token.read().await;
        if let Some(token) = &*token_guard {
            builder.header("Synjones-Auth", format!("bearer {}", token))
        } else {
            builder
        }
    }

    // @GET("/berserker-auth/cas/redirect/neusoftCas")
    pub async fn ycard_login_redirect(&self, target_url: Option<&str>) -> Result<Response> {
        let target = target_url.unwrap_or("https://ycard.ahu.edu.cn/plat/?name=loginTransit");
        self.http
            .get(format!(
                "{}/berserker-auth/cas/redirect/neusoftCas",
                BASE_URL
            ))
            .query(&[("targetUrl", target)])
            .send()
            .await
    }

    // @GET("/berserker-app/ykt/tsm/queryCard") -> @GET("/campus-card/")
    pub async fn load_card_recharge(&self) -> Result<String> {
        let token = {
            let token_guard = self.ycard_token.read().await;
            token_guard.as_deref().unwrap_or("").to_string()
        };

        self.http
            .get(format!("{}/campus-card/", BASE_URL))
            .query(&[
                ("name", "cardRecharge"),
                ("appId", "27"),
                ("synAccessSource", "h5"),
                ("synjones-auth", &token),
            ])
            .send()
            .await?
            .text()
            .await
    }

    // @POST("/charge/order/thirdOrder")
    pub async fn get_order_third_data(&self, body: &Value) -> Result<String> {
        let builder = self
            .http
            .post(format!("{}/charge/order/thirdOrder", BASE_URL))
            .json(body);

        self.with_auth(builder).await.send().await?.text().await
    }

    // @POST("/charge/feeitem/getThirdData")
    pub async fn get_fee_item_third_data(&self, body: &Value) -> Result<String> {
        let builder = self
            .http
            .post(format!("{}/charge/feeitem/getThirdData", BASE_URL))
            .json(body);

        self.with_auth(builder).await.send().await?.text().await
    }

    // @POST("/blade-pay/pay")
    pub async fn pay(&self, body: &Value) -> Result<String> {
        let builder = self
            .http
            .post(format!("{}/blade-pay/pay", BASE_URL))
            .json(body);

        self.with_auth(builder).await.send().await?.text().await
    }

    // @POST("/berserker-auth/oauth/token")
    pub async fn get_token(&self, username: &str, password: &str) -> Result<Value> {
        let params = [
            ("username", username),
            ("password", password),
            ("grant_type", "password"),
            ("scope", "all"),
            ("loginFrom", "h5"),
            ("logintype", "sso"),
            ("device_token", "h5"),
            ("synAccessSource", "h5"),
        ];

        let auth_header =
            "Basic bW9iaWxlX3NlcnZpY2VfcGxhdGZvcm06bW9iaWxlX3NlcnZpY2VfcGxhdGZvcm1fc2VjcmV0";

        let response: Value = self
            .http
            .post(format!("{}/berserker-auth/oauth/token", BASE_URL))
            .header("Authorization", auth_header)
            .form(&params)
            .send()
            .await?
            .json()
            .await?;

        if let Some(access_token) = response.get("access_token").and_then(|v| v.as_str()) {
            let mut token_guard = self.ycard_token.write().await;
            *token_guard = Some(access_token.to_string());
        }

        Ok(response)
    }
    /*
    username=&password=&grant_type=password&scope=all&loginFrom=h5&logintype=sso&device_token=h5&synAccessSource=h5
    */
}
