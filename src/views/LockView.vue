<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from "vue";
import { useRouter } from "vue-router";
import { useAppStore } from "../stores/app";
import { open } from "@tauri-apps/plugin-dialog";
import { emit } from "@tauri-apps/api/event";
import { api } from "../api";
import { NCard, NInput, NButton, NForm, NFormItem, NSpin, useMessage } from "naive-ui";

const router = useRouter();
const store = useAppStore();
const message = useMessage();

// 等待 store.init() 完成后再渲染表单，避免“新建主密码”界面一闪而过
const ready = ref(false);
const isSetup = computed(() => !store.hasMaster);
const mode = ref<"new" | "import">("new");
const password = ref("");
const confirm = ref("");
const importPassword = ref("");
const loading = ref(false);
const passwordRef = ref<InstanceType<typeof NInput> | null>(null);

async function focusInput() {
  await nextTick();
  passwordRef.value?.focus();
}

onMounted(async () => {
  try {
    await store.init();
  } finally {
    ready.value = true;
  }
  // 锁屏界面已渲染，通知后端显示窗口（窗口以 visible:false 启动）
  await nextTick();
  try {
    await emit("app-ready");
  } catch {
    // 浏览器调试环境无 Tauri 事件系统，忽略
  }
  if (!store.locked) {
    router.replace("/main");
    return;
  }
  focusInput();
  window.addEventListener("focus", onFocus);
});

onUnmounted(() => {
  window.removeEventListener("focus", onFocus);
});

function onFocus() {
  if (store.locked) focusInput();
}

async function submit() {
  if (mode.value === "new") {
    if (password.value.length < 6) {
      message.warning("主密码至少 6 位");
      return;
    }
    if (password.value !== confirm.value) {
      message.error("两次输入不一致");
      return;
    }
    loading.value = true;
    try {
      await store.setMasterPassword(password.value);
      message.success("主密码设置成功");
      password.value = "";
      confirm.value = "";
      router.replace("/main");
    } catch (e: any) {
      message.error(e?.toString() || "操作失败");
    } finally {
      loading.value = false;
    }
  } else {
    if (!importPassword.value) {
      message.warning("请输入旧数据的主密码");
      return;
    }
    const file = await open({
      title: "选择导入文件",
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!file || typeof file !== "string") return;

    loading.value = true;
    try {
      const result = await api.importData(file, importPassword.value);
      message.success(`导入成功，共 ${result.imported} 条记录`);
      store.locked = false;
      await store.refreshAll();
      router.replace("/main");
    } catch (e: any) {
      message.error(e?.toString() || "导入失败");
    } finally {
      loading.value = false;
    }
  }
}

async function unlock() {
  if (password.value.length < 6) {
    message.warning("主密码至少 6 位");
    return;
  }
  loading.value = true;
  try {
    await store.unlock(password.value);
    password.value = "";
    router.replace("/main");
  } catch (e: any) {
    message.error(e?.toString() || "解锁失败");
  } finally {
    loading.value = false;
  }
}
</script>

<template>
  <div class="lock-wrap">
    <n-card class="lock-card" :bordered="false">
      <div class="logo">ZPass</div>

      <div v-if="!ready" class="loading">
        <n-spin size="small" />
      </div>

      <template v-else-if="isSetup">
        <div class="mode-switch">
          <n-button
            :type="mode === 'new' ? 'primary' : 'default'"
            @click="mode = 'new'"
            size="small"
          >
            新建主密码
          </n-button>
          <n-button
            :type="mode === 'import' ? 'primary' : 'default'"
            @click="mode = 'import'"
            size="small"
          >
            导入旧数据
          </n-button>
        </div>

        <n-form @submit.prevent="submit" v-if="mode === 'new'">
          <div class="subtitle">首次使用，请设置主密码</div>
          <n-form-item label="主密码（设置后不可更改）">
            <n-input
              ref="passwordRef"
              v-model:value="password"
              type="password"
              show-password-on="click"
              placeholder="至少 6 位"
            />
          </n-form-item>
          <n-form-item label="确认主密码">
            <n-input
              v-model:value="confirm"
              type="password"
              show-password-on="click"
              placeholder="再次输入"
            />
          </n-form-item>
          <n-button
            type="primary"
            block
            :loading="loading"
            attr-type="submit"
          >
            设置并进入
          </n-button>
          <div class="hint">
            主密码用于加密所有数据并解锁应用，丢失将无法恢复数据。
          </div>
        </n-form>

        <n-form @submit.prevent="submit" v-else>
          <div class="subtitle">从导出文件恢复数据</div>
          <n-form-item label="旧数据的主密码">
            <n-input
              ref="passwordRef"
              v-model:value="importPassword"
              type="password"
              show-password-on="click"
              placeholder="输入旧数据的主密码"
            />
          </n-form-item>
          <n-button
            type="primary"
            block
            :loading="loading"
            attr-type="submit"
          >
            选择文件并导入
          </n-button>
          <div class="hint">
            导入后旧数据的主密码将成为当前主密码。
          </div>
        </n-form>
      </template>

      <template v-else>
        <div class="subtitle">输入主密码解锁</div>
        <n-form @submit.prevent="unlock">
          <n-input
            ref="passwordRef"
            v-model:value="password"
            type="password"
            show-password-on="click"
            placeholder="输入主密码"
          />
          <n-button
            type="primary"
            block
            :loading="loading"
            attr-type="submit"
            style="margin-top: 16px"
          >
            解锁
          </n-button>
        </n-form>
      </template>
    </n-card>
  </div>
</template>

<style scoped>
.lock-wrap {
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #1f2937, #111627);
}
.lock-card {
  width: 360px;
  border-radius: 16px;
  padding: 12px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.3);
}
.logo {
  text-align: center;
  font-size: 32px;
  font-weight: 700;
  color: var(--primary);
  margin-bottom: 4px;
}
.loading {
  display: flex;
  justify-content: center;
  padding: 32px 0;
}
.mode-switch {
  display: flex;
  gap: 6px;
  justify-content: center;
  margin-bottom: 16px;
}
.subtitle {
  text-align: center;
  color: var(--text-sub);
  margin-bottom: 20px;
}
.hint {
  margin-top: 16px;
  font-size: 12px;
  color: var(--text-sub);
  text-align: center;
}
</style>
