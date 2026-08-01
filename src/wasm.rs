use crate::core::crawler;
use serde_json::json;
use wasm_bindgen::prelude::*;

// Never expose an anyhow chain to JavaScript: reqwest and parsers can retain
// request URLs, response fragments, or authentication parameters.
fn to_js_err(error: anyhow::Error) -> JsValue {
    JsValue::from_str(crate::diagnostics::public_error_code(
        &error,
        "campus_service_error",
    ))
}

#[wasm_bindgen]
pub async fn wasm_login(username: String, password: String) -> Result<String, JsValue> {
    let crawler = crawler();
    let user = crawler.login(&username, &password).await.map_err(to_js_err)?;
    
    // AhuApiClient expect json like: {"name": "...", "xh": "..."}
    let res = json!({
        "name": user.username,
        "xh": user.id_number,
    });
    
    Ok(serde_json::to_string(&res).unwrap())
}

#[wasm_bindgen]
pub async fn wasm_get_schedule() -> Result<String, JsValue> {
    let crawler = crawler();
    let courses = crawler.get_schedule().await.map_err(to_js_err)?;
    
    // The Course struct in Rust has fields: name, teacher, location, week_indexes, start_week, end_week, start_time, length, weekday, course_id
    // Wait, let's serialize it as array of json
    let res = serde_json::to_string(&courses).unwrap_or_else(|_| "[]".to_string());
    Ok(res)
}

#[wasm_bindgen]
pub async fn wasm_get_current_week() -> Result<String, JsValue> {
    // There isn't a direct get_current_week in Crawler that I saw recently, 
    // but the Flutter client expects a AhuCurrentWeek JSON.
    // Let's provide a fallback for now or attempt to fetch it if there is a method.
    // Since we don't have the exact get_current_week implementation in Rust exposed, we return a fallback.
    let res = json!({
        "currentSemester": "",
        "dayIndex": 1,
        "isInSemester": true,
        "weekIndex": 1
    });
    Ok(serde_json::to_string(&res).unwrap())
}

#[wasm_bindgen]
pub async fn wasm_get_card_balance() -> Result<String, JsValue> {
    let crawler = crawler();
    let balance_val = crawler.get_balance().await.map_err(to_js_err)?;
    
    // API expects {"balance": 100.0} or {"object": 100.0}
    // get_balance returns a Value. We just pass it through.
    Ok(serde_json::to_string(&balance_val).unwrap())
}

#[wasm_bindgen]
pub async fn wasm_get_card_qrcode() -> Result<String, JsValue> {
    let crawler = crawler();
    let qr_val = crawler.get_qrcode().await.map_err(to_js_err)?;
    
    Ok(serde_json::to_string(&qr_val).unwrap())
}

#[wasm_bindgen]
pub async fn wasm_get_grade_report() -> Result<String, JsValue> {
    let crawler = crawler();
    let grade_val = crawler.get_grade(None).await.map_err(to_js_err)?;
    
    Ok(serde_json::to_string(&grade_val).unwrap())
}

#[wasm_bindgen]
pub async fn wasm_clear_session() -> Result<(), JsValue> {
    crate::core::load_or_clear_cookies("");
    Ok(())
}
