<script setup lang="ts">
import { ref, computed, watch, h } from "vue";
import type { TreeOption } from "naive-ui";
import { NButton, NIcon, NModal, NInput, NTree } from "naive-ui";
import { templateIcon, uiIcon } from "../utils/templateIcons";
import { useAppStore } from "../stores/app";

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{
  (e: "update:show", v: boolean): void;
  (e: "select", templateId: string): void;
  (e: "golibrary"): void;
}>();

const store = useAppStore();
const search = ref("");
const ChevronIcon = uiIcon("chevron-down");

interface PickNode extends TreeOption {
  isTemplate: boolean;
  templateId?: string;
  fieldCount?: number;
  templateCount?: number;
  iconKey?: string | null;
  color?: string | null;
}

const q = computed(() => search.value.trim().toLowerCase());
const filteredTemplates = computed(() =>
  q.value
    ? store.templates.filter((t) => t.name.toLowerCase().includes(q.value))
    : store.templates
);

function catColor(catId: string): string | null {
  return store.categories.find((c) => c.id === catId)?.color ?? null;
}

function templatesOf(catId: string): PickNode[] {
  const color = catColor(catId);
  return filteredTemplates.value
    .filter((t) => t.category_id === catId)
    .map((t) => ({
      key: "tmpl-" + t.id,
      label: t.name,
      isTemplate: true,
      templateId: t.id,
      fieldCount: t.fields.length,
      iconKey: t.icon,
      color,
    }));
}

function buildCats(parentId: string | null): PickNode[] {
  const out: PickNode[] = [];
  for (const c of store.categories.filter((x) => (x.parent_id ?? null) === parentId)) {
    const children = [...buildCats(c.id), ...templatesOf(c.id)];
    if (children.length === 0) continue;
    out.push({
      key: "cat-" + c.id,
      label: c.name,
      isTemplate: false,
      iconKey: c.icon,
      color: c.color,
      templateCount: children.filter((n) => n.isTemplate).length,
      children,
    });
  }
  return out;
}

const treeData = computed(() => buildCats(null));

const expandedKeys = ref<string[]>([]);
watch(
  treeData,
  (nodes) => {
    const keys: string[] = [];
    const walk = (list: PickNode[]) => {
      for (const n of list) {
        if (n.children?.length) {
          keys.push(n.key as string);
          walk(n.children as PickNode[]);
        }
      }
    };
    walk(nodes);
    expandedKeys.value = keys;
  },
  { immediate: true }
);

function renderSwitcher({ expanded }: { expanded: boolean }) {
  return h(ChevronIcon, {
    style: {
      transform: expanded ? "rotate(-90deg)" : "rotate(0deg)",
      transition: "transform 0.2s",
      color: "var(--text-sub)",
    },
  });
}

function renderLabel({ option }: { option: TreeOption }) {
  const o = option as PickNode;
  return h(
    "span",
    { class: o.isTemplate ? "pick-node tmpl" : "pick-node cat" },
    [
      h(
        NIcon,
        {
          size: o.isTemplate ? 15 : 17,
          color: o.color ?? "var(--text-sub)",
        },
        { default: () => h(templateIcon(o.iconKey)) }
      ),
      h("span", { class: "pick-label" }, o.label),
    ]
  );
}

function renderSuffix({ option }: { option: TreeOption }) {
  const o = option as PickNode;
  if (o.isTemplate) {
    return h("span", { class: "badge" }, `${o.fieldCount} 字段`);
  }
  return h("span", { class: "count" }, `${o.templateCount}`);
}

function onSelect(keys: Array<string | number>) {
  const key = keys[0];
  if (typeof key !== "string" || !key.startsWith("tmpl-")) return;
  emit("select", key.slice(5));
  emit("update:show", false);
}
</script>

<template>
  <n-modal
    :show="props.show"
    preset="card"
    title="选择模板"
    style="width: 460px"
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
      <div v-else-if="treeData.length === 0" class="empty">
        未找到匹配的模板
      </div>
      <div v-else class="tree-wrap">
        <n-tree
          class="pick-tree"
          :data="treeData"
          :expanded-keys="expandedKeys"
          :indent="18"
          :render-switcher-icon="renderSwitcher"
          :render-label="renderLabel"
          :render-suffix="renderSuffix"
          :node-props="() => ({ class: 'pick-node-row' })"
          @update:expanded-keys="(k) => (expandedKeys = k as string[])"
          @update:selected-keys="onSelect"
        />
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
.tree-wrap {
  overflow-y: auto;
  flex: 1;
  min-height: 0;
  padding: 6px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--panel);
}

/*
 * render-label / render-suffix 返回的元素由 naive-ui NTree 内部创建，
 * 不带本组件 scoped 属性，必须通过 :deep() 从模板根元素 .pick-tree 穿透选择。
 */
.pick-tree :deep(.n-tree-node) {
  border-radius: 8px;
}
.pick-tree :deep(.n-tree-node-content) {
  padding: 0 6px;
  min-height: 32px;
  display: flex;
  width: 100%;
}
.pick-tree :deep(.n-tree-node-content__text) {
  display: flex;
  align-items: center;
  flex-grow: 1;
  min-width: 0;
}
.pick-tree :deep(.n-tree-node-content__suffix) {
  display: inline-flex;
  align-items: center;
  margin-left: auto;
  padding-left: 8px;
}
.pick-tree :deep(.n-tree-node-wrapper) {
  padding: 1px 0;
}
.pick-tree :deep(.n-tree-node-switcher) {
  width: 20px;
  height: 20px;
  margin-right: 4px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}
.pick-tree :deep(.n-tree-node-switcher svg) {
  width: 14px;
  height: 14px;
  display: block;
}
.pick-tree :deep(.n-tree-node:not(.n-tree-node--selected):hover) {
  background: var(--sidebar);
}

.pick-tree :deep(.pick-node) {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  line-height: 1;
}
.pick-tree :deep(.pick-node svg) {
  display: block;
  flex-shrink: 0;
}
.pick-tree :deep(.pick-node.cat) {
  font-weight: 600;
  color: var(--text);
}
.pick-tree :deep(.pick-node.tmpl) {
  font-weight: 400;
  color: var(--text);
}
.pick-tree :deep(.pick-label) {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.pick-tree :deep(.badge) {
  font-size: 11px;
  color: var(--text-sub);
  opacity: 0.75;
  background: var(--sidebar);
  border: 1px solid var(--border);
  border-radius: 20px;
  padding: 2px 8px;
  white-space: nowrap;
  line-height: 1.2;
}
.pick-tree :deep(.count) {
  font-size: 12px;
  color: var(--text-sub);
  opacity: 0.75;
  background: var(--sidebar);
  border-radius: 20px;
  padding: 2px 8px;
  white-space: nowrap;
  line-height: 1.2;
}
</style>
