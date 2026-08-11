<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from "vue";
import type { Entry, EntryInput, Field } from "../types";
import { DEFAULT_TEMPLATE_FIELDS } from "../types";
import { useAppStore } from "../stores/app";
import { templateIcon } from "../utils/templateIcons";
import { NButton, NInput, NSelect, NIcon, useMessage, useDialog } from "naive-ui";
import { openUrl } from "@tauri-apps/plugin-opener";
import TypedFieldInput from "./TypedFieldInput.vue";
import FieldConfigDialog from "./FieldConfigDialog.vue";
import {
  CloseOutline,
  EyeOutline,
  EyeOffOutline,
  CopyOutline,
  CreateOutline,
  TrashOutline,
  RefreshOutline,
  AddOutline,
  RemoveOutline,
  OpenOutline,
} from "@vicons/ionicons5";

const props = defineProps<{
  entry: Entry | null;
  isNew: boolean;
  initialTemplateId?: string | null;
}>();
const emit = defineEmits<{
  (e: "close"): void;
  (e: "saved", id: string): void;
}>();

const store = useAppStore();
const message = useMessage();
const dialog = useDialog();

const category = ref<string>("login");
const title = ref("");
const icon = ref<string | null>(null);
const note = ref<string | null>(null);
const fields = ref<Field[]>([]);
const selectedTags = ref<string[]>([]);

// OTP-related state
const selectedOtpId = ref<string | null>(null);
const otpMode = ref<string | null>(null);
const otpCode = ref<string>("");
const otpCountdown = ref<number>(0);
const combinedCode = ref<string>("");

const otpOptions = computed(() =>
  store.otpEntries.map((otp) => ({
    label: `${otp.issuer} - ${otp.account}`,
    value: otp.id,
  }))
);

const otpModeOptions = [
  { label: "密码拼接", value: "password_concat" },
  { label: "二次验证", value: "secondary" },
];

const tagOptions = computed(() =>
  store.tags.map((t) => ({ label: t.name, value: t.name }))
);

function buildCatTree(parentId: string | null, depth: number): { label: string; value: string }[] {
  const children = store.categories.filter((c) => c.parent_id === parentId);
  const result: { label: string; value: string }[] = [];
  for (const c of children) {
    const prefix = depth > 0 ? "  ".repeat(depth) + "└ " : "";
    result.push({ label: prefix + c.name, value: c.id });
    result.push(...buildCatTree(c.id, depth + 1));
  }
  return result;
}

const categories = computed(() => buildCatTree(null, 0));

function catMeta(id: string) {
  const c = store.categoryById(id);
  return {
    color: c?.color || "#999",
    icon: templateIcon(c?.icon),
    label: c?.name || id,
  };
}



// OTP generation logic
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

async function generateTotp(secret: string, interval: number, digits: number, algorithm: string): Promise<string> {
  const now = Math.floor(Date.now() / 1000);
  const counter = Math.floor(now / interval);

  const counterBytes = new Uint8Array(8);
  let temp = counter;
  for (let i = 7; i >= 0; i--) {
    counterBytes[i] = temp & 0xff;
    temp = Math.floor(temp / 256);
  }

  const secretBytes = base32Decode(secret);

  const algorithmName = algorithm === "SHA256" ? "SHA-256" :
                        algorithm === "SHA512" ? "SHA-512" : "SHA-1";

  const key = await crypto.subtle.importKey(
    "raw",
    secretBytes,
    { name: "HMAC", hash: algorithmName },
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
  ) % Math.pow(10, digits);
  return code.toString().padStart(digits, "0");
}

async function updateOtpCode() {
  if (!selectedOtpId.value) {
    otpCode.value = "";
    otpCountdown.value = 0;
    combinedCode.value = "";
    return;
  }

  const otpEntry = store.otpEntries.find((e) => e.id === selectedOtpId.value);
  if (!otpEntry) return;

  try {
    otpCode.value = await generateTotp(
      otpEntry.secret,
      otpEntry.interval,
      otpEntry.digits,
      otpEntry.algorithm
    );
    otpCountdown.value = otpEntry.interval - (Math.floor(Date.now() / 1000) % otpEntry.interval);

    // Update combined code if mode is password_concat
    if (otpMode.value === "password_concat") {
      const passwordField = fields.value.find((f) => f.name === "密码");
      if (passwordField && passwordField.value) {
        combinedCode.value = passwordField.value + otpCode.value;
      } else {
        combinedCode.value = otpCode.value;
      }
    } else {
      combinedCode.value = "";
    }
  } catch (e) {
    otpCode.value = "错误";
    otpCountdown.value = 0;
  }
}

let otpTimer: ReturnType<typeof setInterval> | null = null;

onMounted(() => {
  updateOtpCode();
  otpTimer = setInterval(updateOtpCode, 1000);
});

onUnmounted(() => {
  if (otpTimer) clearInterval(otpTimer);
});

function fieldsFromTemplate(t: { fields: { name: string; field_type: string; secret: boolean }[] }): Field[] {
  return t.fields.map((f) => ({
    name: f.name,
    value: "",
    secret: f.secret,
    field_type: f.field_type,
  }));
}

function resetForm() {
  if (props.isNew) {
    category.value = "login";
    title.value = "";
    icon.value = null;
    note.value = null;
    selectedTags.value = [];
    selectedOtpId.value = null;
    otpMode.value = null;
    const tpl =
      store.templateById(props.initialTemplateId) ||
      store.templateById(store.templates[0]?.id);
    fields.value = tpl ? fieldsFromTemplate(tpl) : (DEFAULT_TEMPLATE_FIELDS as Field[]);
  } else if (props.entry) {
    const e = props.entry;
    category.value = e.category;
    title.value = e.title;
    icon.value = e.icon || null;
    note.value = e.note || null;
    fields.value = e.fields.map((f) => ({
      name: f.name,
      value: f.value,
      secret: !!f.secret,
      field_type: f.field_type || "text",
    }));
    selectedTags.value = e.tags.map((t) => t.name);

    // OTP 信息为条目的独立属性
    selectedOtpId.value = e.otp_id || null;
    otpMode.value = e.otp_mode || null;
  }
}

const editing = ref(false);

// 字段配置弹窗：记录正在编辑的字段下标
const fieldConfigIndex = ref<number | null>(null);

function openFieldConfig(idx: number) {
  fieldConfigIndex.value = idx;
}

function saveFieldConfig(cfg: { name: string; field_type: string; secret: boolean }) {
  const idx = fieldConfigIndex.value;
  if (idx === null || !fields.value[idx]) return;
  const f = fields.value[idx];
  fields.value[idx] = { ...f, ...cfg };
  fieldConfigIndex.value = null;
}



function onCategoryChange(cat: string) {
  category.value = cat;
}

function addField() {
  fields.value.push({ name: "", value: "", secret: false, field_type: "text" });
}
function removeField(idx: number) {
  fields.value.splice(idx, 1);
}

function onOtpSelect(otpId: string) {
  selectedOtpId.value = otpId;
  updateOtpCode();
}

function onOtpModeChange(mode: string) {
  otpMode.value = mode;
  updateOtpCode();
}

function formatCountdown(seconds: number): string {
  return `${seconds}s`;
}

function copyToClipboard(text: string) {
  if (navigator.clipboard) {
    navigator.clipboard.writeText(text);
    message.success("已复制");
  }
}

const reveal = ref<Record<number, boolean>>({});
const revealCombined = ref(false);

function openExternal(url: string) {
  openUrl(url).catch(() => message.error("打开链接失败"));
}

async function save() {
  if (!title.value.trim()) {
    message.warning("请填写标题");
    return;
  }

  // Prepare fields
  const allFields = fields.value
    .filter((f) => f.name.trim() !== "")
    .map((f) => ({
      name: f.name,
      value: f.value,
      secret: f.secret,
      field_type: f.field_type || "text",
    }));

  const input: EntryInput = {
    category: category.value,
    title: title.value.trim(),
    icon: icon.value,
    note: note.value,
    otp_id: selectedOtpId.value,
    otp_mode: selectedOtpId.value ? otpMode.value : null,
    fields: allFields,
    tags: selectedTags.value,
  };
  try {
    if (props.isNew) {
      const e = await store.createEntry(input);
      message.success("已保存");
      emit("saved", e.id);
    } else if (props.entry) {
      await store.updateEntry(props.entry.id, input);
      message.success("已更新");
      editing.value = false;
      emit("saved", props.entry.id);
    }
  } catch (e: any) {
    message.error(e?.toString() || "保存失败");
  }
}

function copyValue(v: string) {
  if (navigator.clipboard) {
    navigator.clipboard.writeText(v);
    message.success("已复制");
  }
}

function toTrash() {
  if (!props.entry) return;
  dialog.warning({
    title: "移入回收站",
    content: "确定将该条目移入回收站？",
    positiveText: "确定",
    negativeText: "取消",
    onPositiveClick: async () => {
      await store.deleteEntry(props.entry!.id);
      message.success("已移入回收站");
      emit("close");
    },
  });
}
function restore() {
  if (!props.entry) return;
  store.restoreEntry(props.entry.id).then(() => {
    message.success("已恢复");
    emit("close");
  });
}
function permanentDelete() {
  if (!props.entry) return;
  dialog.error({
    title: "彻底删除",
    content: "此操作不可恢复，确定彻底删除？",
    positiveText: "彻底删除",
    negativeText: "取消",
    onPositiveClick: async () => {
      await store.permanentlyDelete(props.entry!.id);
      message.success("已删除");
      emit("close");
    },
  });
}

function formatTime(ts: string | null): string {
  if (!ts) return "—";
  const d = new Date(Number(ts) * 1000);
  const pad = (n: number) => n.toString().padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

watch(
  () => [props.entry, props.isNew],
  () => {
    editing.value = props.isNew;
    resetForm();
  },
  { immediate: true }
);

watch(
  [selectedOtpId, otpMode],
  () => {
    updateOtpCode();
  }
);
</script>

<template>
  <div class="detail">
    <div class="header">
      <template v-if="isNew || editing">
        <span class="h-title">{{ isNew ? "新建密码" : "编辑密码" }}</span>
      </template>
      <template v-else>
        <span class="h-title">{{ entry?.title }}</span>
      </template>
      <div class="spacer" />
      <n-button v-if="!isNew && !editing" size="small" quaternary @click="editing = true">
        <template #icon><n-icon><CreateOutline /></n-icon></template>
      </n-button>
      <n-button v-if="!isNew && entry?.deleted_at" size="small" quaternary type="warning" @click="restore">
        <template #icon><n-icon><RefreshOutline /></n-icon></template>
      </n-button>
      <n-button v-if="!isNew && entry?.deleted_at" size="small" quaternary type="error" @click="permanentDelete">
        <template #icon><n-icon><RemoveOutline /></n-icon></template>
      </n-button>
      <n-button v-if="!isNew && !entry?.deleted_at && !editing" size="small" quaternary type="error" @click="toTrash">
        <template #icon><n-icon><TrashOutline /></n-icon></template>
      </n-button>
      <n-button size="small" quaternary @click="emit('close')">
        <template #icon><n-icon><CloseOutline /></n-icon></template>
      </n-button>
    </div>

    <div class="scroll">
      <template v-if="!isNew && !editing && entry">
        <div class="cat-badge">
          <n-icon :size="16" :color="catMeta(entry.category).color">
            <component :is="catMeta(entry.category).icon" />
          </n-icon>
          <span>{{ catMeta(entry.category).label }}</span>
        </div>
        <div class="field-row" v-for="(f, idx) in entry.fields" :key="idx">
          <div class="f-name">{{ f.name }}</div>
          <div class="f-value">
            <template v-if="f.field_type === 'url' && f.value">
              <a class="url-link" :href="f.value" target="_blank" rel="noopener">{{ f.value }}</a>
            </template>
            <span v-else-if="f.secret && !reveal[idx]">••••••••</span>
            <span v-else class="f-text" :class="{ multiline: f.field_type === 'multiline' }">{{ f.value || "—" }}</span>
          </div>
          <div class="f-actions">
            <n-button v-if="f.field_type === 'url' && f.value" text size="tiny" @click="openExternal(f.value)">
              <template #icon><n-icon :size="14"><OpenOutline /></n-icon></template>
            </n-button>
            <n-button v-if="f.secret" text size="tiny" @click="reveal[idx] = !reveal[idx]">
              <template #icon><n-icon :size="14"><component :is="reveal[idx] ? EyeOffOutline : EyeOutline" /></n-icon></template>
            </n-button>
            <n-button v-if="f.value" text size="tiny" @click="copyValue(f.value)">
              <template #icon><n-icon :size="14"><CopyOutline /></n-icon></template>
            </n-button>
          </div>
        </div>

        <!-- OTP Display Section -->
        <div class="field-row" v-if="selectedOtpId && otpCode">
          <div class="f-name">OTP 动态码</div>
          <div class="f-value">
            <span>{{ otpCode }}</span>
            <span class="otp-countdown" :class="{ urgent: otpCountdown <= 5 }">{{ formatCountdown(otpCountdown) }}</span>
          </div>
          <div class="f-actions">
            <n-button text size="tiny" @click="copyToClipboard(otpCode)">
              <template #icon><n-icon :size="14"><CopyOutline /></n-icon></template>
            </n-button>
          </div>
        </div>

        <div class="field-row" v-if="selectedOtpId && otpMode === 'password_concat' && combinedCode">
          <div class="f-name">密码+OTP</div>
          <div class="f-value">
            <span v-if="!revealCombined">••••••••</span>
            <span v-else>{{ combinedCode }}</span>
            <span class="otp-countdown" :class="{ urgent: otpCountdown <= 5 }">{{ formatCountdown(otpCountdown) }}</span>
          </div>
          <div class="f-actions">
            <n-button text size="tiny" @click="revealCombined = !revealCombined">
              <template #icon><n-icon :size="14"><component :is="revealCombined ? EyeOffOutline : EyeOutline" /></n-icon></template>
            </n-button>
            <n-button text size="tiny" @click="copyToClipboard(combinedCode)">
              <template #icon><n-icon :size="14"><CopyOutline /></n-icon></template>
            </n-button>
          </div>
        </div>

        <div class="note-block" v-if="entry.note">
          <div class="note-title">备注</div>
          <div class="note-body">{{ entry.note }}</div>
        </div>
        <div class="tag-block" v-if="entry.tags.length">
          <span v-for="t in entry.tags" :key="t.id" class="tag" :style="{ background: t.color || '#ccc' }">{{ t.name }}</span>
        </div>
        <div class="time-info">
          <div class="time-row">
            <span class="time-label">创建时间</span>
            <span class="time-value">{{ formatTime(entry.created_at) }}</span>
          </div>
          <div class="time-row">
            <span class="time-label">修改时间</span>
            <span class="time-value">{{ formatTime(entry.updated_at) }}</span>
          </div>
        </div>
      </template>

      <template v-else>
        <div class="form-row">
          <label>类别</label>
          <n-select
            :options="categories"
            v-model:value="category"
            @update:value="onCategoryChange"
          />
        </div>
        <div class="form-row">
          <label>标题</label>
          <n-input v-model:value="title" placeholder="如：GitHub、公司数据库" />
        </div>

        <div class="fields-section">
          <div class="fields-title">
            字段
            <span class="fields-hint">点击字段名可配置名称 / 类型 / 敏感</span>
          </div>
          <div class="field-edit" v-for="(f, idx) in fields" :key="idx">
            <button type="button" class="f-name-btn" @click="openFieldConfig(idx)">
              {{ f.name || "未命名字段" }}
            </button>
            <TypedFieldInput
              v-model="f.value"
              :type="f.field_type || 'text'"
              :placeholder="f.secret ? '密码/敏感值' : '值'"
              class="f-val-input"
            />
            <n-button v-if="f.field_type === 'password'" text size="tiny" @click="reveal[idx] = !reveal[idx]">
              <template #icon><n-icon :size="14"><component :is="reveal[idx] ? EyeOffOutline : EyeOutline" /></n-icon></template>
            </n-button>
            <n-button text type="error" size="tiny" @click="removeField(idx)">
              <template #icon><n-icon><CloseOutline /></n-icon></template>
            </n-button>
          </div>
          <n-button dashed size="small" @click="addField">
            <template #icon><n-icon><AddOutline /></n-icon></template>
            添加字段
          </n-button>
        </div>

        <div class="form-row" style="margin-top: 16px">
          <label>OTP验证码</label>
          <n-select
            :options="otpOptions"
            v-model:value="selectedOtpId"
            @update:value="onOtpSelect"
            placeholder="选择OTP验证码（可选）"
            clearable
          />
        </div>

        <div class="form-row" v-if="selectedOtpId">
          <label>OTP验证模式</label>
          <n-select
            :options="otpModeOptions"
            v-model:value="otpMode"
            @update:value="onOtpModeChange"
            placeholder="选择验证模式"
          />
        </div>

        <div class="form-row" style="margin-top: 16px">
          <label>标签</label>
          <n-select
            multiple
            filterable
            tag
            :options="tagOptions"
            v-model:value="selectedTags"
            placeholder="选择或输入新标签"
          />
        </div>

        <div class="form-row form-row-textarea">
          <label>备注</label>
          <n-input
            v-model:value="note"
            type="textarea"
            :autosize="{ minRows: 3, maxRows: 6 }"
            placeholder="可选备注"
          />
        </div>
      </template>
    </div>

    <FieldConfigDialog
      :show="fieldConfigIndex !== null"
      :field="fieldConfigIndex !== null ? fields[fieldConfigIndex] : null"
      @update:show="fieldConfigIndex = $event ? fieldConfigIndex : null"
      @save="saveFieldConfig"
    />

    <div class="footer" v-if="isNew || editing">
      <n-button @click="emit('close')">取消</n-button>
      <n-button type="primary" @click="save">保存</n-button>
    </div>
  </div>
</template>

<style scoped>
.detail {
  flex: 1;
  min-width: 0;
  min-height: 0;
  align-self: stretch;
  display: flex;
  flex-direction: column;
  overflow: hidden;
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
.cat-badge {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 4px 12px;
  border-radius: 8px;
  background: var(--panel);
  border: 1px solid var(--border);
  font-size: 13px;
  margin-bottom: 16px;
}
.field-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 0;
  border-bottom: 1px solid var(--border);
}
.f-name {
  width: 120px;
  color: var(--text-sub);
  font-size: 13px;
  flex-shrink: 0;
}
.f-value {
  flex: 1;
  font-family: monospace;
  word-break: break-all;
}
.url-link {
  color: var(--primary);
  text-decoration: underline;
  word-break: break-all;
}
.f-text.multiline {
  white-space: pre-wrap;
  font-family: inherit;
}
.url-link {
  color: var(--primary);
  text-decoration: underline;
}
.f-text.multiline {
  white-space: pre-wrap;
  font-family: inherit;
}
.f-actions {
  display: flex;
  gap: 8px;
  flex-shrink: 0;
}
.note-block {
  margin-top: 16px;
}
.note-title {
  font-size: 13px;
  color: var(--text-sub);
  margin-bottom: 6px;
}
.note-body {
  white-space: pre-wrap;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px;
}
.tag-block {
  margin-top: 16px;
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
.tag {
  font-size: 12px;
  color: #fff;
  padding: 2px 10px;
  border-radius: 8px;
}
.time-info {
  margin-top: 16px;
  padding-top: 12px;
  border-top: 1px solid var(--border);
}
.time-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
}
.time-label {
  font-size: 13px;
  color: var(--text-sub);
  width: 80px;
  flex-shrink: 0;
}
.time-value {
  font-size: 13px;
  color: var(--text);
}
.form-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 14px;
}
.form-row label {
  font-size: 13px;
  color: var(--text-sub);
  width: 100px;
  flex-shrink: 0;
}
.form-row-textarea {
  align-items: flex-start;
}
.form-row-textarea label {
  margin-top: 6px;
}
.fields-section {
  margin: 16px 0;
  padding: 16px 0;
  border-top: 1px solid var(--border);
  border-bottom: 1px solid var(--border);
}
.fields-title {
  font-size: 13px;
  color: var(--text-sub);
  margin: 0 0 8px;
  font-weight: 600;
}
.fields-hint {
  font-weight: 400;
  font-size: 12px;
  color: var(--text-sub);
  margin-left: 6px;
}
.field-edit {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
.f-name-btn {
  width: 100px;
  flex-shrink: 0;
  font-size: 13px;
  color: var(--text);
  background: none;
  border: none;
  padding: 0;
  text-align: left;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  text-decoration: underline;
  cursor: pointer;
}
.f-name-btn:hover {
  color: var(--primary);
}
.f-val-input {
  flex: 1;
}
.footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 12px 24px;
  border-top: 1px solid var(--border);
  background: var(--bg);
}
.otp-countdown {
  font-size: 13px;
  color: var(--text-sub);
  min-width: 30px;
  margin-left: 8px;
}
.otp-countdown.urgent {
  color: #e53e3e;
  font-weight: 600;
}
</style>
