use serde::{Deserialize, Serialize};

/// 字段（条目的默认字段或自定义字段）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub value: String,
    /// 是否敏感（密码类，显示时默认掩码）
    #[serde(default)]
    pub secret: bool,
    /// 字段类型快照（text/url/password/email/phone/number/month/date/multiline）
    #[serde(default = "default_field_type")]
    pub field_type: String,
}

pub const FIELD_TYPES: [&str; 9] = [
    "text", "url", "password", "email", "phone", "number", "month", "date", "multiline",
];

pub fn validate_field_type(t: &str) -> bool {
    FIELD_TYPES.contains(&t)
}

fn default_field_type() -> String {
    "text".into()
}

/// 创建/更新条目的请求体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryInput {
    pub category: String,
    pub title: String,
    pub icon: Option<String>,
    pub note: Option<String>,
    /// 关联的 OTP 条目 ID（独立属性，不存入字段列表）
    #[serde(default)]
    pub otp_id: Option<String>,
    /// OTP 验证模式：password_concat / secondary
    #[serde(default)]
    pub otp_mode: Option<String>,
    pub fields: Vec<Field>,
    pub tags: Vec<String>, // 标签名称列表（自动创建不存在的）
}

/// 返回给前端的条目（字段已解密）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryOut {
    pub id: String,
    pub category: String,
    pub title: String,
    pub icon: Option<String>,
    pub note: Option<String>,
    pub otp_id: Option<String>,
    pub otp_mode: Option<String>,
    pub fields: Vec<Field>,
    pub tags: Vec<TagOut>,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagOut {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}

// ---------------- 类别 / 模板库 ----------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    pub sort_order: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryInput {
    pub name: String,
    pub parent_id: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateField {
    pub name: String,
    pub field_type: String,
    pub secret: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateFieldInput {
    pub name: String,
    pub field_type: String,
    pub secret: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInput {
    pub category_id: String,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    pub fields: Vec<TemplateFieldInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub category_id: String,
    pub category_name: String,
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    pub fields: Vec<TemplateField>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub is_builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryStat {
    pub category: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagStat {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub lock_timeout: u64,
    pub theme: String,
    pub storage_path: String,
}

/// OTP 条目输入
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtpEntryInput {
    pub issuer: String,
    pub account: String,
    pub secret: String,
    #[serde(default = "default_interval")]
    pub interval: u32,
    #[serde(default = "default_digits")]
    pub digits: u32,
    #[serde(default = "default_algorithm")]
    pub algorithm: String,
}

fn default_interval() -> u32 { 30 }
fn default_digits() -> u32 { 6 }
fn default_algorithm() -> String { "SHA1".into() }

/// 返回给前端的 OTP 条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtpEntryOut {
    pub id: String,
    pub issuer: String,
    pub account: String,
    pub secret: String,
    pub interval: u32,
    pub digits: u32,
    pub algorithm: String,
    pub created_at: String,
}
