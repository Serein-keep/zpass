<script setup lang="ts">
import { ref, watch } from "vue";
import {
  NModal,
  NInput,
  NSelect,
  NCheckbox,
  NButton,
  useMessage,
} from "naive-ui";
import type { Field, TemplateFieldInput } from "../types";
import { FIELD_TYPE_OPTIONS } from "../types";

const props = defineProps<{
  show: boolean;
  field: Field | null;
}>();
const emit = defineEmits<{
  (e: "update:show", v: boolean): void;
  (e: "save", cfg: TemplateFieldInput): void;
}>();

const message = useMessage();

const name = ref("");
const fieldType = ref("text");
const secret = ref(false);

watch(
  () => [props.show, props.field],
  () => {
    if (props.show && props.field) {
      name.value = props.field.name;
      fieldType.value = props.field.field_type || "text";
      secret.value = !!props.field.secret;
    }
  },
  { immediate: true }
);

// 密码类型强制敏感，与模板编辑器保持一致
function onTypeChange() {
  if (fieldType.value === "password") secret.value = true;
}

function confirm() {
  if (!name.value.trim()) {
    message.warning("请填写字段名");
    return;
  }
  emit("save", {
    name: name.value.trim(),
    field_type: fieldType.value,
    secret: secret.value,
  });
  emit("update:show", false);
}
</script>

<template>
  <n-modal
    :show="props.show"
    preset="card"
    title="字段配置"
    style="width: 420px"
    @update:show="emit('update:show', $event)"
  >
    <div class="cfg">
      <div class="row">
        <label>字段名</label>
        <n-input v-model:value="name" placeholder="如：用户名、密码" />
      </div>
      <div class="row">
        <label>类型</label>
        <n-select
          v-model:value="fieldType"
          :options="FIELD_TYPE_OPTIONS"
          @update:value="onTypeChange"
        />
      </div>
      <div class="row">
        <label>是否敏感</label>
        <n-checkbox
          v-model:checked="secret"
          :disabled="fieldType === 'password'"
          title="密码类型强制敏感"
        >
          敏感字段（查看详情时默认隐藏）
        </n-checkbox>
      </div>
      <div class="footer">
        <n-button @click="emit('update:show', false)">取消</n-button>
        <n-button type="primary" @click="confirm">确定</n-button>
      </div>
    </div>
  </n-modal>
</template>

<style scoped>
.cfg {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.row {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.row label {
  font-size: 13px;
  color: var(--text-sub);
}
.footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 8px;
}
</style>
