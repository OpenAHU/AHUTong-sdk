use std::fmt::format;
use crate::data::api::client::AHUClient;
use serde_json::Value;
use reqwest::{Result, Response};

const BASE_URL: &str = "https://jw.ahu.edu.cn";

impl AHUClient {
    // @GET("/student/sso/login")
    pub async fn fetch_login_info(&self) -> Result<(String, reqwest::Url)> {
        let resp = self.http
            .get(format!("{}/student/sso/login", BASE_URL))
            .send()
            .await?;
        
        let url = resp.url().clone();
        let text = resp.text().await?;
        
        Ok((text, url))
    }

    // @GET("/student/for-std/course-table/semester/{id}/print-data")
    pub async fn get_course(
        &self,
        semester_path_id: i32,
        semester_query_id: i32,
        has_experiment: bool
    ) -> Result<Value> {
        let url = format!("{}/student/for-std/course-table/semester/{}/print-data", BASE_URL, semester_path_id);
        self.http
            .get(url)
            .query(&[
                ("semesterId", semester_path_id.to_string()),
                ("hasExperiment", has_experiment.to_string())
            ])
            .send()
            .await?
            .json::<Value>()
            .await
    }

    // @GET("/student/for-std/course-table")
    pub async fn fetch_course_table_basic_info(&self) -> Result<String> {
        self.http
            .get(format!("{}/student/for-std/course-table", BASE_URL))
            .send()
            .await?
            .text()
            .await
    }

    // @GET("/student/home/get-current-teach-week")
    pub async fn get_current_teach_week(&self) -> Result<Value> {
        self.http
            .get(format!("{}/student/home/get-current-teach-week", BASE_URL))
            .send()
            .await?
            .json::<Value>()
            .await
    }

    // @GET("/student/for-std/exam-arrange")
    pub async fn get_exam_info(&self) -> Result<String> {
        self.http
            .get(format!("{}/student/for-std/exam-arrange", BASE_URL))
            .send()
            .await?
            .text()
            .await
    }

    pub async fn get_grade_sheet_entry_url(&self) -> Result<String> {
        let resp = self.http
            .get(format!("{}/student/for-std/grade/sheet", BASE_URL))
            .send()
            .await?;
        Ok(resp.url().to_string())
    }

    // @GET("/student/for-std/grade/sheet")
    // To retrieve a student's examInfo/grade, you need their ID
    // This interface return's student' grade, and it also returns student's ID via its redirect URL
    // So,before you get above data, you need access this interface to get student's ID
    pub async fn get_grade_sheet_entry(&self) -> Result<String> {
        self.http
            .get(format!("{}/student/for-std/grade/sheet", BASE_URL))
            .send()
            .await?
            .text()
            .await
    }

    // @GET("/student/for-std/grade/sheet/info/{id}")
    pub async fn get_grade_info(&self, id: &str) -> Result<Value> {
        self.http
            .get(format!("{}/student/for-std/grade/sheet/info/{}", BASE_URL, id))
            .send()
            .await?
            .json::<Value>()
            .await
    }

    // @POST device
    pub async fn device_login(
        &self,
        url: &str,
        username_len: usize,
        password_len: usize,
        rsa: &str
    ) -> Result<String> {
        let params = [
            ("ul", username_len.to_string()),
            ("pl", password_len.to_string()),
            ("rsa", rsa.to_string()),
            ("method", "login".to_string()),
        ];

        self.http
            .post(url)
            .form(&params)
            .send()
            .await?
            .text()
            .await
    }

    // @POST login
    pub async fn jwxt_login(
        &self,
        url: &str,
        rsa: &str,
        username_len: usize,
        password_len: usize,
        lt: &str,
        execution: &str,
        event_id: &str,
    ) -> Result<Response> {
        let params = [
            ("rsa", rsa.to_string()),
            ("ul", username_len.to_string()),
            ("pl", password_len.to_string()),
            ("lt", lt.to_string()),
            ("execution", execution.to_string()),
            ("_eventId", event_id.to_string()),
        ];

        self.http
            .post(url)
            .form(&params)
            .send()
            .await
    }
}
