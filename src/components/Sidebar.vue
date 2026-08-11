<script setup lang="ts">
import { computed, h } from "vue";
import { useAppStore } from "../stores/app";
import { NIcon, NDropdown, useMessage } from "naive-ui";
import { templateIcon, uiIcon } from "../utils/templateIcons";
import type { Category } from "../types";

const props = defineProps<{
  filterCategory: string | null;
  filterTag: string | null;
  view: "all" | "trash" | "otp" | "templates";
}>();
const emit = defineEmits<{
  (e: "category", cat: string | null): void;
  (e: "tag", tag: string | null): void;
  (e: "view", v: "all" | "trash" | "otp" | "templates"): void;
  (e: "managecategory", cat: Category): void;
  (e: "createcategory", parent: Category | null): void;
  (e: "createtag"): void;
  (e: "managetag", tag: { id: string; name: string; color?: string | null }): void;
}>();

const store = useAppStore();
const message = useMessage();

const ListIcon = uiIcon("list");
const TrashIcon = uiIcon("trash");
const KeyIcon = uiIcon("key");
const AlbumsIcon = uiIcon("albums");
const EllipsisIcon = uiIcon("ellipsis-vertical");
const AddIcon = uiIcon("add");
const CreateIcon = uiIcon("create");

const total = computed(() => store.activeEntries.length);
const trashCount = computed(() => store.trashedEntries.length);
const otpCount = computed(() => store.otpEntries.length);

function statCount(cat: string): number {
  const direct = store.categoryStats.find((s) => s.category === cat)?.count || 0;
  const childSum = store
    .childCategories(cat)
    .reduce((acc, c) => acc + statCount(c.id), 0);
  return direct + childSum;
}
function isCatActive(cat: string) {
  return props.view === "all" && props.filterCategory === cat;
}
function isTagActive(tag: string) {
  return props.view === "all" && props.filterTag === tag;
}

const catMenu = () => [
  {
    label: "新建子类",
    key: "add-child",
    icon: () => h(NIcon, null, { default: () => h(AddIcon) }),
  },
  {
    label: "编辑",
    key: "edit",
    icon: () => h(NIcon, null, { default: () => h(CreateIcon) }),
  },
  {
    label: "删除",
    key: "delete",
    icon: () => h(NIcon, null, { default: () => h(TrashIcon) }),
  },
];

function onCatMenu(key: string, cat: Category) {
  if (key === "add-child") {
    emit("createcategory", cat);
  } else if (key === "edit") {
    emit("managecategory", cat);
  } else if (key === "delete") {
    removeCategory(cat);
  }
}

async function removeCategory(cat: Category) {
  try {
    await store.deleteCategory(cat.id);
    message.success("已删除类别");
    if (props.filterCategory === cat.id) emit("category", null);
  } catch (e: any) {
    message.error(e?.toString() || "删除类别失败");
  }
}

const tagMenu = () => [
  {
    label: "编辑",
    key: "edit",
    icon: () => h(NIcon, null, { default: () => h(CreateIcon) }),
  },
  {
    label: "删除",
    key: "delete",
    icon: () => h(NIcon, null, { default: () => h(TrashIcon) }),
  },
];

function onTagMenu(key: string, tag: { id: string; name: string; color?: string | null }) {
  if (key === "edit") {
    emit("managetag", tag);
  } else if (key === "delete") {
    removeTag(tag);
  }
}

async function removeTag(tag: { id: string; name: string }) {
  try {
    await store.deleteTag(tag.name);
    message.success("已删除标签");
    if (props.filterTag === tag.name) emit("tag", null);
  } catch (e: any) {
    message.error(e?.toString() || "删除标签失败");
  }
}
</script>

<template>
  <div class="sidebar">
    <div class="brand">ZPass</div>

    <div class="section">
      <div
        class="item"
        :class="{ active: view === 'all' && !filterCategory && !filterTag }"
        @click="emit('view', 'all')"
      >
        <n-icon :size="16"><ListIcon /></n-icon>
        <span class="label">全部条目</span>
        <span class="count">{{ total }}</span>
      </div>

      <div class="group-title">
        类别
        <span class="group-add" @click="emit('createcategory', null)">
          <n-icon :size="14"><AddIcon /></n-icon>
        </span>
      </div>

      <div
        v-for="cat in store.rootCategories"
        :key="cat.id"
        class="cat-block"
      >
        <div
          class="item cat-item"
          :class="{ active: isCatActive(cat.id) }"
          @click="emit('category', cat.id)"
        >
          <n-icon :size="16" :color="cat.color || undefined">
            <component :is="templateIcon(cat.icon)" />
          </n-icon>
          <span class="label">{{ cat.name }}</span>
          <span class="count">{{ statCount(cat.id) }}</span>
          <n-dropdown
            :options="catMenu()"
            trigger="hover"
            @select="(k) => onCatMenu(k, cat)"
          >
            <button type="button" class="cat-more" @click.stop>
              <n-icon :size="14"><EllipsisIcon /></n-icon>
            </button>
          </n-dropdown>
        </div>

        <div
          v-for="child in store.childCategories(cat.id)"
          :key="child.id"
          class="item cat-item child"
          :class="{ active: isCatActive(child.id) }"
          @click="emit('category', child.id)"
        >
          <n-icon :size="15" :color="child.color || undefined">
            <component :is="templateIcon(child.icon)" />
          </n-icon>
          <span class="label">{{ child.name }}</span>
          <span class="count">{{ statCount(child.id) }}</span>
          <n-dropdown
            :options="catMenu()"
            trigger="hover"
            @select="(k) => onCatMenu(k, child)"
          >
            <button type="button" class="cat-more" @click.stop>
              <n-icon :size="14"><EllipsisIcon /></n-icon>
            </button>
          </n-dropdown>
        </div>
      </div>

    </div>

    <div class="section">
      <div class="group-title">
        标签
        <span class="group-add" @click="emit('createtag')">
          <n-icon :size="14"><AddIcon /></n-icon>
        </span>
      </div>
      <div v-if="store.tagStats.length === 0" class="empty-tip">暂无标签</div>
      <div
        v-for="t in store.tagStats"
        :key="t.id"
        class="item tag-item"
        :class="{ active: isTagActive(t.name) }"
        @click="emit('tag', t.name)"
      >
        <span class="dot" :style="{ background: t.color || '#999' }" />
        <span class="label">{{ t.name }}</span>
        <span class="count">{{ t.count }}</span>
        <n-dropdown
          :options="tagMenu()"
          trigger="hover"
          @select="(k) => onTagMenu(k, t)"
        >
          <button type="button" class="cat-more" @click.stop>
            <n-icon :size="14"><EllipsisIcon /></n-icon>
          </button>
        </n-dropdown>
      </div>
    </div>

    <div class="section">
      <div class="group-title">其它</div>
      <div
        class="item otp"
        :class="{ active: view === 'otp' }"
        @click="emit('view', 'otp')"
      >
        <n-icon :size="16"><KeyIcon /></n-icon>
        <span class="label">OTP 验证码</span>
        <span class="count" v-if="otpCount">{{ otpCount }}</span>
      </div>
      <div
        class="item templates"
        :class="{ active: view === 'templates' }"
        @click="emit('view', 'templates')"
      >
        <n-icon :size="16"><AlbumsIcon /></n-icon>
        <span class="label">模板库</span>
        <span class="count" v-if="store.templates.length">{{ store.templates.length }}</span>
      </div>
      <div
        class="item trash"
        :class="{ active: view === 'trash' }"
        @click="emit('view', 'trash')"
      >
        <n-icon :size="16"><TrashIcon /></n-icon>
        <span class="label">回收站</span>
        <span class="count" v-if="trashCount">{{ trashCount }}</span>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sidebar {
  padding: 16px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  height: 100%;
  overflow-y: auto;
}
.brand {
  font-size: 18px;
  font-weight: 700;
  color: var(--primary);
  padding: 4px 8px 12px;
}
.section {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.group-title {
  font-size: 12px;
  color: var(--text-sub);
  padding: 10px 8px 4px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.group-add {
  color: var(--text-sub);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2px 4px;
  border-radius: 4px;
  transition: color 0.15s, background 0.15s;
}
.group-add:hover {
  color: var(--primary);
  background: var(--panel);
}
.item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 14px;
  color: var(--text);
  transition: background 0.15s;
}
.item:hover {
  background: var(--panel);
}
.item.active {
  background: var(--primary);
  color: #fff;
}
.item.active .count {
  color: #fff;
  opacity: 0.85;
}
.cat-item {
  padding-right: 6px;
}
.cat-item.child {
  padding-left: 26px;
  font-size: 13px;
}
.cat-more {
  flex-shrink: 0;
  border: none;
  background: transparent;
  padding: 4px;
  border-radius: 6px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-sub);
  opacity: 0.75;
  cursor: pointer;
  transition: opacity 0.15s, background 0.15s, color 0.15s;
}
.cat-item:hover .cat-more {
  opacity: 1;
}
.cat-more:hover {
  background: var(--bg);
  color: var(--primary);
}
.tag-item {
  padding-right: 6px;
}
.label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.count {
  font-size: 12px;
  color: var(--text-sub);
  background: var(--bg);
  border-radius: 10px;
  padding: 1px 8px;
  flex-shrink: 0;
}
.item.active .count {
  background: rgba(255, 255, 255, 0.2);
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
.otp {
  margin-top: 4px;
}
.trash {
  margin-top: 4px;
}
</style>