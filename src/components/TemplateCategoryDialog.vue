<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { NButton, NModal, NInput, useMessage } from "naive-ui";
import { useAppStore } from "../stores/app";
import type { Category, CategoryInput } from "../types";
import IconSelect from "./IconSelect.vue";

const props = defineProps<{
  show: boolean;
  category: Category | null;
  parent: Category | null;
}>();
const emit = defineEmits<{ (e: "update:show", v: boolean): void }>();

const store = useAppStore();
const message = useMessage();

const name = ref("");
const icon = ref<string | null>(null);
const color = ref<string | null>(null);

const colorInput = computed({
  get: () => color.value || "#3b5bdb",
  set: (v: string) => {
    color.value = v;
  },
});

function reset() {
  if (props.category) {
    name.value = props.category.name;
    icon.value = props.category.icon || null;
    color.value = props.category.color || null;
  } else {
    name.value = "";
    icon.value = null;
    color.value = null;
  }
}

async function save() {
  const n = name.value.trim();
  if (!n) {
    message.warning("请输入类别名称");
    return;
  }
  const input: CategoryInput = {
    name: n,
    parent_id: props.category?.parent_id ?? props.parent?.id ?? null,
    icon: icon.value,
    color: color.value,
  };
  try {
    if (props.category) {
      await store.updateCategory(props.category.id, input);
    } else {
      await store.createCategory(input);
    }
    message.success("已保存");
    emit("update:show", false);
  } catch (e: any) {
    message.error(e?.toString() || "保存失败");
  }
}

watch(
  () => [props.show, props.category],
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
    :title="props.category ? '编辑类别' : '新建类别'"
    style="width: 400px"
    @update:show="emit('update:show', $event)"
  >
    <div class="cat-form">
      <div class="row">
        <label>类别名称</label>
        <div class="field-wrap">
          <n-input v-model:value="name" placeholder="类别名称" @keyup.enter="save" />
        </div>
      </div>
      <div class="row">
        <label>图标</label>
        <div class="field-wrap">
          <IconSelect v-model:value="icon" />
        </div>
      </div>
      <div class="row">
        <label>颜色</label>
        <div class="field-wrap">
          <div class="color-line">
            <input
              v-model="colorInput"
              type="color"
              class="color-input"
            />
            <n-input v-model:value="color" placeholder="#3b5bdb" size="small" />
          </div>
        </div>
      </div>
      <div class="footer">
        <n-button @click="emit('update:show', false)">取消</n-button>
        <n-button type="primary" @click="save">保存</n-button>
      </div>
    </div>
  </n-modal>
</template>

<style scoped>
.cat-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.row {
  display: flex;
  align-items: center;
  gap: 12px;
}
.row label {
  font-size: 13px;
  color: var(--text-sub);
  width: 80px;
  flex-shrink: 0;
}
.field-wrap {
  flex: 1;
  min-width: 0;
}
.color-line {
  display: flex;
  align-items: center;
  gap: 8px;
  flex: 1;
}
.color-input {
  width: 40px;
  height: 32px;
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 2px;
  background: var(--bg);
  cursor: pointer;
  flex-shrink: 0;
}
.footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 8px;
}
</style>
