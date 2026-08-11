<script setup lang="ts">
import { h } from "vue";
import { NIcon, NSelect } from "naive-ui";
import { TEMPLATE_ICON_OPTIONS } from "../types";
import { templateIcon } from "../utils/templateIcons";

const props = defineProps<{
  value: string | null;
  placeholder?: string;
}>();
const emit = defineEmits<{
  (e: "update:value", v: string | null): void;
}>();

const options = TEMPLATE_ICON_OPTIONS.map((o) => ({
  label: o.label,
  value: o.value,
  iconKey: o.value,
}));

function renderLabel(option: { label: string; iconKey: string }) {
  return h("div", { style: "display:flex;align-items:center;gap:8px" }, [
    h(NIcon, { size: 16 }, { default: () => h(templateIcon(option.iconKey)) }),
    h("span", null, option.label),
  ]);
}
</script>

<template>
  <n-select
    :value="props.value"
    :options="options"
    :placeholder="props.placeholder || '选择图标（可选）'"
    clearable
    :render-label="renderLabel"
    @update:value="emit('update:value', $event)"
  />
</template>
