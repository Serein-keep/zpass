<script setup lang="ts">
import { ref, watch, computed } from "vue";
import {
  NButton,
  NIcon,
  NModal,
  NInput,
  NSelect,
  useMessage,
} from "naive-ui";
import { uiIcon } from "../utils/templateIcons";
import { useAppStore } from "../stores/app";
import type { Template, TemplateFieldInput } from "../types";
import { FIELD_TYPE_OPTIONS, DEFAULT_TEMPLATE_FIELDS } from "../types";
import IconSelect from "./IconSelect.vue";

const props = defineProps<{
  show: boolean;
  template: Template | null;
}>();
const emit = defineEmits<{
  (e: "update:show", v: boolean): void;
  (e: "saved"): void;
}>();

const store = useAppStore();
const message = useMessage();

const AddIcon = uiIcon("add");
const CloseIcon = uiIcon("close");
const ChevronUpIcon = uiIcon("chevron-up");
const ChevronDownIcon = uiIcon("chevron-down");

const name = ref("");
const categoryId = ref<string | null>(null);
const icon = ref<string | null>(null);
const note = ref("");
const fields = ref<TemplateFieldInput[]>([]);

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

const catOptions = computed(() => buildCatTree(null, 0));

function reset() {
  if (props.template) {
    name.value = props.template.name;
    categoryId.value = props.template.category_id;
    icon.value = props.template.icon || null;
    note.value = props.template.note || "";
    fields.value = props.template.fields.map((f) => ({ ...f }));
  } else {
    name.value = "";
    categoryId.value = store.categories[0]?.id || null;
    icon.value = null;
    note.value = "";
    fields.value = DEFAULT_TEMPLATE_FIELDS.map((f) => ({ ...f }));
  }
}

function addField() {
  fields.value.push({ name: "", field_type: "text", secret: false });
}
function moveField(idx: number, dir: -1 | 1) {
  const to = idx + dir;
  if (to < 0 || to >= fields.value.length) return;
  const arr = fields.value;
  [arr[idx], arr[to]] = [arr[to], arr[idx]];
}
function removeField(idx: number) {
  fields.value.splice(idx, 1);
}
function onTypeChange(f: TemplateFieldInput) {
  if (f.field_type === "password") f.secret = true;
}

async function save() {
  if (!name.value.trim()) {
    message.warning("请填写模板名称");
    return;
  }
  if (!categoryId.value) {
    message.warning("请选择模板类别");
    return;
  }
  const cleaned = fields.value.filter((f) => f.name.trim() !== "");
  if (cleaned.length === 0) {
    message.warning("模板至少需要一个字段");
    return;
  }
  const names = new Set<string>();
  for (const f of cleaned) {
    const n = f.name.trim();
    if (names.has(n)) {
      message.warning(`字段名重复: ${n}`);
      return;
    }
    names.add(n);
  }
  const input = {
    category_id: categoryId.value,
    name: name.value.trim(),
    icon: icon.value,
    note: note.value.trim() || null,
    fields: cleaned,
  };
  try {
    if (props.template) {
      await store.updateTemplate(props.template.id, input);
    } else {
      await store.createTemplate(input);
    }
    message.success("已保存");
    emit("saved");
    emit("update:show", false);
  } catch (e: any) {
    message.error(e?.toString() || "保存失败");
  }
}

watch(
  () => [props.show, props.template],
  () => {
    if (props.show) reset();
  },
  { immediate: true }
);
</script>

<template>
  <n-modal
    :show="props.show"
    preset="card"
    :title="props.template ? '编辑模板' : '新建模板'"
    style="width: 560px"
    @update:show="emit('update:show', $event)"
  >
    <div class="editor">
      <div class="row">
        <label>模板名称</label>
        <n-input v-model:value="name" placeholder="如：SSH服务器、Wi-Fi" />
      </div>

      <div class="row">
        <label>模板图标</label>
        <IconSelect v-model:value="icon" />
      </div>

      <div class="row">
        <label>模板类别</label>
        <n-select
          v-model:value="categoryId"
          :options="catOptions"
          placeholder="选择类别"
        />
      </div>

      <div class="fields-title">表单字段</div>
      <div v-for="(f, idx) in fields" :key="idx" class="field-line">
        <n-input
          v-model:value="f.name"
          placeholder="字段名"
          size="small"
          class="f-name"
        />
        <n-select
          v-model:value="f.field_type"
          :options="FIELD_TYPE_OPTIONS"
          size="small"
          class="f-type"
          @update:value="onTypeChange(f)"
        />
        <label class="sec-label" title="密码类型强制敏感">
          <input
            type="checkbox"
            v-model="f.secret"
            :disabled="f.field_type === 'password'"
          />
          敏感
        </label>
        <n-button text size="tiny" @click="moveField(idx, -1)">
          <template #icon><n-icon><ChevronUpIcon /></n-icon></template>
        </n-button>
        <n-button text size="tiny" @click="moveField(idx, 1)">
          <template #icon><n-icon><ChevronDownIcon /></n-icon></template>
        </n-button>
        <n-button text type="error" size="tiny" @click="removeField(idx)">
          <template #icon><n-icon><CloseIcon /></n-icon></template>
        </n-button>
      </div>
      <n-button dashed size="small" @click="addField">
        <template #icon><n-icon><AddIcon /></n-icon></template>
        添加字段
      </n-button>

      <div class="row row-textarea">
        <label>模板备注</label>
        <n-input
          v-model:value="note"
          type="textarea"
          :autosize="{ minRows: 2, maxRows: 5 }"
          placeholder="仅作模板说明，不会出现在密码条目中"
        />
      </div>

      <div class="footer">
        <n-button @click="emit('update:show', false)">取消</n-button>
        <n-button type="primary" @click="save">保存</n-button>
      </div>
    </div>
  </n-modal>
</template>

<style scoped>
.editor {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.row {
  display: flex;
  align-items: center;
  gap: 12px;
}
.row label,
.fields-title {
  font-size: 13px;
  color: var(--text-sub);
  width: 80px;
  flex-shrink: 0;
}
.row-textarea {
  align-items: flex-start;
}
.row-textarea label {
  margin-top: 6px;
}
.fields-title {
  font-weight: 600;
  margin-top: 4px;
}
.cat-line {
  display: flex;
  align-items: center;
  gap: 8px;
}
.cat-line .n-select {
  flex: 1;
}
.field-line {
  display: flex;
  align-items: center;
  gap: 8px;
}
.f-name {
  width: 150px;
  flex-shrink: 0;
}
.f-type {
  flex: 1;
}
.sec-label {
  font-size: 12px;
  color: var(--text-sub);
  display: flex;
  align-items: center;
  gap: 4px;
  flex-shrink: 0;
}
.footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 8px;
}
</style>