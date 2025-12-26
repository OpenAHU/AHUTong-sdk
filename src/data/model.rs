use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    #[serde(rename = "name")]
    pub username: String,

    #[serde(rename = "xh")]
    pub id_number: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Course {
    pub name: String,
    pub teacher: String,
    pub location: String,

    #[serde(rename = "weekIndexes")]
    pub week_indexes: Vec<i32>,

    #[serde(rename = "startWeek")]
    pub start_week: String,

    #[serde(rename = "endWeek")]
    pub end_week: String,

    #[serde(rename = "startTime")]
    pub start_time: String,

    pub length: String,
    pub weekday: String,

    #[serde(rename = "courseId")]
    pub course_id: String,
}

// Grade 和 Exam 暂时不需要改，因为我们还没有迁移这两个功能
// 但为了将来准备，建议也按照 Android 端的字段名进行对齐
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Grade {
    #[serde(rename = "course")]
    pub course_name: String,

    pub credit: String,
    pub grade: String,

    #[serde(rename = "gradePoint")]
    pub grade_point: String,

    #[serde(rename = "courseType")]
    pub course_nature: String,

    #[serde(rename = "courseNum")]
    pub course_num: String,

    // term 在 Android Grade.java 中可能是在 TermGradeListBean 里，结构比较复杂
    // 这里暂时保留，等迁移 Grade 功能时再细化
    pub term: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Exam {
    pub course: String,
    pub time: String,

    #[serde(rename = "seatNum")]
    pub seat_num: String,

    pub location: String,

    // Android Exam.java 似乎没有 finished 字段，或者是不序列化的
    // 如果不需要传给 Android，可以加 #[serde(skip)] 或者保留（Gson 会忽略多余字段）
    pub finished: bool,
}