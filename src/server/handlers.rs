use axum::{
    extract::{Json, Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use serde::Deserialize;

use crate::core;
use crate::server::dto::*;
use crate::server::error::AppError;

#[derive(Clone)]
pub struct AppState {
    pub token: String,
}

/// 校验 token：避免其他 App 扫描 127.0.0.1 调用接口
fn check_token(headers: &HeaderMap, state: &AppState) -> Result<(), StatusCode> {
    let got = headers
        .get("X-AHUTONG-TOKEN")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if got == state.token {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

pub async fn init(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<InitReq>,
) -> Result<impl IntoResponse, AppError> {
    check_token(&headers, &state).map_err(|s| anyhow::anyhow!("unauthorized: {s}"))?;
    core::load_or_clear_cookies(&req.cookies_json);
    Ok((StatusCode::OK, Json(serde_json::json!({"ok": true}))))
}

pub async fn dump_cookies(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    check_token(&headers, &state).map_err(|s| anyhow::anyhow!("unauthorized: {s}"))?;
    let cookies = core::dump_cookies_json();
    Ok((StatusCode::OK, Json(serde_json::json!({ "cookies": cookies }))))
}

pub async fn cookies_flat(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    check_token(&headers, &state).map_err(|s| anyhow::anyhow!("unauthorized: {s}"))?;
    let json = core::cookies_flat_json();
    Ok((StatusCode::OK, Json(serde_json::from_str::<serde_json::Value>(&json)?)))
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LoginReq>,
) -> Result<impl IntoResponse, AppError> {
    check_token(&headers, &state).map_err(|s| anyhow::anyhow!("unauthorized: {s}"))?;
    let user = core::crawler().login(&req.username, &req.password).await?;
    Ok((StatusCode::OK, Json(user)))
}

pub async fn schedule(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    check_token(&headers, &state).map_err(|s| anyhow::anyhow!("unauthorized: {s}"))?;
    let courses = core::crawler().get_schedule().await?;
    Ok((StatusCode::OK, Json(courses)))
}

pub async fn exam(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    check_token(&headers, &state).map_err(|s| anyhow::anyhow!("unauthorized: {s}"))?;
    let exams = core::crawler().get_exam_info().await?;
    Ok((StatusCode::OK, Json(exams)))
}

#[derive(Debug, Deserialize)]
pub struct GradeQuery {
    pub student_id: Option<String>,
}

pub async fn grade(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<GradeQuery>,
) -> Result<impl IntoResponse, AppError> {
    check_token(&headers, &state).map_err(|s| anyhow::anyhow!("unauthorized: {s}"))?;
    let v = core::crawler().get_grade(q.student_id).await?;
    Ok((StatusCode::OK, Json(v)))
}

pub async fn balance(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    check_token(&headers, &state).map_err(|s| anyhow::anyhow!("unauthorized: {s}"))?;
    let v = core::crawler().get_balance().await?;
    Ok((StatusCode::OK, Json(v)))
}

pub async fn qrcode(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    check_token(&headers, &state).map_err(|s| anyhow::anyhow!("unauthorized: {s}"))?;
    let v = core::crawler().get_qrcode().await?;
    Ok((StatusCode::OK, Json(v)))
}

pub async fn refresh_token(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError> {
    check_token(&headers, &state).map_err(|s| anyhow::anyhow!("unauthorized: {s}"))?;
    let token = core::auth_manager().refresh_token().await?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "access_token": token }))))
}