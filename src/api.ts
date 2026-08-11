import { invoke } from "@tauri-apps/api/core";
import type {
  Entry,
  EntryInput,
  Tag,
  CategoryStat,
  TagStat,
  Settings,
  OtpEntry,
  OtpEntryInput,
  Template,
  TemplateInput,
  Category,
  CategoryInput,
} from "./types";

export const api = {
  hasMasterPassword: () => invoke<boolean>("has_master_password"),
  setMasterPassword: (password: string) =>
    invoke<void>("set_master_password", { password }),
  unlock: (password: string) => invoke<void>("unlock", { password }),
  lock: () => invoke<void>("lock"),
  heartbeat: () => invoke<void>("heartbeat"),

  createEntry: (input: EntryInput) => invoke<Entry>("create_entry", { input }),
  updateEntry: (id: string, input: EntryInput) =>
    invoke<Entry>("update_entry", { id, input }),
  getEntries: (includeDeleted = false) =>
    invoke<Entry[]>("get_entries", { includeDeleted }),
  getEntry: (id: string) => invoke<Entry>("get_entry", { id }),
  deleteEntry: (id: string) => invoke<void>("delete_entry", { id }),
  restoreEntry: (id: string) => invoke<void>("restore_entry", { id }),
  permanentlyDeleteEntry: (id: string) =>
    invoke<void>("permanently_delete_entry", { id }),
  emptyTrash: () => invoke<void>("empty_trash"),

  getTags: () => invoke<Tag[]>("get_tags"),
  createTag: (name: string, color?: string) =>
    invoke<Tag>("create_tag", { name, color }),
  deleteTag: (name: string) => invoke<void>("delete_tag", { name }),

  getCategoryStats: () => invoke<CategoryStat[]>("get_category_stats"),
  getTagStats: () => invoke<TagStat[]>("get_tag_stats"),

  getSettings: () => invoke<Settings>("get_settings"),
  getDatabasePath: () => invoke<string>("get_database_path"),
  updateSetting: (key: string, value: string) =>
    invoke<void>("update_setting", { key, value }),

  exportData: (dirPath: string) => invoke<string>("export_data", { dirPath }),
  importData: (filePath: string, password: string) =>
    invoke<{ imported: number; replaced: number }>("import_data", { filePath, password }),

  getOtpEntries: () => invoke<OtpEntry[]>("get_otp_entries"),
  createOtpEntry: (input: OtpEntryInput) =>
    invoke<OtpEntry>("create_otp_entry", { input }),
  updateOtpEntry: (id: string, input: OtpEntryInput) =>
    invoke<OtpEntry>("update_otp_entry", { id, input }),
  deleteOtpEntry: (id: string) => invoke<void>("delete_otp_entry", { id }),
  importOtpEntries: (entries: OtpEntryInput[]) =>
    invoke<number>("import_otp_entries", { entries }),
  captureQrCode: () => invoke<string>("capture_qr_code"),
  readFileText: (path: string) => invoke<string>("read_file_text", { path }),
  decodeQrImage: (path: string) => invoke<string>("decode_qr_image", { path }),

  listCategories: () => invoke<Category[]>("list_categories"),
  createCategory: (input: CategoryInput) =>
    invoke<Category>("create_category", { input }),
  updateCategory: (id: string, input: CategoryInput) =>
    invoke<Category>("update_category", { id, input }),
  deleteCategory: (id: string) => invoke<void>("delete_category", { id }),
  listTemplates: () => invoke<Template[]>("list_templates"),
  getTemplate: (id: string) => invoke<Template>("get_template", { id }),
  createTemplate: (input: TemplateInput) =>
    invoke<Template>("create_template", { input }),
  updateTemplate: (id: string, input: TemplateInput) =>
    invoke<Template>("update_template", { id, input }),
  deleteTemplate: (id: string) =>
    invoke<void>("delete_template", { id }),
};
