<script setup lang="ts">
import { ref, computed } from "vue";
import { NButton, NIcon, NSelect, NEmpty, useDialog, useMessage } from "naive-ui";
import { useAppStore } from "../stores/app";
import type { Template } from "../types";
import { templateIcon, uiIcon } from "../utils/templateIcons";

const emit = defineEmits<{
  (e: "edit", template: Template): void;
  (e: "create"): void;
}>();

const store = useAppStore();
const dialog = useDialog();
const message = useMessage();

const AddIcon = uiIcon("add");
const CreateIcon = uiIcon("create");
const TrashIcon = uiIcon("trash");

const catFilter = ref<string>("all");

const catOptions = computed(() => [
  { label: "全部类别", value: "all" },
  ...store.categories.map((c) => ({ label: c.name, value: c.id })),
]);

const filteredTemplates = computed(() =>
  catFilter.value !== "all"
    ? store.templates.filter((t) => t.category_id === catFilter.value)
    : store.templates
);

function remove(t: Template) {
  dialog.warning({
    title: "删除模板",
    content: `确定删除模板「${t.name}」？已创建的密码条目不受影响。`,
    positiveText: "删除",
    negativeText: "取消",
    onPositiveClick: async () => {
      try {
        await store.deleteTemplate(t.id);
        message.success("已删除");
      } catch (e: any) {
        message.error(e?.toString() || "删除失败");
      }
    },
  });
}
</script>

<template>
  <div class="library">
    <div class="toolbar">
      <n-select
        v-model:value="catFilter"
        :options="catOptions"
        class="cat-filter"
        placeholder="筛选类别"
      />
      <div class="spacer" />
      <n-button quaternary @click="emit('create')">
        <template #icon><n-icon><AddIcon /></n-icon></template>
        新建模板
      </n-button>
    </div>

    <div v-if="filteredTemplates.length === 0" class="empty-box">
      <n-empty
        :description="store.templates.length === 0 ? '暂无模板，点击右上角新建' : '该类别下暂无模板'"
      />
    </div>

    <div v-else class="grid">
      <div v-for="t in filteredTemplates" :key="t.id" class="card">
        <div class="card-left">
          <div class="card-icon" :style="{ background: (store.categoryById(t.category_id)?.color || 'var(--primary)') + '18' }">
            <n-icon :size="22" :color="store.categoryById(t.category_id)?.color || 'var(--primary)'">
              <component :is="templateIcon(t.icon)" />
            </n-icon>
          </div>
        </div>
        <div class="card-right">
          <div class="card-top">
            <div class="card-info">
              <div class="t-name">{{ t.name }}</div>
              <div class="t-cat">
                <n-icon :size="12">
                  <component :is="templateIcon(store.categoryById(t.category_id)?.icon)" />
                </n-icon>
                {{ t.category_name }}
              </div>
            </div>
          <div class="card-actions" v-if="!t.is_builtin">
            <n-button text size="small" @click="emit('edit', t)">
                <template #icon><n-icon :size="14"><CreateIcon /></n-icon></template>
              </n-button>
              <n-button text size="small" type="error" @click="remove(t)">
                <template #icon><n-icon :size="14"><TrashIcon /></n-icon></template>
              </n-button>
            </div>
          </div>
          <div class="card-meta">
            <span>{{ t.fields.length }} 个字段</span>
          </div>
          <div v-if="t.note" class="t-note" :title="t.note">{{ t.note }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.library {
  flex: 1;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.toolbar {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  background: var(--panel);
}
.cat-filter {
  width: 180px;
}
.spacer {
  flex: 1;
}
.empty-box {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
.grid {
  flex: 1;
  overflow-y: auto;
  padding: 16px;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 10px;
  align-content: start;
}
.card {
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--panel);
  display: flex;
  flex-direction: row;
  align-items: stretch;
  overflow: hidden;
  transition: box-shadow 0.15s, border-color 0.15s;
}
.card:hover {
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
  border-color: var(--primary);
}
.card-left {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 14px 12px;
}
.card-icon {
  width: 44px;
  height: 44px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.card-right {
  flex: 1;
  min-width: 0;
  padding: 12px 14px 12px 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.card-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 8px;
}
.card-info {
  min-width: 0;
}
.card-actions {
  display: flex;
  gap: 0;
  flex-shrink: 0;
  opacity: 0;
  transition: opacity 0.15s;
}
.card:hover .card-actions {
  opacity: 1;
}
.t-name {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.t-cat {
  font-size: 12px;
  color: var(--text-sub);
  display: flex;
  align-items: center;
  gap: 4px;
  margin-top: 2px;
}
.card-meta {
  font-size: 12px;
  color: var(--text-sub);
}
.t-note {
  font-size: 12px;
  color: var(--text-sub);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-top: 2px;
}
</style>
