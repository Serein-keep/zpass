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
      this.otpEntries = [];
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
      this.entries.unshift(e);
      this.recomputeStats();
      return e;
    },
    async updateEntry(id: string, input: EntryInput) {
      const e = await api.updateEntry(id, input);
      const i = this.entries.findIndex((x) => x.id === id);
      if (i >= 0) this.entries[i] = e;
      this.recomputeStats();
      return e;
    },
    async deleteEntry(id: string) {
      await api.deleteEntry(id);
      const e = this.entries.find((x) => x.id === id);
      if (e) e.deleted_at = String(Math.floor(Date.now() / 1000));
      this.recomputeStats();
    },
    async restoreEntry(id: string) {
      await api.restoreEntry(id);
      const e = this.entries.find((x) => x.id === id);
      if (e) e.deleted_at = null;
      this.recomputeStats();
    },
    async permanentlyDelete(id: string) {
      await api.permanentlyDeleteEntry(id);
      this.entries = this.entries.filter((x) => x.id !== id);
      this.recomputeStats();
    },
    async emptyTrash() {
      await api.emptyTrash();
      this.entries = this.entries.filter((x) => !x.deleted_at);
      this.recomputeStats();
    },
    async createTag(name: string, color?: string) {
      const t = await api.createTag(name, color);
      this.tags.push(t);
      this.tags.sort((a, b) => a.name.localeCompare(b.name, "zh"));
      this.recomputeStats();
      return t;
    },
    async deleteTag(name: string) {
      await api.deleteTag(name);
      const tag = this.tags.find((t) => t.name === name);
      this.tags = this.tags.filter((t) => t.name !== name);
      if (tag) {
        for (const e of this.entries) {
          e.tags = e.tags.filter((t) => t.id !== tag.id);
        }
      }
      this.recomputeStats();
    },
    async updateSetting(key: string, value: string) {
      await api.updateSetting(key, value);
      this.settings = await api.getSettings();
      this.applyTheme();
    },
    async createOtpEntry(input: OtpEntryInput) {
      const e = await api.createOtpEntry(input);
      this.otpEntries.unshift(e);
      return e;
    },
    async updateOtpEntry(id: string, input: OtpEntryInput) {
      const e = await api.updateOtpEntry(id, input);
      const i = this.otpEntries.findIndex((x) => x.id === id);
      if (i >= 0) this.otpEntries[i] = e;
      return e;
    },
    async deleteOtpEntry(id: string) {
      await api.deleteOtpEntry(id);
      this.otpEntries = this.otpEntries.filter((x) => x.id !== id);
    },
    async importOtpEntries(entries: OtpEntryInput[]) {
      const count = await api.importOtpEntries(entries);
      this.otpEntries = await api.getOtpEntries();
      return count;
    },
    async createTemplate(input: TemplateInput) {
      const t = await api.createTemplate(input);
      this.templates.push(t);
      return t;
    },
    async updateTemplate(id: string, input: TemplateInput) {
      const t = await api.updateTemplate(id, input);
      const i = this.templates.findIndex((x) => x.id === id);
      if (i >= 0) this.templates[i] = t;
      return t;
    },
    async deleteTemplate(id: string) {
      await api.deleteTemplate(id);
      this.templates = this.templates.filter((x) => x.id !== id);
    },
    async createCategory(input: CategoryInput) {
      const c = await api.createCategory(input);
      this.categories.push(c);
      return c;
    },
    async updateCategory(id: string, input: CategoryInput) {
      const c = await api.updateCategory(id, input);
      const i = this.categories.findIndex((x) => x.id === id);
      if (i >= 0) this.categories[i] = c;
      return c;
    },
    async deleteCategory(id: string) {
      await api.deleteCategory(id);
      this.categories = this.categories.filter((x) => x.id !== id);
    },
    recomputeStats() {
      this.categoryStats = Object.entries(
        this.entries.reduce<Record<string, number>>((acc, e) => {
          if (!e.deleted_at) acc[e.category] = (acc[e.category] || 0) + 1;
          return acc;
        }, {}),
      ).map(([category, count]) => ({ category, count }));
      this.tagStats = this.tags.map((t) => ({
        id: t.id,
        name: t.name,
        color: t.color ?? null,
        count: this.entries.filter(
          (e) => !e.deleted_at && e.tags.some((et) => et.id === t.id),
        ).length,
      }));
    },
  },
});
