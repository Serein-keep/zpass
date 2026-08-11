<script setup lang="ts">
import { ref, watch, computed } from "vue";
import {
  NButton,
  NIcon,
  NModal,
  NSlider,
  NSwitch,
  NSelect,
  NInput,
  useMessage,
} from "naive-ui";
import { uiIcon } from "../utils/templateIcons";

const props = defineProps<{ show: boolean }>();
const emit = defineEmits<{ (e: "update:show", v: boolean): void }>();

const message = useMessage();

const CopyIcon = uiIcon("copy");
const RefreshIcon = uiIcon("refresh");
const ChevronDownIcon = uiIcon("chevron-down");
const ChevronForwardIcon = uiIcon("chevron-forward");

const LOWERCASE = "abcdefghijklmnopqrstuvwxyz";
const UPPERCASE = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
const DIGITS = "0123456789";
const SYMBOLS = "!@#$%^&*()-_=+[]{};:,.?/";

const length = ref(16);
const includeUpper = ref(true);
const includeDigits = ref(true);
const includeSymbols = ref(false);
const moreOpen = ref(false);
const upperCount = ref(1);
const digitCount = ref(1);
const symbolCount = ref(1);
const password = ref("");
const symbolMode = ref<"include" | "exclude">("include");
const symbolChars = ref(SYMBOLS);
const symbolModeOptions = [
  { label: "包含", value: "include" },
  { label: "排除", value: "exclude" },
];

const effectiveSymbolPool = computed(() => {
  const unique = (s: string) => Array.from(new Set(s.split(""))).join("");
  let pool = SYMBOLS;
  if (symbolMode.value === "include") {
    if (symbolChars.value.trim()) pool = unique(symbolChars.value);
  } else {
    const excluded = new Set(symbolChars.value.split(""));
    pool = unique(Array.from(SYMBOLS).filter((c) => !excluded.has(c)).join(""));
  }
  return pool || SYMBOLS;
});

const symbolEmpty = computed(() => {
  if (symbolMode.value === "include") return !symbolChars.value.trim();
  const excluded = new Set(symbolChars.value.split(""));
  return Array.from(SYMBOLS).every((c) => excluded.has(c));
});

const activeTypes = computed(() => {
  const types: { key: "upper" | "digits" | "symbols"; label: string; pool: string; count: number }[] = [];
  if (includeUpper.value) types.push({ key: "upper", label: "大写", pool: UPPERCASE, count: upperCount.value });
  if (includeDigits.value) types.push({ key: "digits", label: "数字", pool: DIGITS, count: digitCount.value });
  if (includeSymbols.value) types.push({ key: "symbols", label: "符号", pool: effectiveSymbolPool.value, count: symbolCount.value });
  return types;
});

const overflow = computed(() => {
  const sum = activeTypes.value.reduce((acc, t) => acc + Math.max(1, t.count), 0);
  return sum > length.value;
});

function randFrom(pool: string): string {
  const buf = new Uint32Array(1);
  const max = Math.floor(0x100000000 / pool.length) * pool.length;
  do {
    crypto.getRandomValues(buf);
  } while (buf[0] >= max);
  return pool[buf[0] % pool.length];
}

function shuffle<T>(arr: T[]): T[] {
  const a = [...arr];
  for (let i = a.length - 1; i > 0; i--) {
    const buf = new Uint32Array(1);
    crypto.getRandomValues(buf);
    const j = buf[0] % (i + 1);
    [a[i], a[j]] = [a[j], a[i]];
  }
  return a;
}

function generate() {
  let chars: string[] = [];
  for (const t of activeTypes.value) {
    for (let i = 0; i < Math.max(1, t.count); i++) {
      chars.push(randFrom(t.pool));
    }
  }
  const allPool = [LOWERCASE, ...activeTypes.value.map((t) => t.pool)].join("");
  if (chars.length > length.value) {
    chars = shuffle(chars).slice(0, length.value);
  }
  while (chars.length < length.value) {
    chars.push(randFrom(allPool));
  }
  password.value = shuffle(chars).join("");
}

watch(
  [length, includeUpper, includeDigits, includeSymbols, upperCount, digitCount, symbolCount, symbolMode, symbolChars],
  generate,
  { immediate: true }
);

function copyPassword() {
  if (navigator.clipboard) {
    navigator.clipboard.writeText(password.value);
    message.success("已复制");
  }
}

function onCountChange(key: "upper" | "digits" | "symbols", v: number) {
  if (key === "upper") upperCount.value = v;
  else if (key === "digits") digitCount.value = v;
  else symbolCount.value = v;
}

watch(
  () => props.show,
  (v) => {
    if (v) moreOpen.value = false;
  }
);
</script>

<template>
  <n-modal
    :show="props.show"
    preset="card"
    title="密码生成器"
    style="width: 460px"
    @update:show="emit('update:show', $event)"
  >
    <div class="pw-display">
      <div class="pw-value" @click="copyPassword">{{ password }}</div>
      <div class="pw-actions">
        <n-button text size="tiny" @click="copyPassword">
          <template #icon><n-icon :size="16"><CopyIcon /></n-icon></template>
        </n-button>
        <n-button text size="tiny" @click="generate">
          <template #icon><n-icon :size="16"><RefreshIcon /></n-icon></template>
        </n-button>
      </div>
    </div>

    <div class="option-row">
      <div class="opt-label">长度 <span class="opt-value">{{ length }}</span></div>
      <n-slider v-model:value="length" :min="4" :max="100" />
    </div>

    <div class="section-title">包括</div>
    <div class="switch-row">
      <span>大写</span>
      <n-switch v-model:value="includeUpper" />
    </div>
    <div class="switch-row">
      <span>数字</span>
      <n-switch v-model:value="includeDigits" />
    </div>
    <div class="switch-row">
      <span>符号</span>
      <n-switch v-model:value="includeSymbols" />
    </div>

    <div class="more-toggle" @click="moreOpen = !moreOpen">
      <span>更多选项</span>
      <n-icon :size="14">
        <component :is="moreOpen ? ChevronDownIcon : ChevronForwardIcon" />
      </n-icon>
    </div>

    <div class="more-body" v-if="moreOpen">
      <div class="count-row" v-for="t in activeTypes" :key="t.key">
        <div class="opt-label">{{ t.label }} <span class="opt-value">{{ t.key === 'upper' ? upperCount : t.key === 'digits' ? digitCount : symbolCount }}</span></div>
        <n-slider
          :value=" t.key === 'upper' ? upperCount : t.key === 'digits' ? digitCount : symbolCount"
          @update:value="(v) => onCountChange(t.key, v)"
          :min="1"
          :max="length"
        />
      </div>
      <div class="symbol-config" v-if="includeSymbols">
        <div class="opt-label">符号字符</div>
        <div class="symbol-input-row">
          <n-select
            v-model:value="symbolMode"
            :options="symbolModeOptions"
            size="small"
            style="width: 110px"
          />
          <n-input
            v-model:value="symbolChars"
            placeholder="如：!@#$%^&*"
            size="small"
          />
        </div>
        <div class="overflow-hint" v-if="symbolEmpty">符号字符集为空，已使用默认符号</div>
      </div>
      <div class="overflow-hint" v-if="overflow">各字符位数之和超过密码长度，将按长度截断</div>
    </div>
  </n-modal>
</template>

<style scoped>
.pw-display {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px;
  background: var(--panel);
  border: 1px solid var(--border);
  border-radius: 8px;
  margin-bottom: 16px;
}
.pw-value {
  flex: 1;
  font-family: monospace;
  font-size: 20px;
  letter-spacing: 1px;
  color: var(--primary);
  word-break: break-all;
  cursor: pointer;
  line-height: 1.4;
}
.pw-actions {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}
.option-row {
  margin-bottom: 12px;
}
.opt-label {
  font-size: 13px;
  color: var(--text-sub);
  margin-bottom: 6px;
}
.opt-value {
  font-weight: 600;
  color: var(--text);
  font-family: monospace;
}
.section-title {
  font-size: 13px;
  color: var(--text-sub);
  font-weight: 600;
  margin: 12px 0 8px;
}
.switch-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 14px;
  padding: 5px 0;
}
.more-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: var(--text-sub);
  cursor: pointer;
  margin-top: 12px;
  padding: 6px 0;
  user-select: none;
}
.more-toggle:hover {
  color: var(--text);
}
.more-body {
  border-top: 1px dashed var(--border);
  padding: 10px 0 4px;
}
.count-row {
  padding: 6px 0;
}
.count-row .opt-label {
  margin-bottom: 6px;
}
.symbol-config {
  padding: 8px 0 4px;
  border-top: 1px dashed var(--border);
  margin-top: 6px;
}
.symbol-input-row {
  display: flex;
  gap: 8px;
}
.symbol-input-row .n-input {
  flex: 1;
}
.overflow-hint {
  font-size: 12px;
  color: var(--text-sub);
  margin-top: 8px;
}
</style>