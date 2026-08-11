<script setup lang="ts">
import { computed } from "vue";
import { NInput, NInputNumber } from "naive-ui";

const props = withDefaults(
  defineProps<{
    type: string;
    modelValue: string;
    placeholder?: string;
    disabled?: boolean;
  }>(),
  { placeholder: "", disabled: false }
);
const emit = defineEmits<{
  (e: "update:modelValue", v: string): void;
}>();

const isNativeInput = computed(() =>
  ["url", "email", "phone", "month", "date"].includes(props.type)
);

function onNativeInput(e: Event) {
  emit("update:modelValue", (e.target as HTMLInputElement).value);
}

function inputTypeAttr(type: string): string {
  switch (type) {
    case "url":
      return "url";
    case "email":
      return "email";
    case "phone":
      return "tel";
    case "month":
      return "month";
    case "date":
      return "date";
    default:
      return "text";
  }
}
</script>

<template>
  <!-- 多行文本：默认 4 行 -->
  <n-input
    v-if="type === 'multiline'"
    :value="modelValue"
    type="textarea"
    :autosize="{ minRows: 4, maxRows: 12 }"
    :placeholder="placeholder"
    :disabled="disabled"
    @update:value="emit('update:modelValue', $event)"
  />
  <!-- 密码：内建眼睛切换 -->
  <n-input
    v-else-if="type === 'password'"
    :value="modelValue"
    type="password"
    show-password-on="click"
    :placeholder="placeholder"
    :disabled="disabled"
    @update:value="emit('update:modelValue', $event)"
  />
  <!-- 数字 -->
  <n-input-number
    v-else-if="type === 'number'"
    :value="modelValue === '' ? null : Number(modelValue)"
    :placeholder="placeholder"
    :disabled="disabled"
    style="width: 100%"
    @update:value="emit('update:modelValue', $event === null ? '' : String($event))"
  />
  <!-- url/email/phone/month/date：原生 input 以获得对应键盘与校验 -->
  <input
    v-else-if="isNativeInput"
    class="native-input"
    :type="inputTypeAttr(type)"
    :value="modelValue"
    :placeholder="placeholder"
    :disabled="disabled"
    autocapitalize="off"
    spellcheck="false"
    @input="onNativeInput"
  />
  <!-- 文本兜底 -->
  <n-input
    v-else
    :value="modelValue"
    :placeholder="placeholder"
    :disabled="disabled"
    @update:value="emit('update:modelValue', $event)"
  />
</template>

<style scoped>
.native-input {
  width: 100%;
  padding: 9px 12px;
  border: 1px solid var(--border);
  border-radius: 3px;
  background: var(--panel);
  color: var(--text);
  font-size: 14px;
  outline: none;
  transition: border-color 0.2s;
}
.native-input:focus {
  border-color: var(--primary);
}
.native-input:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
</style>