<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useRouter } from "vue-router";
import { useAppStore } from "../stores/app";
import { NButton, NIcon, useMessage } from "naive-ui";
import { uiIcon } from "../utils/templateIcons";
import Sidebar from "../components/Sidebar.vue";
import EntryList from "../components/EntryList.vue";
import EntryDetail from "../components/EntryDetail.vue";
import EntryOverview from "../components/EntryOverview.vue";
import OtpPanel from "../components/OtpPanel.vue";
import PasswordGenerator from "../components/PasswordGenerator.vue";
import TemplateLibraryView from "../components/TemplateLibraryView.vue";
import TemplatePickerDialog from "../components/TemplatePickerDialog.vue";
import TemplateEditorDialog from "../components/TemplateEditorDialog.vue";
import TemplateCategoryDialog from "../components/TemplateCategoryDialog.vue";
import TagDialog from "../components/TagDialog.vue";
import type { Template, Category } from "../types";

const router = useRouter();
const store = useAppStore();
const message = useMessage();

const LockIcon = uiIcon("lock-closed");
const SettingsIcon = uiIcon("settings");
const AddIcon = uiIcon("add");
const KeyIcon = uiIcon("key");

const selectedId = ref<string | null>(null);
const view = ref<"all" | "trash" | "otp" | "templates">("all");
const filterCategory = ref<string | null>(null);
const filterTag = ref<string | null>(null);
const search = ref("");
const showGenerator = ref(false);

const showPicker = ref(false);
const pendingTemplateId = ref<string | null>(null);
const showTemplateEditor = ref(false);
const editingTemplate = ref<Template | null>(null);

const showCategoryDialog = ref(false);
const editingCategory = ref<Category | null>(null);
const categoryParent = ref<Category | null>(null);

const showTagDialog = ref(false);
const editingTag = ref<{ id: string; name: string; color?: string | null } | null>(null);

const selectedEntry = computed(() =>
  store.entries.find((e) => e.id === selectedId.value) || null
);

const filteredEntries = computed(() => {
  let list = store.entries;
  if (view.value === "trash") {
    list = list.filter((e) => e.deleted_at);
  } else {
    list = list.filter((e) => !e.deleted_at);
  }
  if (filterCategory.value) {
    list = list.filter((e) => e.category === filterCategory.value);
  }
  if (filterTag.value) {
    list = list.filter((e) => e.tags.some((t) => t.name === filterTag.value));
  }
  if (search.value.trim()) {
    const q = search.value.toLowerCase();
    list = list.filter(
      (e) =>
        e.title.toLowerCase().includes(q) ||
        e.fields.some((f) => f.value.toLowerCase().includes(q)) ||
        (e.note || "").toLowerCase().includes(q) ||
        e.tags.some((t) => t.name.toLowerCase().includes(q))
    );
  }
  return list;
});

function selectEntry(id: string) {
  selectedId.value = id;
}

function onCategory(cat: string | null) {
  filterCategory.value = cat;
  filterTag.value = null;
  view.value = "all";
  selectedId.value = null;
}

function onTag(tag: string | null) {
  filterTag.value = tag;
  filterCategory.value = null;
  view.value = "all";
  selectedId.value = null;
}

function onView(v: "all" | "trash" | "otp" | "templates") {
  view.value = v;
  filterCategory.value = null;
  filterTag.value = null;
  selectedId.value = null;
}

function newEntry() {
  if (store.templates.length === 0) {
    message.info("请先到模板库创建模板");
    view.value = "templates";
    return;
  }
  showPicker.value = true;
}

function onTemplatePicked(id: string) {
  pendingTemplateId.value = id;
  selectedId.value = "__new__";
}

function onTemplateSaved() {
  editingTemplate.value = null;
  showTemplateEditor.value = false;
}

function editTemplate(t: Template) {
  editingTemplate.value = t;
  showTemplateEditor.value = true;
}

function createTemplate() {
  editingTemplate.value = null;
  showTemplateEditor.value = true;
}

function editCategory(cat: Category) {
  editingCategory.value = cat;
  showCategoryDialog.value = true;
}

function createCategory(parent: Category | null) {
  editingCategory.value = null;
  categoryParent.value = parent;
  showCategoryDialog.value = true;
}

function onCategorySaved() {
  showCategoryDialog.value = false;
  editingCategory.value = null;
  categoryParent.value = null;
}

function createTag() {
  editingTag.value = null;
  showTagDialog.value = true;
}

function editTag(tag: { id: string; name: string; color?: string | null }) {
  editingTag.value = tag;
  showTagDialog.value = true;
}

function onTagSaved() {
  showTagDialog.value = false;
  editingTag.value = null;
}

function goSettings() {
  router.push("/settings");
}

async function lockNow() {
  await store.lock();
  router.replace("/lock");
}

onMounted(() => {
  if (store.locked) router.replace("/lock");
});
</script>

<template>
  <div class="app-layout">
    <div class="sidebar-panel">
      <Sidebar
        :filter-category="filterCategory"
        :filter-tag="filterTag"
        :view="view"
        @category="onCategory"
        @tag="onTag"
        @view="onView"
        @managecategory="editCategory"
        @createcategory="createCategory"
        @createtag="createTag"
        @managetag="editTag"
      />
    </div>
    <div class="content-area">
      <div class="topbar">
        <input
          class="search"
          v-model="search"
          placeholder="搜索标题、账号、备注、标签…"
        />
        <div class="spacer" />
        <n-button quaternary @click="newEntry">
          <template #icon><n-icon><AddIcon /></n-icon></template>
          新建
        </n-button>
        <n-button quaternary @click="lockNow">
          <template #icon><n-icon><LockIcon /></n-icon></template>
          锁定
        </n-button>
        <n-button quaternary @click="showGenerator = true">
          <template #icon><n-icon><KeyIcon /></n-icon></template>
          生成
        </n-button>
        <n-button quaternary @click="goSettings">
          <template #icon><n-icon><SettingsIcon /></n-icon></template>
          设置
        </n-button>
      </div>
      <PasswordGenerator v-model:show="showGenerator" />
      <TemplatePickerDialog
        v-model:show="showPicker"
        @select="onTemplatePicked"
        @golibrary="view = 'templates'"
      />
      <div class="panels">
        <template v-if="view === 'otp'">
          <OtpPanel />
        </template>
        <template v-else-if="view === 'templates'">
          <TemplateLibraryView
            @edit="editTemplate"
            @create="createTemplate"
          />
          <TemplateEditorDialog
            v-model:show="showTemplateEditor"
            :template="editingTemplate"
            @saved="onTemplateSaved"
          />
        </template>
        <template v-else>
          <EntryList
            :entries="filteredEntries"
            :selected-id="selectedId"
            @select="selectEntry"
          />
          <EntryDetail
            v-if="selectedEntry || selectedId === '__new__'"
            :entry="selectedEntry"
            :is-new="selectedId === '__new__'"
            :initial-template-id="pendingTemplateId"
            @close="
              selectedId = null;
              pendingTemplateId = null;
            "
            @saved="
              selectedId = $event;
              pendingTemplateId = null;
            "
          />
          <EntryOverview
            v-else
            @category="onCategory"
            @tag="onTag"
            @view="onView"
          />
        </template>
      </div>
      <TemplateCategoryDialog
        v-model:show="showCategoryDialog"
        :category="editingCategory"
        :parent="categoryParent"
        @update:show="onCategorySaved"
      />
      <TagDialog
        v-model:show="showTagDialog"
        :tag="editingTag"
        @update:show="onTagSaved"
      />
    </div>
  </div>
</template>

<style scoped>
/* ── 根布局：左 sidebar + 右内容区 ── */
.app-layout {
  display: flex;
  height: 100vh;
  overflow: hidden;
}

/* ── 左侧 sidebar ── */
.sidebar-panel {
  width: 260px;
  flex-shrink: 0;
  background: var(--sidebar);
  border-right: 1px solid var(--border);
}

/* ── 右侧内容区：topbar + panels 纵向排列 ── */
.content-area {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
}

/* ── 顶部工具栏：永远固定，不参与滚动 ── */
.topbar {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  background: var(--panel);
}
.search {
  flex: 1;
  max-width: 400px;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg);
  color: var(--text);
  outline: none;
  font-size: 14px;
}
.search:focus {
  border-color: var(--primary);
}
.spacer {
  flex: 1;
}

/* ── 中间 + 右侧面板：横向排列，各自独立滚动 ── */
.panels {
  flex: 1;
  display: flex;
  min-height: 0;
}
</style>
