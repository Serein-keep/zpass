<script setup lang="ts">
import { computed, onMounted, onUnmounted } from "vue";
import { useRoute, useRouter } from "vue-router";
import { NConfigProvider, NMessageProvider, NDialogProvider, darkTheme } from "naive-ui";
import { useAppStore } from "./stores/app";
import { listen } from "@tauri-apps/api/event";
import { api } from "./api";

const route = useRoute();
const router = useRouter();
const store = useAppStore();

// 锁屏路由（含首帧路由未就绪时）铺深色背景，避免启动白屏；
// 进入主界面/设置后恢复浅色，不影响 var(--bg) 体系
const isBootScreen = computed(
  () => !router.isReady() || route.name === "lock"
);

const naiveTheme = computed(() =>
  store.settings.theme === "dark" ? darkTheme : null
);

const themeOverrides = computed(() =>
  store.settings.theme === "dark"
    ? {
        common: {
          primaryColor: "#7c9cff",
          primaryColorHover: "#94b0ff",
          primaryColorPressed: "#6c8cff",
          primaryColorSuppl: "#94b0ff",
        },
      }
    : {
        common: {
          primaryColor: "#3b5bdb",
          primaryColorHover: "#4c6ef5",
          primaryColorPressed: "#364fc7",
          primaryColorSuppl: "#4c6ef5",
        },
      }
);

// 监听后端自动锁屏事件
let unlisten: (() => void) | null = null;

async function setupLockListener() {
  unlisten = await listen("app-locked", () => {
    store.lock();
    router.push("/lock");
  });
}

// 用户活动心跳
function onActivity() {
  if (!store.locked) {
    api.heartbeat();
  }
}

onMounted(async () => {
  await setupLockListener();
  window.addEventListener("mousemove", onActivity);
  window.addEventListener("keydown", onActivity);
  window.addEventListener("click", onActivity);
});

onUnmounted(() => {
  unlisten?.();
  window.removeEventListener("mousemove", onActivity);
  window.removeEventListener("keydown", onActivity);
  window.removeEventListener("click", onActivity);
});
</script>

<template>
  <n-config-provider :theme="naiveTheme" :theme-overrides="themeOverrides">
    <n-message-provider>
      <n-dialog-provider>
        <div class="shell" :class="{ boot: isBootScreen }">
          <router-view />
        </div>
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>

<style>
/* 打通百分比高度链：#app(100vh) → config/message/dialog provider → 各视图 height:100% */
.n-config-provider,
.n-message-provider,
.n-dialog-provider,
.shell {
  height: 100%;
}
/* 挂载后由 shell 接管背景：默认跟随主题 var(--bg)（覆盖 index.html 中 #app 的启动深色背景） */
.shell {
  background: var(--bg);
}
/* 锁屏期间铺深色背景，与锁屏页渐变衔接，防止浅色/白色透出 */
.shell.boot {
  background-color: #1f2937;
}
</style>
