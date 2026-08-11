<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from "vue";
import { useRouter } from "vue-router";
import { useAppStore } from "../stores/app";
import { open } from "@tauri-apps/plugin-dialog";
import { openPath } from "@tauri-apps/plugin-opener";
import { api } from "../api";
import {
  NButton,
  NSelect,
  NInputNumber,
  NInput,
  NModal,
  useMessage,
  useDialog,
} from "naive-ui";

const router = useRouter();
const store = useAppStore();
const message = useMessage();
const dialog = useDialog();

const theme = ref("light");
const lockTimeout = ref(30);
const storagePath = ref("");
const dbPath = ref("");
const trashCount = ref(0);

const showImportModal = ref(false);
const importFile = ref("");
const importPassword = ref("");
const importLoading = ref(false);

const themeOptions = [
  { label: "浅色", value: "light" },
  { label: "深色", value: "dark" },
];

const sections = [
  { id: "appearance", label: "外观" },
  { id: "security", label: "安全" },
  { id: "storage", label: "数据存储" },
  { id: "trash", label: "回收站" },
  { id: "about", label: "关于" },
];
const activeSection = ref("appearance");
let observer: IntersectionObserver | null = null;

async function load() {
  await store.refreshAll();
  theme.value = store.settings.theme;
  lockTimeout.value = store.settings.lock_timeout;
  storagePath.value = store.settings.storage_path;
  trashCount.value = store.trashedEntries.length;
  dbPath.value = await api.getDatabasePath();
  await nextTick();
  setupObserver();
}

function setupObserver() {
  observer?.disconnect();
  observer = new IntersectionObserver(
    (entries) => {
      for (const e of entries) {
        if (e.isIntersecting) {
          activeSection.value = e.target.id;
        }
      }
    },
    { root: document.querySelector(".settings-scroll"), rootMargin: "-80px 0px -60% 0px", threshold: 0 }
  );
  for (const s of sections) {
    const el = document.getElementById(s.id);
    if (el) observer.observe(el);
  }
}

function scrollTo(id: string) {
  document.getElementById(id)?.scrollIntoView({ behavior: "smooth" });
}

async function saveTheme() {
  await store.updateSetting("theme", theme.value);
  message.success("主题已更新");
}
async function saveLock() {
  await store.updateSetting("lock_timeout", String(lockTimeout.value));
  message.success("锁屏时间已更新");
}
async function pickPath() {
  const selected = await open({ directory: true });
  if (selected && typeof selected === "string") {
    storagePath.value = selected;
    await store.updateSetting("storage_path", selected);
    dbPath.value = await api.getDatabasePath();
    message.info("存储路径已保存（重启应用后生效）");
  }
}

async function openDir() {
  const dir = dbPath.value.replace(/[/\\][^/\\]+$/, "");
  await openPath(dir);
}

async function doExport() {
  const dir = await open({ directory: true, title: "选择导出目录" });
  if (!dir || typeof dir !== "string") return;
  try {
    const path = await api.exportData(dir);
    message.success("导出成功: " + path);
  } catch (e: any) {
    message.error(e?.toString() || "导出失败");
  }
}

async function doImport() {
  const file = await open({
    title: "选择导入文件",
    filters: [{ name: "JSON", extensions: ["json"] }],
  });
  if (!file || typeof file !== "string") return;
  importFile.value = file;
  importPassword.value = "";
  importLoading.value = false;
  showImportModal.value = true;
}

async function confirmImport() {
  if (!importPassword.value) {
    message.warning("请输入主密码");
    return;
  }
  importLoading.value = true;
  try {
    const result = await api.importData(importFile.value, importPassword.value);
    let msg = `导入成功，共 ${result.imported} 条记录`;
    if (result.replaced > 0) {
      msg += `，其中 ${result.replaced} 条替换了已有数据`;
    }
    message.success(msg);
    showImportModal.value = false;
    await store.refreshAll();
    trashCount.value = store.trashedEntries.length;
  } catch (e: any) {
    message.error(e?.toString() || "导入失败");
  } finally {
    importLoading.value = false;
  }
}

function emptyTrash() {
  if (trashCount.value === 0) return;
  dialog.warning({
    title: "清空回收站",
    content: `确定彻底删除 ${trashCount.value} 个条目？此操作不可恢复。`,
    positiveText: "清空",
    negativeText: "取消",
    onPositiveClick: async () => {
      await store.emptyTrash();
      trashCount.value = 0;
      message.success("回收站已清空");
    },
  });
}

function back() {
  router.push("/main");
}

onMounted(load);
onUnmounted(() => observer?.disconnect());
</script>

<template>
  <div class="settings">
    <div class="topbar">
      <span class="title">设置</span>
      <div class="spacer" />
      <n-button size="small" quaternary @click="back">返回</n-button>
    </div>

    <div class="settings-body">
      <nav class="settings-nav">
        <div
          v-for="s in sections"
          :key="s.id"
          class="nav-item"
          :class="{ active: activeSection === s.id }"
          @click="scrollTo(s.id)"
        >
          {{ s.label }}
        </div>
      </nav>

      <div class="settings-scroll">
        <section id="appearance" class="section">
          <h3 class="section-title">外观</h3>
          <div class="section-content">
            <div class="row">
              <span class="row-label">主题样式</span>
              <n-select
                :options="themeOptions"
                v-model:value="theme"
                style="width: 160px"
              />
              <n-button @click="saveTheme">应用</n-button>
            </div>
          </div>
        </section>

        <div class="section-divider" />

        <section id="security" class="section">
          <h3 class="section-title">安全</h3>
          <div class="section-content">
            <div class="row">
              <span class="row-label">无操作自动锁屏（秒）</span>
              <n-input-number
                v-model:value="lockTimeout"
                :min="5"
                :max="3600"
                style="width: 140px"
              />
              <n-button @click="saveLock">保存</n-button>
            </div>
            <div class="tip">
              默认 30 秒。应用在无鼠标/键盘操作达到该时长后自动锁定，需主密码解锁。
            </div>
          </div>
        </section>

        <div class="section-divider" />

        <section id="storage" class="section">
          <h3 class="section-title">数据存储</h3>
          <div class="section-content">
            <div class="row">
              <span class="row-label">数据库路径</span>
              <n-button @click="pickPath">选择目录</n-button>
              <n-button @click="openDir" :disabled="!dbPath">
                打开目录
              </n-button>
            </div>
            <div class="path">{{ dbPath }}</div>
            <div class="tip">修改路径后需重启应用生效。</div>

            <div class="row" style="margin-top: 20px">
              <n-button @click="doExport">导出数据</n-button>
              <n-button @click="doImport">导入数据</n-button>
            </div>
            <div class="tip">导出为加密 JSON 文件，导入时需提供对应主密码。</div>
          </div>
        </section>

        <div class="section-divider" />

        <section id="trash" class="section">
          <h3 class="section-title">回收站</h3>
          <div class="section-content">
            <div class="row">
              <span class="row-label">回收站中条目数：{{ trashCount }}</span>
              <n-button
                type="error"
                :disabled="trashCount === 0"
                @click="emptyTrash"
              >
                清空回收站
              </n-button>
            </div>
          </div>
        </section>

        <div class="section-divider" />

        <section id="about" class="section">
          <h3 class="section-title">关于</h3>
          <div class="section-content">
            <div class="tip" style="margin-top: 0">
              ZPass — 基于 SQLite + AES-256-GCM 的本地密码管理器。
            </div>
          </div>
        </section>

        <div style="height: 40px" />
      </div>
    </div>

    <n-modal
      v-model:show="showImportModal"
      preset="dialog"
      title="导入数据"
      positive-text="导入"
      negative-text="取消"
      :loading="importLoading"
      @positive-click="confirmImport"
    >
      <n-input
        v-model:value="importPassword"
        type="password"
        show-password-on="click"
        placeholder="请输入导入数据的主密码"
        @keyup.enter="confirmImport"
        style="margin-top: 8px"
      />
    </n-modal>
  </div>
</template>

<style scoped>
.settings {
  height: 100%;
  display: flex;
  flex-direction: column;
  background: var(--bg);
}
.topbar {
  display: flex;
  align-items: center;
  padding: 12px 20px;
  border-bottom: 1px solid var(--border);
  background: var(--panel);
}
.title {
  font-weight: 600;
  font-size: 15px;
}
.spacer {
  flex: 1;
}
.settings-body {
  flex: 1;
  display: flex;
  min-height: 0;
}
.settings-nav {
  width: 180px;
  flex-shrink: 0;
  padding: 20px 12px 20px 20px;
  border-right: 1px solid var(--border);
  background: var(--sidebar);
}
.nav-item {
  padding: 8px 12px;
  border-radius: 8px;
  font-size: 14px;
  color: var(--text);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
  margin-bottom: 2px;
}
.nav-item:hover {
  background: var(--panel);
}
.nav-item.active {
  background: var(--primary);
  color: #fff;
}
.settings-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 28px 40px;
}
.section {
  scroll-margin-top: 12px;
}
.section-title {
  font-size: 16px;
  font-weight: 600;
  color: var(--text);
  margin: 0 0 16px 0;
}
.section-content {
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.section-divider {
  height: 1px;
  background: var(--border);
  margin: 28px 0;
}
.row {
  display: flex;
  align-items: center;
  gap: 12px;
}
.row-label {
  min-width: 160px;
  font-size: 14px;
  color: var(--text);
}
.tip {
  font-size: 12px;
  color: var(--text-sub);
  margin-top: 2px;
}
.path {
  font-family: monospace;
  font-size: 13px;
  color: var(--text-sub);
  word-break: break-all;
}
</style>
