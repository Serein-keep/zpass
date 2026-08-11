// 与 Rust 后端对应的前端类型定义
export interface Field {
  name: string;
  value: string;
  secret?: boolean;
  field_type?: string;
}

export interface Tag {
  id: string;
  name: string;
  color?: string | null;
}

export interface Entry {
  id: string;
  category: string;
  title: string;
  icon?: string | null;
  note?: string | null;
  otp_id?: string | null;
  otp_mode?: string | null;
  fields: Field[];
  tags: Tag[];
  created_at: string;
  updated_at: string;
  deleted_at?: string | null;
}

export interface EntryInput {
  category: string;
  title: string;
  icon?: string | null;
  note?: string | null;
  otp_id?: string | null;
  otp_mode?: string | null;
  fields: Field[];
  tags: string[];
}

export interface CategoryStat {
  category: string;
  count: number;
}

export interface TagStat {
  id: string;
  name: string;
  color?: string | null;
  count: number;
}

export interface Settings {
  lock_timeout: number;
  theme: string;
  storage_path: string;
}

export interface OtpEntry {
  id: string;
  issuer: string;
  account: string;
  secret: string;
  interval: number;
  digits: number;
  algorithm: string;
  created_at: string;
}

export interface Category {
  id: string;
  parent_id?: string | null;
  name: string;
  icon?: string | null;
  color?: string | null;
  sort_order: number;
  created_at: string;
}

export interface CategoryInput {
  name: string;
  parent_id?: string | null;
  icon?: string | null;
  color?: string | null;
}

export interface TemplateField {
  name: string;
  field_type: string;
  secret: boolean;
}

export interface TemplateFieldInput {
  name: string;
  field_type: string;
  secret: boolean;
}

export interface TemplateInput {
  category_id: string;
  name: string;
  icon?: string | null;
  note?: string | null;
  fields: TemplateFieldInput[];
}

export interface Template {
  id: string;
  category_id: string;
  category_name: string;
  name: string;
  icon?: string | null;
  note?: string | null;
  fields: TemplateField[];
  created_at: string;
  updated_at: string;
  is_builtin?: boolean;
}

export const TEMPLATE_ICON_OPTIONS = [
  { value: "globe", label: "地球", icon: "globe" },
  { value: "server", label: "服务器" },
  { value: "mail", label: "邮箱" },
  { value: "phone-portrait", label: "手机" },
  { value: "wifi", label: "Wi-Fi" },
  { value: "key", label: "密钥" },
  { value: "lock-closed", label: "锁" },
  { value: "card", label: "卡片" },
  { value: "folder-open", label: "文件夹" },
  { value: "document-text", label: "文档" },
  { value: "shield-checkmark", label: "盾牌" },
  { value: "cloud", label: "云" },
  { value: "home", label: "首页" },
  { value: "people", label: "人员" },
  { value: "time", label: "时间" },
  { value: "albums", label: "相册" },
  { value: "book", label: "书籍" },
  { value: "person", label: "个人" },
  { value: "git-branch", label: "分支" },
  { value: "hardware-chip", label: "芯片" },
];

export const FIELD_TYPE_OPTIONS = [
  { value: "text", label: "文本" },
  { value: "url", label: "URL" },
  { value: "password", label: "密码" },
  { value: "email", label: "邮箱" },
  { value: "phone", label: "手机号" },
  { value: "number", label: "数字" },
  { value: "month", label: "月份" },
  { value: "date", label: "日期" },
  { value: "multiline", label: "多行文本" },
];

export const DEFAULT_TEMPLATE_FIELDS: TemplateFieldInput[] = [
  { name: "用户名", field_type: "text", secret: false },
  { name: "密码", field_type: "password", secret: true },
];

export interface OtpEntryInput {
  issuer: string;
  account: string;
  secret: string;
  interval?: number;
  digits?: number;
  algorithm?: string;
}
