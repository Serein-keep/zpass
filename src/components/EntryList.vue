<script setup lang="ts">
import type { Entry } from "../types";
import { useAppStore } from "../stores/app";
import { NIcon } from "naive-ui";
import { templateIcon } from "../utils/templateIcons";

defineProps<{
  entries: Entry[];
  selectedId: string | null;
}>();
const emit = defineEmits<{ (e: "select", id: string): void }>();

const store = useAppStore();

function catMeta(id: string) {
  const c = store.categoryById(id);
  return { color: c?.color || "#999", icon: templateIcon(c?.icon) };
}

function subtitle(e: Entry) {
  const user = e.fields.find((f) => ["用户名", "邮箱地址", "数据库名", "用户"].includes(f.name));
  if (user?.value) return user.value;
  const first = e.fields.find((f) => f.value);
  return first?.value || "无字段内容";
}
</script>

<template>
  <div class="list">
    <div v-if="entries.length === 0" class="empty">没有匹配的条目</div>
    <div
      v-for="e in entries"
      :key="e.id"
      class="row"
      :class="{ active: e.id === selectedId }"
      @click="emit('select', e.id)"
    >
      <div class="avatar" :style="{ background: catMeta(e.category).color }">
        <n-icon :size="20" color="#fff">
          <component :is="catMeta(e.category).icon" />
        </n-icon>
      </div>
      <div class="info">
        <div class="title">{{ e.title }}</div>
        <div class="sub">{{ subtitle(e) }}</div>
      </div>
      <div class="tags">
        <span
          v-for="t in e.tags.slice(0, 2)"
          :key="t.id"
          class="tag"
          :style="{ background: t.color || '#ccc' }"
          >{{ t.name }}</span
        >
      </div>
    </div>
  </div>
</template>

<style scoped>
.list {
  width: 340px;
  flex: none;
  align-self: stretch;
  min-height: 0;
  border-right: 1px solid var(--border);
  overflow-y: auto;
  background: var(--panel);
}
.empty {
  padding: 40px 16px;
  text-align: center;
  color: var(--text-sub);
  font-size: 13px;
}
.row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  cursor: pointer;
  border-bottom: 1px solid var(--border);
  transition: background 0.15s;
}
.row:hover {
  background: var(--bg);
}
.row.active {
  background: color-mix(in srgb, var(--primary) 12%, transparent);
  border-left: 3px solid var(--primary);
}
.avatar {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.info {
  flex: 1;
  min-width: 0;
}
.title {
  font-size: 14px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.sub {
  font-size: 12px;
  color: var(--text-sub);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.tags {
  display: flex;
  gap: 4px;
}
.tag {
  font-size: 11px;
  color: #fff;
  padding: 1px 6px;
  border-radius: 6px;
  max-width: 60px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
