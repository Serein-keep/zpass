<script setup lang="ts">
import { ref, computed } from "vue";
import { NButton, NIcon, NModal, NInput } from "naive-ui";
import { AlbumsOutline } from "@vicons/ionicons5";
import { useAppStore } from "../stores/app";

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{
  (e: "update:show", v: boolean): void;
  (e: "select", templateId: string): void;
  (e: "golibrary"): void;
}>();

const store = useAppStore();
const search = ref("");

const groups = computed(() => {
  const q = search.value.trim().toLowerCase();
  const filtered = store.templates.filter(
    (t) => !q || t.name.toLowerCase().includes(q)
  );
  const map = new Map<string, typeof filtered>();
  for (const t of filtered) {
    const key = t.category_name || "未分类";
    if (!map.has(key)) map.set(key, []);
    map.get(key)!.push(t);
  }
  return Array.from(map.entries());
});

function pick(id: string) {
  emit("select", id);
  emit("update:show", false);
}
</script>

<template>
  <n-modal
    :show="props.show"
    preset="card"
    title="选择模板"
    style="width: 480px"
    @update:show="emit('update:show', $event)"
  >
    <div class="picker">
      <n-input
        v-model:value="search"
        placeholder="搜索模板…"
        clearable
        class="pick-search"
      />
      <div v-if="store.templates.length === 0" class="empty">
        <p>暂无模板，请先创建模板</p>
        <n-button
          type="primary"
          size="small"
          @click="
            emit('golibrary');
            emit('update:show', false);
          "
        >
          前往模板库
        </n-button>
      </div>
      <div v-else-if="groups.length === 0" class="empty">未找到匹配的模板</div>
      <div v-else class="groups">
        <div v-for="[cat, items] in groups" :key="cat" class="group">
          <div class="g-title">{{ cat }}</div>
          <div
            v-for="t in items"
            :key="t.id"
            class="tmpl-item"
            @click="pick(t.id)"
          >
            <n-icon :size="16"><AlbumsOutline /></n-icon>
            <span class="tmpl-name">{{ t.name }}</span>
            <span class="tmpl-count">{{ t.fields.length }} 字段</span>
          </div>
        </div>
      </div>
    </div>
  </n-modal>
</template>

<style scoped>
.picker {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 200px;
  max-height: 420px;
}
.pick-search {
  flex-shrink: 0;
}
.empty {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  color: var(--text-sub);
  font-size: 13px;
  text-align: center;
  padding: 40px 0;
}
.groups {
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.group {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.g-title {
  font-size: 12px;
  color: var(--text-sub);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  padding: 2px 4px;
}
.tmpl-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 12px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 14px;
  color: var(--text);
  transition: background 0.15s;
  border: 1px solid var(--border);
}
.tmpl-item:hover {
  background: var(--panel);
  border-color: var(--primary);
}
.tmpl-name {
  flex: 1;
}
.tmpl-count {
  font-size: 12px;
  color: var(--text-sub);
}
</style>