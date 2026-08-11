<script setup lang="ts">
import { computed } from "vue";
import { useAppStore } from "../stores/app";
import { NIcon } from "naive-ui";
import { templateIcon, uiIcon } from "../utils/templateIcons";

const emit = defineEmits<{
  (e: "category", cat: string | null): void;
  (e: "tag", tag: string | null): void;
  (e: "view", v: "all" | "trash"): void;
}>();

const store = useAppStore();
const TrashIcon = uiIcon("trash");

const totalActive = computed(() => store.activeEntries.length);
const trashCount = computed(() => store.trashedEntries.length);

function statCount(cat: string): number {
  return store.categoryStats.find((s) => s.category === cat)?.count || 0;
}

const categoryEntries = computed(() =>
  store.categories.map((c) => ({
    key: c.id,
    label: c.name,
    icon: templateIcon(c.icon),
    color: c.color || "#999",
    count: statCount(c.id),
    share: totalActive.value > 0 ? statCount(c.id) / totalActive.value : 0,
  }))
);

const hasTagStats = computed(() => store.tagStats.length > 0);
</script>

<template>
  <div class="overview">
    <div class="section-title">
      <n-icon :size="18" color="var(--primary)"><component :is="templateIcon('albums')" /></n-icon>
      <span>密码概览</span>
    </div>

    <div class="stat-cards">
      <div class="stat-card">
        <div class="stat-num">{{ totalActive }}</div>
        <div class="stat-label">全部条目</div>
      </div>
      <div class="stat-card clickable" @click="emit('view', 'trash')">
        <div class="stat-num">{{ trashCount }}</div>
        <div class="stat-label">
          <n-icon :size="14"><TrashIcon /></n-icon>
          回收站
        </div>
      </div>
      <div class="stat-card">
        <div class="stat-num">{{ store.tags.length }}</div>
        <div class="stat-label">标签</div>
      </div>
    </div>

    <div class="section-title sub">分类统计</div>
    <div
      v-for="item in categoryEntries"
      :key="item.key"
      class="stat-row"
      @click="emit('category', item.key)"
    >
      <n-icon :size="18" :color="item.color"><component :is="item.icon" /></n-icon>
      <span class="stat-row-label">{{ item.label }}</span>
      <span class="stat-row-bar">
        <span class="bar-fill" :style="{ width: item.share * 100 + '%' }" />
      </span>
      <span class="stat-row-count">{{ item.count }}</span>
    </div>

    <template v-if="hasTagStats">
      <div class="section-title sub">标签</div>
      <div
        v-for="t in store.tagStats"
        :key="t.id"
        class="stat-row"
        @click="emit('tag', t.name)"
      >
        <span class="dot" :style="{ background: t.color || '#999' }" />
        <span class="stat-row-label">{{ t.name }}</span>
        <span class="stat-row-count">{{ t.count }}</span>
      </div>
    </template>
    <div v-else class="empty-tip">暂无标签</div>

    <div class="hint">点击统计项可筛选列表</div>
  </div>
</template>

<style scoped>
.overview {
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow-y: auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.section-title {
  font-size: 15px;
  font-weight: 600;
  display: flex;
  align-items: center;
  gap: 8px;
}
.section-title.sub {
  font-size: 13px;
  color: var(--text-sub);
  margin-top: 4px;
}
.stat-cards {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 10px;
}
.stat-card {
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 14px 12px;
  text-align: center;
}
.stat-card.clickable {
  cursor: pointer;
  transition: background 0.15s;
}
.stat-card.clickable:hover {
  background: var(--bg);
}
.stat-num {
  font-size: 22px;
  font-weight: 700;
  color: var(--text);
}
.stat-label {
  font-size: 12px;
  color: var(--text-sub);
  margin-top: 4px;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
}
.stat-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  transition: background 0.15s;
}
.stat-row:hover {
  background: var(--panel);
}
.stat-row-label {
  flex: 1;
  font-size: 14px;
}
.stat-row-bar {
  width: 60px;
  height: 4px;
  background: var(--border);
  border-radius: 2px;
  overflow: hidden;
}
.bar-fill {
  display: block;
  height: 100%;
  background: var(--primary);
  border-radius: 2px;
  transition: width 0.3s;
}
.stat-row-count {
  font-size: 13px;
  color: var(--text-sub);
  min-width: 20px;
  text-align: right;
}
.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}
.empty-tip {
  font-size: 12px;
  color: var(--text-sub);
  padding: 4px 10px;
}
.hint {
  font-size: 12px;
  color: var(--text-sub);
  text-align: center;
  margin-top: auto;
}
</style>
