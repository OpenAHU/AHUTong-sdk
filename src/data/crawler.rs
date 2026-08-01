use crate::data::api::client::AHUClient;
use crate::data::model::{Course, Exam, User};
use crate::utils::des::DES;
use crate::utils::parser::Parser;
use anyhow::{Context, Result, anyhow};
use log::{debug, error, info, warn};
use serde::Serialize;
use serde_json::Value;

const JWXT_HOME: &str = "https://jw.ahu.edu.cn/student/home";

pub struct Crawler {
    pub client: AHUClient,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GradeStudentProfile {
    pub id: String,
    pub training_type: String,
    pub department: String,
    pub major: String,
}

impl Crawler {
    pub fn new(client: AHUClient) -> Self {
        Self { client }
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<User> {
        info!("Starting login flow");
        let mut login_success_info: Option<Value> = None;
        let mut fetched_auth_code = false;
        for i in 0..5 {
            info!("ADWMH login attempt {}/5", i + 1);
            let auth_code_bytes = match self.client.get_auth_code().await {
                Ok(bytes) => {
                    fetched_auth_code = true;
                    bytes
                }
                Err(_) => {
                    warn!("Failed to get auth code on attempt {}", i + 1);
                    continue;
                }
            };
            debug!("Auth code bytes length: {}", auth_code_bytes.len());

            let captcha_res = self
                .client
                .get_captcha_result("https://118.25.8.226/ocr/captcha", auth_code_bytes.to_vec())
                .await;

            let captcha_code = match captcha_res {
                Ok(res) => res["result"].as_str().unwrap_or("").to_string(),
                Err(_) => {
                    warn!("OCR service request failed");
                    String::new()
                }
            };

            if captcha_code.is_empty() {
                warn!("Captcha recognition failed (empty result), retrying...");
                continue;
            }
            debug!("Captcha recognition succeeded");

            let adwmh_login = self
                .client
                .login_with_captcha(username, password, 0, &captcha_code)
                .await?;
            if adwmh_login["code"].as_i64() == Some(10000) {
                info!("ADWMH login successful");
                login_success_info = Some(adwmh_login);
                break;
            } else {
                warn!("ADWMH login rejected");
            }
        }

        // Master 分支逻辑：ADWMH 登录必须成功，否则整个登录失败
        let login_info = login_success_info.ok_or_else(|| {
            if !fetched_auth_code {
                anyhow!("auth_code_unavailable")
            } else {
                anyhow!("campus_login_rejected")
            }
        })?;

        let user_obj = &login_info["object"]["user"];
        let user = User {
            username: user_obj["userName"].as_str().unwrap_or("").to_string(),
            id_number: user_obj["idNumber"].as_str().unwrap_or("").to_string(),
        };

        info!("Fetching CAS login page...");
        let (login_page_html, current_url) = self.client.fetch_login_info().await?;

        // 检查是否已经登录 (重定向到了首页)
        let current_url_str = current_url.as_str();
        if current_url_str.ends_with("/student/home") || current_url_str == JWXT_HOME {
            info!("Already logged in (redirected to home). Skipping CAS login.");
            let verify = self.client.get_grade_sheet_entry().await;
            match verify {
                Ok(_) => {
                    info!("Session verification successful (Grade sheet accessible).");
                    return Ok(user);
                }
                Err(_) => {
                    warn!("Session verification failed; forcing re-login");
                    // 如果验证失败，说明 Cookie 其实是坏的，或者被软拦截了。
                    // 这里可以选择继续往下走（尝试 CAS 登录），或者报错。
                    // 继续往下走可能会因为没有 LT 参数而失败。
                    // 最好的办法是：清除 Cookie，重新开始？
                    // 暂时先让它继续往下走，看看 Parser 能不能解析出东西（大概率不能，因为在 Home 页）。
                }
            }
        }

        let (lt, execution, action) = match Parser::parse_cas_params(&login_page_html) {
            Some(params) => params,
            None => {
                // 如果解析失败，可能是因为 ADWMH 登录后 Cookie 已过期，CAS 页面不是预期的登录页
                // 或者是因为已经被重定向到了其他页面
                error!("CAS param parse failed");
                return Err(anyhow!("cas_parameter_parse_failed"));
            }
        };

        info!("CAS parameters extracted");

        // Kotlin 使用 length() (UTF-16 char count)，Rust len() 是 byte count。
        // 为了与 Java 行为完全一致（包括 Emoji 等 surrogate pairs），必须使用 UTF-16 code units count。
        let ul = username.encode_utf16().count();
        let pl = password.encode_utf16().count();

        let cipher = DES::str_enc(&format!("{}{}{}", username, password, lt), "1", "2", "3");

        info!("Performing device login...");
        self.client
            .device_login("https://one.ahu.edu.cn/cas/device", ul, pl, &cipher)
            .await
            .context("Device login failed")?;

        let login_url = if action.starts_with("http") {
            action
        } else {
            format!("https://one.ahu.edu.cn{}", action)
        };

        info!("Performing JWXT CAS login");
        let response = self
            .client
            .jwxt_login(&login_url, &cipher, ul, pl, &lt, &execution, "submit")
            .await
            .context("CAS login request failed")?;

        // if (jwxtResponse.raw().request.url.toString().endsWith(Constants.JWXT_HOME))
        let final_url = response.url().as_str();
        info!("JWXT login redirect received");

        if final_url == JWXT_HOME || final_url.ends_with("/student/home") {
            info!("JWXT login success verified");
            Ok(user)
        } else {
            warn!("JWXT login failed (redirect mismatch). Expected ending with /student/home");
            Err(anyhow!("jwxt_redirect_mismatch"))
        }
    }

    pub async fn get_schedule(&self) -> Result<Vec<Course>> {
        self.get_schedule_with_semester_offset(0).await
    }

    pub async fn get_next_schedule(&self) -> Result<Vec<Course>> {
        self.get_schedule_with_semester_offset(20).await
    }

    async fn get_schedule_with_semester_offset(&self, semester_offset: i32) -> Result<Vec<Course>> {
        info!("[RustSDKSchedule] Starting get_schedule...");
        self.client.log_current_cookies();

        let basic_info_html = self.client.fetch_course_table_basic_info().await?;
        info!(
            "[RustSDKSchedule] Fetched basic info HTML. Size: {}",
            basic_info_html.len()
        );

        let (semester_id, semester_name) = Parser::parse_current_semester(&basic_info_html)
            .ok_or_else(|| {
                error!("[RustSDKSchedule] Failed to parse current semester ID");
                anyhow!("Failed to parse current semester ID")
            })?;

        info!(
            "[RustSDKSchedule] Parsed semester: id={}, name={}",
            semester_id, semester_name
        );

        let target_semester_id = if semester_offset == 0 {
            semester_id
        } else {
            Parser::resolve_next_semester_id(&basic_info_html, semester_id, &semester_name)
                .unwrap_or_else(|| {
                    warn!(
                        "[RustSDKSchedule] Semester options unavailable; falling back to legacy offset {}",
                        semester_offset
                    );
                    semester_id + semester_offset
                })
        };
        info!(
            "[RustSDKSchedule] Resolved target semester id={}",
            target_semester_id
        );
        let course_json = self
            .client
            .get_course(target_semester_id, semester_id, false)
            .await?;
        debug!("[RustSDKSchedule] Course response received");

        // 检查返回的 JSON 中是否有错误提示
        if course_json
            .get("message")
            .and_then(|v| v.as_str())
            .is_some()
        {
            warn!("[RustSDKSchedule] Server returned a message");
        }

        let mut courses = Vec::new();

        let activities_opt = course_json
            .get("studentTableVms")
            .and_then(|v| v.get(0))
            .and_then(|v| v.get("activities"))
            .and_then(|v| v.as_array());

        // 如果 activities 为空，可能是因为 Master 分支使用了不同的学期 ID
        // 尝试检查是否获取到了正确的学期 ID。如果 semester_id 与当前实际学期不符，可能导致查不到课。
        // Master 分支代码：val courseTable = JwxtApi.API.getCourse(currentSemesterJson.id, currentSemesterJson.id)
        // SDK 代码：self.client.get_course(semester_id, semester_id, false)
        // 参数一致。

        if activities_opt.is_none() {
            error!(
                "[RustSDKSchedule] 'studentTableVms[0].activities' not found or not an array. JSON keys: {:?}",
                course_json
                    .as_object()
                    .map(|o| o.keys().collect::<Vec<_>>())
            );
            // 这里不直接报错，而是允许返回空列表，因为有些时候确实没课
            // return Err(anyhow!("Invalid course table JSON structure"));
        }

        if let Some(activities) = activities_opt {
            info!("[RustSDKSchedule] Found {} activities", activities.len());

            for (idx, act) in activities.iter().enumerate() {
                let mut week_indexes: Vec<i32> =
                    serde_json::from_value(act["weekIndexes"].clone()).unwrap_or_default();
                week_indexes.sort_unstable();

                if week_indexes.is_empty() {
                    warn!(
                        "[RustSDKSchedule] Activity {} has empty weekIndexes, skipping.",
                        idx
                    );
                    continue;
                }

                let start_week = *week_indexes.first().unwrap();
                let end_week = *week_indexes.last().unwrap();

                let start_unit = act["startUnit"].as_i64().unwrap_or(1);
                let end_unit = act["endUnit"].as_i64().unwrap_or(1);
                let length = end_unit - start_unit + 1;

                let teacher = if let Some(arr) = act["teacherNames"].as_array() {
                    arr.iter()
                        .map(|v| v.as_str().unwrap_or(""))
                        .collect::<Vec<_>>()
                        .join(",")
                } else {
                    act["teacherNames"].as_str().unwrap_or("").to_string()
                };

                let weekday_str = if let Some(n) = act["weekday"].as_i64() {
                    n.to_string()
                } else if let Some(s) = act["weekday"].as_str() {
                    s.to_string()
                } else {
                    act["weekday"].to_string() // Fallback, though risky if it's null or object
                };

                let lesson_id_str = if let Some(n) = act["lessonId"].as_i64() {
                    n.to_string()
                } else if let Some(s) = act["lessonId"].as_str() {
                    s.to_string()
                } else {
                    act["lessonId"].to_string()
                };

                let course = Course {
                    name: act["courseName"].as_str().unwrap_or("").to_string(),
                    teacher,
                    location: act["room"].as_str().unwrap_or("未知").to_string(),
                    week_indexes,
                    start_week: start_week.to_string(),
                    end_week: end_week.to_string(),
                    start_time: start_unit.to_string(),
                    length: length.to_string(),
                    weekday: weekday_str,
                    course_id: lesson_id_str,
                };
                // info!("[RustSDKSchedule] Parsed course: {}", course.name);
                courses.push(course);
            }
        } else {
            warn!("[RustSDKSchedule] No activities found in response.");
        }

        info!(
            "[RustSDKSchedule] Successfully parsed {} courses.",
            courses.len()
        );
        Ok(courses)
    }

    pub async fn get_exam_info(&self) -> Result<Vec<Exam>> {
        let html = self.client.get_exam_info().await?;

        // Try new HTML table format first (post-redesign: server-rendered <tr> elements)
        let exams = Parser::parse_exam_from_html(&html);
        if !exams.is_empty() {
            return Ok(exams);
        }

        // Fallback: old format with studentExamInfoVms JS variable
        let re = regex::Regex::new(r"(?s)studentExamInfoVms\s*=\s*(\[.*?\]);").unwrap();

        let json_str = re
            .captures(&html)
            .and_then(|caps| caps.get(1))
            .map(|m| m.as_str())
            .ok_or_else(|| anyhow!("Failed to extract exam info from HTML (neither table nor studentExamInfoVms found)"))?;

        let fixed_json = json_str.replace("'", "\"");

        let items: Vec<Value> =
            serde_json::from_str(&fixed_json).context("Failed to parse extracted JSON")?;

        let mut exams = Vec::new();
        for item in items {
            let course_name = item["course"]["nameZh"].as_str().unwrap_or("");
            let exam_type = item["examType"]["nameZh"].as_str().unwrap_or("");
            let course_display = format!("{}({})", course_name, exam_type);

            let time = item["examTime"].as_str().unwrap_or("").to_string();
            let seat_num = item["seatNo"].to_string();

            let campus = item["requiredCampus"]["nameZh"].as_str().unwrap_or("");
            let room = item["room"].as_str().unwrap_or("");
            let location = format!("{}-{}", campus, room);

            let finished = item["finished"].as_bool().unwrap_or(false);

            exams.push(Exam {
                course: course_display,
                time,
                seat_num,
                location,
                finished,
            });
        }
        Ok(exams)
    }

    pub async fn get_grade(&self, student_id: Option<String>) -> Result<Value> {
        let id = match student_id {
            Some(id) => id,
            None => {
                // 自动获取 ID
                let url = self.client.get_grade_sheet_entry_url().await?;
                let parts: Vec<&str> = url.split('/').collect();
                parts
                    .last()
                    .map(|s| s.to_string())
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| anyhow!("Failed to extract Student ID from URL: {}", url))?
            }
        };

        let grade_json = self.client.get_grade_info(&id).await?;
        Ok(grade_json)
    }

    pub async fn get_grade_profiles(&self) -> Result<Vec<GradeStudentProfile>> {
        let (url, html) = self.client.get_grade_sheet_entry_page().await?;
        if let Some(id) = numeric_last_path_component(&url) {
            return Ok(vec![GradeStudentProfile {
                id,
                training_type: "主修".to_string(),
                department: String::new(),
                major: String::new(),
            }]);
        }
        Ok(parse_grade_profiles(&html))
    }

    pub async fn get_gpa_rank(&self, student_id: &str) -> Result<Value> {
        let html = self.client.get_gpa_rank_page(student_id).await?;
        parse_gpa_rank_model(&html)
    }

    pub async fn get_qrcode(&self) -> Result<Value> {
        Ok(self.client.get_qrcode().await?)
    }

    pub async fn get_balance(&self) -> Result<Value> {
        Ok(self.client.get_balance().await?)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn download_calendar(&self, save_path: &str) -> anyhow::Result<()> {
        let url = "https://openahu.org/download/xiaoli.jpg";

        info!("Crawler: Start downloading calendar.");

        // --- 第一步：建立网络连接 ---
        info!("Crawler: Sending HTTP GET request...");
        let response = match self.client.http.get(url).send().await {
            Ok(resp) => {
                info!("Crawler: Connection established.");
                resp
            }
            Err(e) => {
                error!("Crawler: Network Request FAILED. Could not connect to server.");
                if e.is_timeout() {
                    error!("Crawler: Reason: Connection Timed Out.");
                } else if e.is_connect() {
                    error!("Crawler: Reason: Connection Refused/DNS Error.");
                }
                return Err(e.into());
            }
        };

        // --- 第二步：检查 HTTP 状态码 ---
        let status = response.status();
        info!("Crawler: Server returned HTTP Status: {}", status);

        if !status.is_success() {
            error!("Crawler: Server refused the request!");
            return Err(anyhow::anyhow!(
                "HTTP Request failed with status: {}",
                status
            ));
        }

        // --- 第三步：下载数据 ---
        info!("Crawler: Reading response body bytes...");
        let content_length = response.content_length().unwrap_or(0);
        info!("Crawler: Expected content length: {} bytes", content_length);

        let bytes = match response.bytes().await {
            Ok(b) => {
                info!("Crawler: Successfully downloaded {} bytes.", b.len());
                b
            }
            Err(e) => {
                error!("Crawler: Failed to read body bytes from stream.");
                return Err(e.into());
            }
        };

        // --- 第四步：文件写入 ---
        // 检查父目录是否存在，这在 Android 上很重要
        let path_obj = std::path::Path::new(save_path);
        if let Some(parent) = path_obj.parent() {
            if !parent.exists() {
                info!("Crawler: Parent directory is missing; attempting to create it.");
                if tokio::fs::create_dir_all(parent).await.is_err() {
                    error!("Crawler: Failed to create parent directory.");
                    // 这里不return，尝试直接写试试，或者直接报错
                }
            }
        }

        info!("Crawler: Writing bytes to file system...");
        match tokio::fs::write(save_path, &bytes).await {
            Ok(_) => {
                info!("Crawler: File write operation returned OK.");
                // 二次确认文件是否存在
                if path_obj.exists() {
                    info!("Crawler: VERIFICATION SUCCESS: Output file exists.");
                } else {
                    warn!("Crawler: VERIFICATION WARNING: Write returned OK but file not found!");
                }
                Ok(())
            }
            Err(e) => {
                error!("Crawler: File Write FAILED.");
                // 常见的 IO 错误分析
                match e.kind() {
                    std::io::ErrorKind::PermissionDenied => error!(
                        "Crawler: Reason: Permission Denied. Check Android Manifest/Storage Scopes."
                    ),
                    std::io::ErrorKind::NotFound => error!("Crawler: Reason: Directory Not Found."),
                    _ => error!("Crawler: Reason: Other IO Error."),
                }
                Err(e.into())
            }
        }
    }
}

fn numeric_last_path_component(url: &str) -> Option<String> {
    let component = url.trim_end_matches('/').rsplit('/').next()?;
    (!component.is_empty() && component.chars().all(|value| value.is_ascii_digit()))
        .then(|| component.to_string())
}

fn parse_grade_profiles(html: &str) -> Vec<GradeStudentProfile> {
    let panel =
        regex::Regex::new(r#"(?is)class\s*=\s*[\"'][^\"']*student-panel-block[^\"']*[\"']"#)
            .expect("grade profile panel regex");
    let value = regex::Regex::new(r#"(?is)<button[^>]*onclick\s*=\s*[\"'][^\"']*myFunction[^\"']*[\"'][^>]*value\s*=\s*[\"']([^\"']+)[\"']|<button[^>]*value\s*=\s*[\"']([^\"']+)[\"'][^>]*onclick\s*=\s*[\"'][^\"']*myFunction"#)
        .expect("grade profile id regex");
    let dd = regex::Regex::new(r"(?is)<dd[^>]*>(.*?)</dd>").expect("grade profile field regex");
    let tag = regex::Regex::new(r"(?is)<[^>]+>").expect("html tag regex");
    let mut starts: Vec<usize> = panel.find_iter(html).map(|item| item.start()).collect();
    starts.push(html.len());

    starts
        .windows(2)
        .filter_map(|bounds| {
            let block = &html[bounds[0]..bounds[1]];
            let captures = value.captures(block)?;
            let id = captures
                .get(1)
                .or_else(|| captures.get(2))?
                .as_str()
                .trim()
                .to_string();
            let fields: Vec<String> = dd
                .captures_iter(block)
                .take(3)
                .map(|capture| {
                    tag.replace_all(capture.get(1).map_or("", |item| item.as_str()), "")
                        .trim()
                        .to_string()
                })
                .collect();
            Some(GradeStudentProfile {
                id,
                training_type: fields.first().cloned().unwrap_or_default(),
                department: fields.get(1).cloned().unwrap_or_default(),
                major: fields.get(2).cloned().unwrap_or_default(),
            })
        })
        .collect()
}

fn parse_gpa_rank_model(html: &str) -> Result<Value> {
    let pattern = regex::Regex::new(r"(?s)var\s+gpaSemesterModel\s*=\s*(\{.*?\});")?;
    let object = pattern
        .captures(html)
        .and_then(|captures| captures.get(1))
        .ok_or_else(|| anyhow!("gpaSemesterModel was not found"))?
        .as_str()
        .replace('\'', "\"");
    serde_json::from_str(&object).context("Failed to parse gpaSemesterModel")
}

#[cfg(test)]
mod grade_tests {
    use super::{numeric_last_path_component, parse_gpa_rank_model, parse_grade_profiles};

    #[test]
    fn parses_multiple_grade_profiles() {
        let html = r#"
        <div class="student-panel-block"><dd>主修</dd><dd>计算机学院</dd><dd>软件工程</dd>
        <button onclick="myFunction(this)" value="122850">成绩</button></div>
        <div class="student-panel-block"><dd>微专业</dd><dd>创新学院</dd><dd>人工智能</dd>
        <button value="122851" onclick="myFunction(this)">成绩</button></div>"#;
        let profiles = parse_grade_profiles(html);
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[1].id, "122851");
        assert_eq!(profiles[1].major, "人工智能");
    }

    #[test]
    fn parses_redirect_and_rank_model() {
        assert_eq!(
            numeric_last_path_component(
                "https://jw.ahu.edu.cn/student/for-std/grade/sheet/info/122850"
            ),
            Some("122850".into())
        );
        let rank = parse_gpa_rank_model(
            "<script>var gpaSemesterModel = {'gpa':3.8,'majorRank':5};</script>",
        )
        .unwrap();
        assert_eq!(rank["majorRank"], 5);
    }

    #[test]
    fn diagnostics_never_log_response_payloads_or_local_paths() {
        let source = include_str!("crawler.rs");
        for fragments in [
            ("Course JSON:", " {:?}"),
            ("Server returned message:", " {}"),
            ("Server Error Body:", " {}"),
            ("Local Save Path:", " {}"),
            ("Network Error Details:", " {:?}"),
            ("Stream Error Details:", " {:?}"),
            ("IO Error Details:", " {:?}"),
            ("File exists at", " {}"),
        ] {
            let forbidden = format!("{}{}", fragments.0, fragments.1);
            assert!(
                !source.contains(&forbidden),
                "forbidden diagnostic: {forbidden}"
            );
        }
    }
}
