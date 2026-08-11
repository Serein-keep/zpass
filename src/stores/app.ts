import { defineStore } from "pinia";
import { api } from "../api";
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
  Category,
  CategoryInput,
  TemplateInput,
} from "../types";

export const useAppStore = defineStore("app", {
  state: () => ({
    locked: true,
    hasMaster: false,
    entries: [] as Entry[],
    tags: [] as Tag[],
    categoryStats: [] as CategoryStat[],
    tagStats: [] as TagStat[],
    settings: { lock_timeout: 30, theme: "light", storage_path: "" } as Settings,
    otpEntries: [] as OtpEntry[],
    templates: [] as Template[],
    categories: [] as Category[],
  }),
  getters: {
    activeEntries: (s) => s.entries.filter((e) => !e.deleted_at),
    trashedEntries: (s) => s.entries.filter((e) => e.deleted_at),
    templateById: (s) => (id: string | null | undefined) =>
      s.templates.find((t) => t.id === id) || null,
    categoryById: (s) => (id: string | null | undefined) =>
      s.categories.find((c) => c.id === id) || null,
    rootCategories: (s) => s.categories.filter((c) => !c.parent_id),
    childCategories: (s) => (parentId: string) =>
      s.categories.filter((c) => c.parent_id === parentId),
  },
  actions: {
    async init() {
      this.hasMaster = await api.hasMasterPassword();
      if (this.hasMaster) {
        // 已设置主密码，等待解锁；未解锁不加载数据
      }
    },
    async refreshAll() {
      const [entries, tags, cs, ts, settings, otpEntries, templates, categories] =
        await Promise.all([
          api.getEntries(true),
          api.getTags(),
          api.getCategoryStats(),
          api.getTagStats(),
          api.getSettings(),
          api.getOtpEntries(),
          api.listTemplates(),
          api.listCategories(),
        ]);
      this.entries = entries;
      this.tags = tags;
      this.categoryStats = cs;
      this.tagStats = ts;
      this.settings = settings;
      this.otpEntries = otpEntries;
      this.templates = templates;
      this.categories = categories;
      this.applyTheme();
    },
    async unlock(password: string) {
      await api.unlock(password);
      this.locked = false;
      this.refreshAll();
    },
    async setMasterPassword(password: string) {
      await api.setMasterPassword(password);
      this.hasMaster = true;
      this.locked = false;
      await this.refreshAll();
    },
    async lock() {
      await api.lock();
      this.locked = true;
      this.entries = [];
    },
    applyTheme() {
      const root = document.documentElement;
      if (this.settings.theme === "dark") {
        root.classList.add("dark");
      } else {
        root.classList.remove("dark");
      }
    },
    async createEntry(input: EntryInput) {
      const e = await api.createEntry(input);
      await this.refreshAll();
      return e;
    },
    async updateEntry(id: string, input: EntryInput) {
      const e = await api.updateEntry(id, input);
      await this.refreshAll();
      return e;
    },
    async deleteEntry(id: string) {
      await api.deleteEntry(id);
      await this.refreshAll();
    },
    async restoreEntry(id: string) {
      await api.restoreEntry(id);
      await this.refreshAll();
    },
    async permanentlyDelete(id: string) {
      await api.permanentlyDeleteEntry(id);
      await this.refreshAll();
    },
    async emptyTrash() {
      await api.emptyTrash();
      await this.refreshAll();
    },
    async createTag(name: string, color?: string) {
      const t = await api.createTag(name, color);
      await this.refreshAll();
      return t;
    },
    async deleteTag(name: string) {
      await api.deleteTag(name);
      await this.refreshAll();
    },
    async updateSetting(key: string, value: string) {
      await api.updateSetting(key, value);
      this.settings = await api.getSettings();
      this.applyTheme();
    },
    async createOtpEntry(input: OtpEntryInput) {
      const e = await api.createOtpEntry(input);
      await this.refreshAll();
      return e;
    },
    async updateOtpEntry(id: string, input: OtpEntryInput) {
      const e = await api.updateOtpEntry(id, input);
      await this.refreshAll();
      return e;
    },
    async deleteOtpEntry(id: string) {
      await api.deleteOtpEntry(id);
      await this.refreshAll();
    },
    async importOtpEntries(entries: OtpEntryInput[]) {
      const count = await api.importOtpEntries(entries);
      await this.refreshAll();
      return count;
    },
    async createTemplate(input: TemplateInput) {
      const t = await api.createTemplate(input);
      await this.refreshAll();
      return t;
    },
    async updateTemplate(id: string, input: TemplateInput) {
      const t = await api.updateTemplate(id, input);
      await this.refreshAll();
      return t;
    },
    async deleteTemplate(id: string) {
      await api.deleteTemplate(id);
      await this.refreshAll();
    },
    async createCategory(input: CategoryInput) {
      const c = await api.createCategory(input);
      await this.refreshAll();
      return c;
    },
    async updateCategory(id: string, input: CategoryInput) {
      const c = await api.updateCategory(id, input);
      await this.refreshAll();
      return c;
    },
    async deleteCategory(id: string) {
      await api.deleteCategory(id);
      await this.refreshAll();
    },
  },
});
