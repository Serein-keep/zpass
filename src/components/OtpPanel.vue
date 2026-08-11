<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { useAppStore } from "../stores/app";
import { api } from "../api";
import type { OtpEntry, OtpEntryInput } from "../types";
import {
  NButton,
  NInput,
  NSelect,
  NIcon,
  NModal,
  useMessage,
  useDialog,
} from "naive-ui";
import { uiIcon } from "../utils/templateIcons";

const store = useAppStore();
const message = useMessage();
const dialog = useDialog();

const AddIcon = uiIcon("add");
const TrashIcon = uiIcon("trash");
const CameraIcon = uiIcon("camera");
const FolderIcon = uiIcon("folder-open");
const ImageIcon = uiIcon("image");
const CreateIcon = uiIcon("create");

const showAddModal = ref(false);
const showImportModal = ref(false);
const showEditModal = ref(false);
const editingId = ref("");
const importFileContent = ref("");

const newEntry = ref<OtpEntryInput>({
  issuer: "",
  account: "",
  secret: "",
  interval: 30,
  digits: 6,
  algorithm: "SHA1",
});

const algorithmOptions = [
  { label: "SHA1", value: "SHA1" },
  { label: "SHA256", value: "SHA256" },
  { label: "SHA512", value: "SHA512" },
];

const otpEntries = computed(() => store.otpEntries);

const otpCodes = ref<Record<string, string>>({});
const otpCountdowns = ref<Record<string, number>>({});

async function generateTotp(entry: OtpEntry): Promise<string> {
  const now = Math.floor(Date.now() / 1000);
  const counter = Math.floor(now / entry.interval);

  const counterBytes = new Uint8Array(8);
  let temp = counter;
  for (let i = 7; i >= 0; i--) {
    counterBytes[i] = temp & 0xff;
    temp = Math.floor(temp / 256);
  }

  const secretBytes = base32Decode(entry.secret);

  const algorithm = entry.algorithm === "SHA256" ? "SHA-256" :
                    entry.algorithm === "SHA512" ? "SHA-512" : "SHA-1";

  const key = await crypto.subtle.importKey(
    "raw",
    secretBytes,
    { name: "HMAC", hash: algorithm },
    false,
    ["sign"]
  );
  const signature = await crypto.subtle.sign("HMAC", key, counterBytes);
  const hmac = new Uint8Array(signature);
  const offset = hmac[hmac.length - 1] & 0x0f;
  const code = (
    ((hmac[offset] & 0x7f) << 24) |
    ((hmac[offset + 1] & 0xff) << 16) |
    ((hmac[offset + 2] & 0xff) << 8) |
    (hmac[offset + 3] & 0xff)
  ) % Math.pow(10, entry.digits);
  return code.toString().padStart(entry.digits, "0");
}

function base32Decode(input: string): Uint8Array {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
  const cleaned = input.replace(/[\s=-]/g, "").toUpperCase();
  let bits = "";
  for (const char of cleaned) {
    const val = alphabet.indexOf(char);
    if (val === -1) continue;
    bits += val.toString(2).padStart(5, "0");
  }
  const bytes = new Uint8Array(Math.floor(bits.length / 8));
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(bits.substr(i * 8, 8), 2);
  }
  return bytes;
}

async function updateOtpCodes() {
  const now = Math.floor(Date.now() / 1000);
  for (const entry of otpEntries.value) {
    try {
      otpCodes.value[entry.id] = await generateTotp(entry);
      otpCountdowns.value[entry.id] = entry.interval - (now % entry.interval);
    } catch (e) {
      otpCodes.value[entry.id] = "错误";
    }
  }
}

let timer: ReturnType<typeof setInterval> | null = null;

onMounted(() => {
  updateOtpCodes();
  timer = setInterval(updateOtpCodes, 1000);
});

onUnmounted(() => {
  if (timer) clearInterval(timer);
});

function formatCountdown(seconds: number): string {
  return `${seconds}s`;
}

function copyCode(code: string) {
  if (navigator.clipboard) {
    navigator.clipboard.writeText(code);
    message.success("已复制");
  }
}

async function addEntry() {
  if (!newEntry.value.issuer.trim() || !newEntry.value.account.trim() || !newEntry.value.secret.trim()) {
    message.warning("请填写完整信息");
    return;
  }
  try {
    await store.createOtpEntry(newEntry.value);
    message.success("已添加");
    showAddModal.value = false;
    resetNewEntry();
  } catch (e: any) {
    message.error(e?.toString() || "添加失败");
  }
}

function resetNewEntry() {
  newEntry.value = {
    issuer: "",
    account: "",
    secret: "",
    interval: 30,
    digits: 6,
    algorithm: "SHA1",
  };
}

function editEntry(entry: OtpEntry) {
  editingId.value = entry.id;
  newEntry.value = {
    issuer: entry.issuer,
    account: entry.account,
    secret: entry.secret,
    interval: entry.interval,
    digits: entry.digits,
    algorithm: entry.algorithm,
  };
  showEditModal.value = true;
}

async function updateEntry() {
  if (!newEntry.value.issuer.trim() || !newEntry.value.account.trim() || !newEntry.value.secret.trim()) {
    message.warning("请填写完整信息");
    return;
  }
  try {
    await store.updateOtpEntry(editingId.value, newEntry.value);
    message.success("已更新");
    showEditModal.value = false;
    resetNewEntry();
  } catch (e: any) {
    message.error(e?.toString() || "更新失败");
  }
}

function deleteEntry(entry: OtpEntry) {
  dialog.warning({
    title: "删除 OTP",
    content: `确定删除 ${entry.issuer} - ${entry.account}？`,
    positiveText: "删除",
    negativeText: "取消",
    onPositiveClick: async () => {
      await store.deleteOtpEntry(entry.id);
      message.success("已删除");
    },
  });
}

function openImportModal() {
  importFileContent.value = "";
  showImportModal.value = true;
}

async function uploadQrImage() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const file = await open({
    title: "选择二维码图片",
    filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "bmp", "gif"] }],
  });
  if (!file || typeof file !== "string") return;

  try {
    const uri = await api.decodeQrImage(file);
    const parsed = parseOtpUri(uri);
    if (parsed) {
      newEntry.value = parsed;
      showAddModal.value = true;
      message.success("已识别二维码，请确认信息");
    } else {
      message.warning("无法解析二维码内容");
    }
  } catch (e: any) {
    message.error(e?.toString() || "识别失败");
  }
}

async function captureQrCode() {
  try {
    const uri = await api.captureQrCode();
    const parsed = parseOtpUri(uri);
    if (parsed) {
      newEntry.value = parsed;
      showAddModal.value = true;
      message.success("已识别二维码，请确认信息");
    } else {
      message.warning("无法解析二维码内容");
    }
  } catch (e: any) {
    message.error(e?.toString() || "截图识别失败");
  }
}

function parseOtpUri(uri: string): OtpEntryInput | null {
  try {
    const url = new URL(uri);
    if (url.protocol !== "otpauth:") return null;

    const decodedPath = decodeURIComponent(url.pathname.slice(1));
    const colonIdx = decodedPath.indexOf(":");
    let issuer = url.searchParams.get("issuer") || "";
    let account = decodedPath;
    if (colonIdx !== -1) {
      issuer = issuer || decodedPath.slice(0, colonIdx);
      account = decodedPath.slice(colonIdx + 1);
    }

    const secret = url.searchParams.get("secret") || "";
    const algorithm = (url.searchParams.get("algorithm") || "SHA1").toUpperCase();
    const digits = parseInt(url.searchParams.get("digits") || "6", 10);
    const interval = parseInt(url.searchParams.get("period") || "30", 10);

    if (!secret) return null;

    return {
      issuer: decodeURIComponent(issuer),
      account: decodeURIComponent(account),
      secret,
      interval,
      digits,
      algorithm,
    };
  } catch {
    return null;
  }
}

async function importFromFile() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const file = await open({
    title: "选择 OTP 导入文件",
    filters: [
      { name: "OTP 文件", extensions: ["json", "txt"] },
      { name: "JSON", extensions: ["json"] },
      { name: "文本", extensions: ["txt"] },
    ],
  });
  if (!file || typeof file !== "string") return;

  try {
    const content = await api.readFileText(file);
    const entries: OtpEntryInput[] = [];

    if (file.endsWith(".json")) {
      const data = JSON.parse(content);
      if (Array.isArray(data)) {
        for (const item of data) {
          if (item.issuer && item.account && item.secret) {
            entries.push({
              issuer: item.issuer,
              account: item.account,
              secret: item.secret,
              interval: item.interval || 30,
              digits: item.digits || 6,
              algorithm: item.algorithm || "SHA1",
            });
          }
        }
      } else if (data.otpaccounts && Array.isArray(data.otpaccounts)) {
        for (const account of data.otpaccounts) {
          if (account.issuer && account.name && account.secret) {
            entries.push({
              issuer: account.issuer,
              account: account.name,
              secret: account.secret,
              interval: account.period || 30,
              digits: account.digits || 6,
              algorithm: account.algorithm || "SHA1",
            });
          }
        }
      }
    } else {
      const lines = content.split(/\r?\n/).filter(l => l.trim());
      for (const line of lines) {
        const parsed = parseOtpUri(line.trim());
        if (parsed) entries.push(parsed);
      }
    }

    if (entries.length === 0) {
      message.warning("未找到有效的 OTP 数据");
      return;
    }

    const count = await store.importOtpEntries(entries);
    message.success(`成功导入 ${count} 条 OTP`);
    showImportModal.value = false;
  } catch (e: any) {
    message.error(e?.toString() || "导入失败");
  }
}
</script>

<template>
  <div class="otp-panel">
    <div class="header">
      <span class="h-title">OTP 验证码</span>
      <div class="spacer" />
      <n-button size="small" quaternary @click="captureQrCode">
        <template #icon><n-icon><CameraIcon /></n-icon></template>
        截图扫码
      </n-button>
      <n-button size="small" quaternary @click="uploadQrImage">
        <template #icon><n-icon><ImageIcon /></n-icon></template>
        上传二维码
      </n-button>
      <n-button size="small" quaternary @click="openImportModal">
        <template #icon><n-icon><FolderIcon /></n-icon></template>
        导入
      </n-button>
      <n-button size="small" quaternary @click="showAddModal = true">
        <template #icon><n-icon><AddIcon /></n-icon></template>
        添加
      </n-button>
    </div>

    <div class="scroll">
      <div v-if="otpEntries.length === 0" class="empty-state">
        <p>暂无 OTP 验证码</p>
        <p class="hint">点击"添加"手动录入，或点击"导入"从文件导入</p>
      </div>

      <div v-else class="otp-list">
        <div
          v-for="entry in otpEntries"
          :key="entry.id"
          class="otp-item"
          @click="copyCode(otpCodes[entry.id] || '')"
        >
          <div class="otp-header">
            <div class="otp-info">
              <div class="otp-issuer">{{ entry.issuer }}</div>
              <div class="otp-account">{{ entry.account }}</div>
            </div>
            <div class="otp-actions">
              <n-button text size="tiny" @click.stop="editEntry(entry)">
                <template #icon><n-icon :size="14"><CreateIcon /></n-icon></template>
              </n-button>
              <n-button text size="tiny" type="error" @click.stop="deleteEntry(entry)">
                <template #icon><n-icon :size="14"><TrashIcon /></n-icon></template>
              </n-button>
            </div>
          </div>
          <div class="otp-code-row">
            <span class="otp-code">{{ otpCodes[entry.id] || "---" }}</span>
            <span class="otp-countdown" :class="{ urgent: (otpCountdowns[entry.id] || 0) <= 5 }">
              {{ formatCountdown(otpCountdowns[entry.id] || 0) }}
            </span>
          </div>
          <div class="otp-meta">
            {{ entry.algorithm }} | {{ entry.digits }}位 | {{ entry.interval }}秒
          </div>
        </div>
      </div>
    </div>

    <n-modal v-model:show="showAddModal" preset="card" title="添加 OTP" style="width: 420px">
      <div class="form-row">
        <label>发行方</label>
        <n-input v-model:value="newEntry.issuer" placeholder="如：Google、GitHub" />
      </div>
      <div class="form-row">
        <label>账户</label>
        <n-input v-model:value="newEntry.account" placeholder="如：user@example.com" />
      </div>
      <div class="form-row">
        <label>密钥（Base32）</label>
        <n-input v-model:value="newEntry.secret" placeholder="如：JBSWY3DPEHPK3PXP" />
      </div>
      <div class="form-row-inline">
        <div class="form-row">
          <label>算法</label>
          <n-select v-model:value="newEntry.algorithm" :options="algorithmOptions" />
        </div>
        <div class="form-row">
          <label>位数</label>
          <n-select v-model:value="newEntry.digits" :options="[{ label: '6', value: 6 }, { label: '8', value: 8 }]" />
        </div>
        <div class="form-row">
          <label>间隔（秒）</label>
          <n-select v-model:value="newEntry.interval" :options="[{ label: '30', value: 30 }, { label: '60', value: 60 }]" />
        </div>
      </div>
      <template #footer>
        <div style="display: flex; gap: 12px; justify-content: flex-end;">
          <n-button @click="showAddModal = false">取消</n-button>
          <n-button type="primary" @click="addEntry">添加</n-button>
        </div>
      </template>
    </n-modal>

    <n-modal v-model:show="showEditModal" preset="card" title="编辑 OTP" style="width: 420px">
      <div class="form-row">
        <label>发行方</label>
        <n-input v-model:value="newEntry.issuer" placeholder="如：Google、GitHub" />
      </div>
      <div class="form-row">
        <label>账户</label>
        <n-input v-model:value="newEntry.account" placeholder="如：user@example.com" />
      </div>
      <div class="form-row">
        <label>密钥（Base32）</label>
        <n-input v-model:value="newEntry.secret" placeholder="如：JBSWY3DPEHPK3PXP" />
      </div>
      <div class="form-row-inline">
        <div class="form-row">
          <label>算法</label>
          <n-select v-model:value="newEntry.algorithm" :options="algorithmOptions" />
        </div>
        <div class="form-row">
          <label>位数</label>
          <n-select v-model:value="newEntry.digits" :options="[{ label: '6', value: 6 }, { label: '8', value: 8 }]" />
        </div>
        <div class="form-row">
          <label>间隔（秒）</label>
          <n-select v-model:value="newEntry.interval" :options="[{ label: '30', value: 30 }, { label: '60', value: 60 }]" />
        </div>
      </div>
      <template #footer>
        <div style="display: flex; gap: 12px; justify-content: flex-end;">
          <n-button @click="showEditModal = false">取消</n-button>
          <n-button type="primary" @click="updateEntry">保存</n-button>
        </div>
      </template>
    </n-modal>

    <n-modal v-model:show="showImportModal" preset="card" title="导入 OTP" style="width: 420px">
      <div class="import-info">
        <p>支持以下格式的文件：</p>
        <ul>
          <li><strong>JSON 文件</strong>：标准 OTP 导出格式，或包含 <code>otpaccounts</code> 数组的格式</li>
          <li><strong>TXT 文件</strong>：每行一个 <code>otpauth://</code> URI</li>
        </ul>
      </div>
      <template #footer>
        <div style="display: flex; gap: 12px; justify-content: flex-end;">
          <n-button @click="showImportModal = false">取消</n-button>
          <n-button type="primary" @click="importFromFile">选择文件导入</n-button>
        </div>
      </template>
    </n-modal>
  </div>
</template>

<style scoped>
.otp-panel {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg);
}
.header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  background: var(--panel);
}
.h-title {
  font-weight: 600;
  font-size: 15px;
}
.spacer {
  flex: 1;
}
.scroll {
  flex: 1;
  overflow-y: auto;
  padding: 20px 24px;
}
.empty-state {
  text-align: center;
  color: var(--text-sub);
  padding: 40px 20px;
}
.empty-state p {
  margin: 8px 0;
}
.hint {
  font-size: 13px;
  color: var(--text-sub);
}
.otp-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 16px;
}
.otp-item {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 14px;
  cursor: pointer;
  transition: border-color 0.15s, box-shadow 0.15s;
}
.otp-item:hover {
  border-color: var(--primary);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
}
.otp-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 10px;
}
.otp-info {
  flex: 1;
  min-width: 0;
}
.otp-issuer {
  font-weight: 600;
  font-size: 14px;
  margin-bottom: 2px;
}
.otp-account {
  font-size: 12px;
  color: var(--text-sub);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.otp-actions {
  display: flex;
  gap: 4px;
}
.otp-code-row {
  display: flex;
  align-items: baseline;
  gap: 12px;
  margin-bottom: 8px;
}
.otp-code {
  font-family: monospace;
  font-size: 28px;
  font-weight: 600;
  letter-spacing: 4px;
  color: var(--primary);
}
.otp-countdown {
  font-size: 13px;
  color: var(--text-sub);
  min-width: 30px;
}
.otp-countdown.urgent {
  color: #e53e3e;
  font-weight: 600;
}
.otp-meta {
  font-size: 11px;
  color: var(--text-sub);
}
.form-row {
  margin-bottom: 14px;
}
.form-row label {
  display: block;
  font-size: 13px;
  color: var(--text-sub);
  margin-bottom: 6px;
}
.form-row-inline {
  display: flex;
  gap: 12px;
}
.form-row-inline .form-row {
  flex: 1;
}
.import-info {
  font-size: 13px;
  color: var(--text-sub);
  line-height: 1.6;
}
.import-info ul {
  margin: 8px 0;
  padding-left: 20px;
}
.import-info code {
  background: var(--bg);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
}
</style>
