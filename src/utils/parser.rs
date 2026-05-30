use crate::data::model::Exam;
use log::{error, info, warn};
use regex::Regex;
use serde::Deserialize;
use std::sync::OnceLock;

#[derive(Deserialize, Debug)]
struct CurrentSemester {
    id: i32,
    name: String,
}

pub struct Parser;

impl Parser {
    pub fn parse_current_semester(html: &str) -> Option<(i32, String)> {
        // 定义一个通用的提取器，作用于任意文本片段（无论是整个 HTML 还是 Script 块）
        let extract_from_text = |text: &str| -> Option<(i32, String)> {
            // 尝试多种正则模式匹配 currentSemester
            // 模式 1: var currentSemester = {...}  (对象字面量)
            // 模式 2: var currentSemester = JSON.parse('...') (JSON 字符串)

            static RE_OBJ: OnceLock<Regex> = OnceLock::new();
            // 放宽对结尾分号的限制，并使用非贪婪匹配
            let re_obj = RE_OBJ
                .get_or_init(|| Regex::new(r"(?s)var\s+currentSemester\s*=\s*(\{.*?\})").unwrap());

            static RE_JSON: OnceLock<Regex> = OnceLock::new();
            // 匹配 JSON.parse('...') 中的内容，支持单引号或双引号包裹
            // 注意：原始字符串中双引号不需要转义，但单引号和双引号在正则字符类中可以直接写
            let re_json = RE_JSON.get_or_init(|| {
                Regex::new(r#"(?s)var\s+currentSemester\s*=\s*JSON\.parse\(\s*['"](.*?)['"]\s*\)"#)
                    .unwrap()
            });

            // 辅助函数：尝试解析 JSON 字符串
            let try_parse_json = |json_str: &str| -> Option<(i32, String)> {
                // 1. 标准解析
                if let Ok(sem) = serde_json::from_str::<CurrentSemester>(json_str) {
                    info!(
                        "[RustSDKSchedule] JSON parse success: id={}, name={}",
                        sem.id, sem.name
                    );
                    return Some((sem.id, sem.name));
                }

                // 2. 修复转义引号 (针对 JSON.parse('{\"id\":...}') 的情况)
                let fixed_json = json_str.replace("\\\"", "\"");
                if let Ok(sem) = serde_json::from_str::<CurrentSemester>(&fixed_json) {
                    info!(
                        "[RustSDKSchedule] Fixed JSON parse success: id={}, name={}",
                        sem.id, sem.name
                    );
                    return Some((sem.id, sem.name));
                }

                // 3. 针对非标准 JSON (单引号 key) 的容错处理
                // 日志显示: {'approvedYear':'2025','calendarAssoc':{'id':1}
                // 这不是合法的 JSON (Key 使用了单引号)，serde_json 会失败。
                // 我们需要将其转换为标准 JSON (双引号 Key)，或者直接回退到正则提取。

                // 尝试将单引号替换为双引号 (简单替换可能误伤 value 中的单引号，但这里值得一试)
                // 还要注意：value 中的单引号会被误伤。
                // 比如：{'name': 'I\'m happy'} -> {"name": "I"m happy"} -> 依然非法
                // 但考虑到教务系统的 key 通常比较简单，我们先尝试这种简单修复
                let single_quote_fixed = json_str.replace("'", "\"");
                if let Ok(sem) = serde_json::from_str::<CurrentSemester>(&single_quote_fixed) {
                    info!(
                        "[RustSDKSchedule] Single-quote fixed JSON parse success: id={}, name={}",
                        sem.id, sem.name
                    );
                    return Some((sem.id, sem.name));
                }

                // 尝试更智能的替换：只替换 key 的单引号？
                // 实际上正则提取已经覆盖了 id 和 name，所以如果这里失败了，下面的正则提取会兜底。
                // 所以我们不需要做太复杂的 JSON 修复。

                warn!(
                    "[RustSDKSchedule] JSON parsing failed. Content: {}",
                    json_str
                );

                // 4. 如果 JSON 解析彻底失败，尝试在这一小段内容中直接正则提取 id 和 name
                // 因为我们已经提取到了 currentSemester 的值片段，哪怕它不是合法 JSON，只要包含 id: ... 就能提取
                // 注意：日志中的 id 是没有引号的：'id':1
                // 我们的正则 (?:id|'id'|"id") 已经覆盖了这种情况
                static ID_RE: OnceLock<Regex> = OnceLock::new();
                let id_re =
                    ID_RE.get_or_init(|| Regex::new(r#"(?:id|'id'|"id")\s*:\s*(\d+)"#).unwrap());

                if let Some(cap) = id_re.captures(json_str) {
                    if let Ok(id) = cap[1].parse::<i32>() {
                        static NAME_RE: OnceLock<Regex> = OnceLock::new();
                        // 支持 name: '...', name: "...", "name": "..."
                        // 还要支持单引号 key: 'name':'...'
                        let name_re = NAME_RE.get_or_init(|| {
                            Regex::new(r#"(?:name|'name'|"name")\s*:\s*['"]([^'"]+)['"]"#).unwrap()
                        });

                        let name = if let Some(name_cap) = name_re.captures(json_str) {
                            name_cap[1].to_string()
                        } else {
                            "2024-2025-1".to_string()
                        };
                        info!(
                            "[RustSDKSchedule] Regex extraction from snippet success: id={}, name={}",
                            id, name
                        );
                        return Some((id, name));
                    }
                }

                None
            };

            // 1. 尝试匹配 JSON.parse('...')
            if let Some(caps) = re_json.captures(text) {
                if let Some(content) = caps.get(1) {
                    info!("[RustSDKSchedule] Matched JSON.parse pattern");
                    if let Some(res) = try_parse_json(content.as_str()) {
                        return Some(res);
                    }
                }
            }

            // 2. 尝试匹配对象字面量 {...}
            if let Some(caps) = re_obj.captures(text) {
                if let Some(content) = caps.get(1) {
                    info!("[RustSDKSchedule] Matched object literal pattern");
                    if let Some(res) = try_parse_json(content.as_str()) {
                        return Some(res);
                    }
                }
            }

            // 3. 暴力提取：在 var currentSemester ... 后的 1000 字符内查找 id: ...
            // 注意：有时候 currentSemester 定义在 semesters 之前，有时候在之后，或者它们之间隔了很远
            // 为了稳妥，我们不再依赖 "var currentSemester" 这个定位锚点，而是直接在整个文本块中搜索符合特征的 JSON 结构
            // 但为了避免匹配到 semesters 里的 id，我们需要更小心

            // 策略 A: 尝试在文本块中搜索 "currentSemester" 关键字，然后在其后搜索
            if let Some(start_idx) = text.find("var currentSemester") {
                let end_idx = std::cmp::min(start_idx + 1000, text.len());
                let snippet = &text[start_idx..end_idx];

                // 匹配 id: 123 或 "id": 123
                static ID_RE: OnceLock<Regex> = OnceLock::new();
                let id_re = ID_RE.get_or_init(|| Regex::new(r#"(?:id|"id")\s*:\s*(\d+)"#).unwrap());

                if let Some(cap) = id_re.captures(snippet) {
                    if let Ok(id) = cap[1].parse::<i32>() {
                        // 匹配 name: "..." 或 "name": "..."
                        static NAME_RE: OnceLock<Regex> = OnceLock::new();
                        let name_re = NAME_RE.get_or_init(|| {
                            Regex::new(r#"(?:name|"name")\s*:\s*"([^"]+)""#).unwrap()
                        });

                        let name = if let Some(name_cap) = name_re.captures(snippet) {
                            name_cap[1].to_string()
                        } else {
                            "2024-2025-1".to_string()
                        };
                        info!(
                            "[RustSDKSchedule] Regex fallback extraction success: id={}, name={}",
                            id, name
                        );
                        return Some((id, name));
                    }
                }
            } else {
                // 策略 B: 如果没找到 var currentSemester（可能是混淆或者压缩了），
                // 但我们在 script 块里，且该块包含 semesters，说明这大概率就是目标块。
                // 我们可以尝试搜索独立的 id 和 name 模式，但排除掉在数组中的（semesters 是个数组）
                // 这比较危险，容易误判。暂时只记录日志。
                warn!("[RustSDKSchedule] 'var currentSemester' not found in this script block.");
            }

            None
        };

        // 1. 优先在 Script 块中查找
        static SCRIPT_RE: OnceLock<Regex> = OnceLock::new();
        let script_re =
            SCRIPT_RE.get_or_init(|| Regex::new(r"(?s)<script[^>]*>(.*?)</script>").unwrap());

        let mut script_found = false;
        for cap in script_re.captures_iter(html) {
            let content = &cap[1];
            // 只有当同时包含这两个关键词时才处理，模拟 Master 分支的逻辑
            if content.contains("var semesters") && content.contains("var currentSemester") {
                script_found = true;
                info!(
                    "[RustSDKSchedule] Found target script block (len: {})",
                    content.len()
                );
                // 打印一部分内容以便调试
                let preview_len = std::cmp::min(content.len(), 500);
                info!(
                    "[RustSDKSchedule] Script Content Head: {}",
                    &content[..preview_len]
                );

                // 尝试提取 currentSemester 对象的完整内容
                // 由于正则匹配嵌套括号非常困难，且容易被非贪婪匹配截断，我们改用“平衡括号提取法”
                // 找到 "var currentSemester =" 后，向后扫描，找到第一个 '{'，然后计数大括号，直到平衡。

                let parse_balanced_json = |start_search_text: &str| -> Option<(i32, String)> {
                    if let Some(start_var) = start_search_text.find("var currentSemester") {
                        let after_var = &start_search_text[start_var..];
                        if let Some(start_brace) = after_var.find('{') {
                            let mut balance = 0;
                            let mut end_brace = 0;
                            let mut found = false;
                            let json_start_slice = &after_var[start_brace..];

                            for (i, c) in json_start_slice.char_indices() {
                                if c == '{' {
                                    balance += 1;
                                } else if c == '}' {
                                    balance -= 1;
                                    if balance == 0 {
                                        end_brace = i + 1; // 包含这个 }
                                        found = true;
                                        break;
                                    }
                                }
                            }

                            if found {
                                let json_str = &json_start_slice[..end_brace];
                                // info!("[RustSDKSchedule] Extracted balanced JSON string (len={}): {:.100}...", json_str.len(), json_str);

                                // 尝试标准解析
                                if let Ok(sem) = serde_json::from_str::<CurrentSemester>(json_str) {
                                    info!(
                                        "[RustSDKSchedule] Balanced JSON parse success: id={}, name={}",
                                        sem.id, sem.name
                                    );
                                    return Some((sem.id, sem.name));
                                }

                                // 尝试单引号修复
                                let single_quote_fixed = json_str.replace("'", "\"");
                                if let Ok(sem) =
                                    serde_json::from_str::<CurrentSemester>(&single_quote_fixed)
                                {
                                    info!(
                                        "[RustSDKSchedule] Balanced Single-quote fixed JSON parse success: id={}, name={}",
                                        sem.id, sem.name
                                    );
                                    return Some((sem.id, sem.name));
                                }

                                // 如果 JSON 解析依然失败，我们在这个完整的字符串里做正则提取
                                // 这比在截断的字符串里提取要安全得多
                                static ID_RE: OnceLock<Regex> = OnceLock::new();
                                let id_re = ID_RE.get_or_init(|| {
                                    Regex::new(r#"(?:id|'id'|"id")\s*:\s*(\d+)"#).unwrap()
                                });

                                if let Some(cap) = id_re.captures(json_str) {
                                    if let Ok(id) = cap[1].parse::<i32>() {
                                        static NAME_RE: OnceLock<Regex> = OnceLock::new();
                                        let name_re = NAME_RE.get_or_init(|| {
                                            Regex::new(
                                                r#"(?:name|'name'|"name")\s*:\s*['"]([^'"]+)['"]"#,
                                            )
                                            .unwrap()
                                        });

                                        let name =
                                            if let Some(name_cap) = name_re.captures(json_str) {
                                                name_cap[1].to_string()
                                            } else {
                                                "2024-2025-1".to_string()
                                            };
                                        info!(
                                            "[RustSDKSchedule] Balanced JSON Regex extraction success: id={}, name={}",
                                            id, name
                                        );
                                        return Some((id, name));
                                    }
                                }
                            } else {
                                warn!(
                                    "[RustSDKSchedule] Failed to find balanced closing brace for currentSemester."
                                );
                            }
                        }
                    }
                    None
                };

                // 优先使用平衡提取法
                if let Some(res) = parse_balanced_json(content) {
                    return Some(res);
                }

                // 回退到原来的正则提取逻辑
                if let Some(res) = extract_from_text(content) {
                    return Some(res);
                }
                warn!("[RustSDKSchedule] Target script block found but extraction failed.");
            }
        }

        if !script_found {
            warn!("[RustSDKSchedule] Target script block NOT found.");
        }

        // 2. 全局回退
        info!("[RustSDKSchedule] Falling back to global search...");
        if let Some(res) = extract_from_text(html) {
            info!("[RustSDKSchedule] Global fallback extraction success.");
            return Some(res);
        }

        error!(
            "[RustSDKSchedule] All parsing attempts failed. HTML len: {}",
            html.len()
        );
        // log first 500 chars for debugging
        if html.len() > 500 {
            info!("[RustSDKSchedule] HTML Head: {}", &html[..500]);
        } else {
            info!("[RustSDKSchedule] HTML: {}", html);
        }

        None
    }

    pub fn parse_semesters_list(html: &str) -> Option<String> {
        static RE: OnceLock<Regex> = OnceLock::new();
        // 增加 (?s) 支持跨行匹配，与 Master 分支 DOT_MATCHES_ALL 保持一致
        let re = RE.get_or_init(|| {
            Regex::new(r"(?s)var\s+semesters\s*=\s*JSON\.parse\(\s*'(.*?)'\s*\);").unwrap()
        });

        re.captures(html)
            .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
    }

    pub fn parse_exam_info(html: &str) -> Vec<Exam> {
        static RE: OnceLock<Regex> = OnceLock::new();
        let re = RE.get_or_init(|| Regex::new(r"studentExamInfoVms\s*=\s*(\[.*?]);").unwrap());

        let mut exams = Vec::new();

        if let Some(caps) = re.captures(html) {
            if let Some(raw_json_match) = caps.get(1) {
                let fixed_json = raw_json_match.as_str().replace("'", "\"");

                #[derive(Deserialize)]
                struct RawExam {
                    #[serde(rename = "course")]
                    course: RawExamCourse,
                    #[serde(rename = "examTime")]
                    exam_time: String,
                    #[serde(rename = "seatNo")]
                    seat_no: i32,
                    #[serde(rename = "room")]
                    room: String,
                    #[serde(rename = "requiredCampus")]
                    campus: RawExamCampus,
                    finished: bool,
                }
                #[derive(Deserialize)]
                struct RawExamCourse {
                    #[serde(rename = "nameZh")]
                    name_zh: String,
                }
                #[derive(Deserialize)]
                struct RawExamCampus {
                    #[serde(rename = "nameZh")]
                    name_zh: String,
                }

                if let Ok(raw_list) = serde_json::from_str::<Vec<RawExam>>(&fixed_json) {
                    for item in raw_list {
                        exams.push(Exam {
                            course: item.course.name_zh,
                            time: item.exam_time,
                            seat_num: item.seat_no.to_string(),
                            location: format!("{}-{}", item.campus.name_zh, item.room),
                            finished: item.finished,
                        });
                    }
                }
            }
        }
        exams
    }

    pub fn parse_cas_params(html: &str) -> Option<(String, String, String)> {
        static INPUT_RE: OnceLock<Regex> = OnceLock::new();
        let input_re = INPUT_RE.get_or_init(|| Regex::new(r"<input([^>]+)>").unwrap());

        static NAME_RE: OnceLock<Regex> = OnceLock::new();
        let name_re = NAME_RE.get_or_init(|| Regex::new(r#"name="([^"]+)""#).unwrap());

        static VALUE_RE: OnceLock<Regex> = OnceLock::new();
        let value_re = VALUE_RE.get_or_init(|| Regex::new(r#"value="([^"]*)""#).unwrap());

        static ACTION_RE: OnceLock<Regex> = OnceLock::new();
        // 修正 action 提取正则，处理 action 属性在不同位置或换行的情况
        let action_re =
            ACTION_RE.get_or_init(|| Regex::new(r#"<form[\s\S]*?action="([^"]+)""#).unwrap());

        let mut lt = String::new();
        let mut execution = String::new();

        for cap in input_re.captures_iter(html) {
            let attrs = &cap[1];
            if let Some(name_cap) = name_re.captures(attrs) {
                let name = &name_cap[1];
                if name == "lt" {
                    if let Some(val_cap) = value_re.captures(attrs) {
                        lt = val_cap[1].to_string();
                    }
                } else if name == "execution" {
                    if let Some(val_cap) = value_re.captures(attrs) {
                        execution = val_cap[1].to_string();
                    }
                }
            }
        }

        let mut action = action_re
            .captures(html)
            .map(|c| c[1].to_string())
            .unwrap_or_default();

        // Simple HTML entity decode for URL
        action = action.replace("&amp;", "&");

        // 增加容错：如果正则提取失败，打印一下 HTML 片段帮助排查（生产环境建议移除）
        if lt.is_empty() || execution.is_empty() {
            // 这里我们无法打印日志，因为 parser.rs 没有引入 log crate。
            // 考虑返回 None，由调用者记录。
            // 但如果 lt 为空，可能是因为正则匹配问题，我们尝试更宽松的正则？
            // 目前先保持原样，调用者已经打印了 "Failed to parse CAS login params"
        }

        if !lt.is_empty() && !execution.is_empty() {
            Some((lt, execution, action))
        } else {
            None
        }
    }
}
