use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

const PUBLIC_INTERNAL_ERROR: &str = "campus_service_error";
const PUBLIC_UNAVAILABLE_ERROR: &str = "campus_service_unavailable";

enum PublicError {
    Unauthorized,
    Unavailable,
    Internal,
}

fn classify(error: &anyhow::Error) -> PublicError {
    match error.to_string().as_str() {
        "campus_login_rejected" => PublicError::Unauthorized,
        "auth_code_unavailable"
        | "cas_parameter_parse_failed"
        | "jwxt_redirect_mismatch"
        | "ycard_redirect_failed"
        | "ycard_ticket_missing"
        | "ycard_ticket_decode_failed"
        | "ycard_token_exchange_failed"
        | "ycard_token_missing" => PublicError::Unavailable,
        _ => PublicError::Internal,
    }
}

#[derive(Debug)]
pub enum AppError {
    Anyhow(anyhow::Error),
    Status(StatusCode),
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AppError::Anyhow(error) => match classify(&error) {
                PublicError::Unauthorized => StatusCode::UNAUTHORIZED.into_response(),
                PublicError::Unavailable => {
                    log::warn!("Local campus service dependency unavailable");
                    let body = Json(ErrorBody {
                        error: PUBLIC_UNAVAILABLE_ERROR.to_string(),
                    });
                    (StatusCode::SERVICE_UNAVAILABLE, body).into_response()
                }
                PublicError::Internal => {
                    log::error!("Local campus service request failed");
                    let body = Json(ErrorBody {
                        error: PUBLIC_INTERNAL_ERROR.to_string(),
                    });
                    (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
                }
            },
            AppError::Status(code) => code.into_response(),
        }
    }
}

// Explicitly implement From<anyhow::Error> instead of generic E to avoid conflict
impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self::Anyhow(e)
    }
}

// This handles ? on Result<T, anyhow::Error> automatically?
// No, ? uses Into<AppError>.
// If I have Result<T, anyhow::Error>, e.into() -> AppError.
// So From<anyhow::Error> is enough for that.

impl From<StatusCode> for AppError {
    fn from(s: StatusCode) -> Self {
        Self::Status(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn internal_error_response_never_exposes_upstream_details() {
        let secret =
            "https://ycard.ahu.edu.cn/redirect?ticket=ST-secret&synjones-auth=token-secret";
        let response = AppError::Anyhow(anyhow::anyhow!(secret)).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read response body");
        let text = String::from_utf8(body.to_vec()).expect("utf8 JSON body");
        assert_eq!(text, r#"{"error":"campus_service_error"}"#);
        assert!(!text.contains("ST-secret"));
        assert!(!text.contains("token-secret"));
        assert!(!text.contains("ticket="));
    }

    #[tokio::test]
    async fn known_auth_failures_keep_safe_status_semantics() {
        let unauthorized =
            AppError::Anyhow(anyhow::anyhow!("campus_login_rejected")).into_response();
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

        let unavailable = AppError::Anyhow(anyhow::anyhow!("ycard_ticket_missing")).into_response();
        assert_eq!(unavailable.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = to_bytes(unavailable.into_body(), usize::MAX)
            .await
            .expect("read response body");
        assert_eq!(
            String::from_utf8(body.to_vec()).expect("utf8 JSON body"),
            r#"{"error":"campus_service_unavailable"}"#
        );
    }
}
