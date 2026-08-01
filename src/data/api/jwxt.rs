use crate::data::api::client::AHUClient;
use anyhow::Result;
use reqwest::Response;
use serde_json::Value;

const BASE_URL: &str = "https://jw.ahu.edu.cn";

impl AHUClient {
    // @GET("/student/sso/login")
    pub async fn fetch_login_info(&self) -> reqwest::Result<(String, reqwest::Url)> {
        let resp = self
            .http
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
        _semester_query_id: i32,
        has_experiment: bool,
    ) -> Result<Value> {
        let url = format!(
            "{}/student/for-std/course-table/semester/{}/print-data",
            BASE_URL, semester_path_id
        );
        self.authenticated_json(self.http.get(url).query(&[
            ("semesterId", semester_path_id.to_string()),
            ("hasExperiment", has_experiment.to_string()),
        ]))
        .await
    }

    // @GET("/student/for-std/course-table")
    pub async fn fetch_course_table_basic_info(&self) -> Result<String> {
        self.authenticated_text(
            self.http
                .get(format!("{}/student/for-std/course-table", BASE_URL)),
        )
        .await
    }

    // @GET("/student/home/get-current-teach-week")
    pub async fn get_current_teach_week(&self) -> Result<Value> {
        self.authenticated_json(
            self.http
                .get(format!("{}/student/home/get-current-teach-week", BASE_URL)),
        )
        .await
    }

    // @GET("/student/for-std/exam-arrange")
    pub async fn get_exam_info(&self) -> Result<String> {
        self.authenticated_text(
            self.http
                .get(format!("{}/student/for-std/exam-arrange", BASE_URL)),
        )
        .await
    }

    pub async fn get_grade_sheet_entry_url(&self) -> Result<String> {
        let (_, final_url) = self
            .authenticated_page(
                self.http
                    .get(format!("{}/student/for-std/grade/sheet", BASE_URL)),
            )
            .await?;
        Ok(final_url.to_string())
    }

    pub async fn get_grade_sheet_entry_page(&self) -> Result<(String, String)> {
        let (html, final_url) = self
            .authenticated_page(
                self.http
                    .get(format!("{}/student/for-std/grade/sheet", BASE_URL)),
            )
            .await?;
        Ok((final_url.to_string(), html))
    }

    // @GET("/student/for-std/grade/sheet")
    // To retrieve a student's examInfo/grade, you need their ID
    // This interface return's student' grade, and it also returns student's ID via its redirect URL
    // So,before you get above data, you need access this interface to get student's ID
    pub async fn get_grade_sheet_entry(&self) -> Result<String> {
        self.authenticated_text(
            self.http
                .get(format!("{}/student/for-std/grade/sheet", BASE_URL)),
        )
        .await
    }

    // @GET("/student/for-std/grade/sheet/info/{id}")
    pub async fn get_grade_info(&self, id: &str) -> Result<Value> {
        self.authenticated_json(self.http.get(format!(
            "{}/student/for-std/grade/sheet/info/{}",
            BASE_URL, id
        )))
        .await
    }

    pub async fn get_gpa_rank_page(&self, id: &str) -> Result<String> {
        self.authenticated_text(self.http.get(format!(
            "{}/student/for-std/grade/sheet/semester-index/{}",
            BASE_URL, id
        )))
        .await
    }

    // @POST device
    pub async fn device_login(
        &self,
        url: &str,
        username_len: usize,
        password_len: usize,
        rsa: &str,
    ) -> reqwest::Result<String> {
        let params = [
            ("ul", username_len.to_string()),
            ("pl", password_len.to_string()),
            ("rsa", rsa.to_string()),
            ("method", "login".to_string()),
        ];

        self.http.post(url).form(&params).send().await?.text().await
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
    ) -> reqwest::Result<Response> {
        let params = [
            ("rsa", rsa.to_string()),
            ("ul", username_len.to_string()),
            ("pl", password_len.to_string()),
            ("lt", lt.to_string()),
            ("execution", execution.to_string()),
            ("_eventId", event_id.to_string()),
        ];

        self.http.post(url).form(&params).send().await
    }
}
