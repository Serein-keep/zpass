<script setup lang="ts">
import { ref, watch } from "vue";
import { NButton, NModal, NInput, useMessage } from "naive-ui";
import { useAppStore } from "../stores/app";

const props = defineProps<{
  show: boolean;
  tag: { id: string; name: string; color?: string | null } | null;
}>();
const emit = defineEmits<{ (e: "update:show", v: boolean): void }>();

const store = useAppStore();
const message = useMessage();

const name = ref("");
const color = ref("#3b5bdb");

function reset() {
  if (props.tag) {
    name.value = props.tag.name;
    color.value = props.tag.color || "#3b5bdb";
  } else {
    name.value = "";
    color.value = "#3b5bdb";
  }
}

async function save() {
  const n = name.value.trim();
  if (!n) {
    message.warning("请输入标签名称");
    return;
  }
  try {
    if (props.tag) {
      await store.deleteTag(props.tag.name);
    }
    await store.createTag(n, color.value);
    message.success("已保存");
    emit("update:show", false);
  } catch (e: any) {
    message.error(e?.toString() || "保存失败");
  }
}

watch(
  () => [props.show, props.tag],
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
    :title="props.tag ? '编辑标签' : '新建标签'"
    style="width: 360px"
    @update:show="emit('update:show', $event)"
  >
    <div class="tag-form">
      <div class="row">
        <label>标签名称</label>
        <div class="field-wrap">
          <n-input v-model:value="name" placeholder="标签名称" @keyup.enter="save" />
        </div>
      </div>
      <div class="row">
        <label>颜色</label>
        <div class="field-wrap">
          <div class="color-line">
            <input v-model="color" type="color" class="color-input" />
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
.tag-form {
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
