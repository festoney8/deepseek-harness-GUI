import { computed, ref, watch } from "vue";
import { BaseDirectory, exists, readTextFile, watch as watchFile } from "@tauri-apps/plugin-fs";
import { parseThemePreference, type ThemePreference } from "../utils/parseThemePreference";

export type ResolvedTheme = "light" | "dark";

const SETTINGS_REL = ".dsh/settings.yaml";
const POLL_MS = 3_000;

/** 配置文件偏好；文件缺失时为 system */
const preference = ref<ThemePreference>("system");
/** 系统深浅色，仅由 matchMedia 更新 */
const systemDark = ref(false);

export const theme = computed<ResolvedTheme>(() =>
  preference.value === "system" ? (systemDark.value ? "dark" : "light") : preference.value,
);

watch(
  theme,
  (t) => {
    document.documentElement.classList.toggle("dark", t === "dark");
    document.documentElement.style.colorScheme = t;
  },
  { immediate: true },
);

let pollId: number | null = null;
let unwatch: (() => void) | null = null;
let media: MediaQueryList | null = null;
let onMediaChange: ((e: MediaQueryListEvent) => void) | null = null;

export function initTheme() {
  if (pollId != null || unwatch != null) return;
  media = window.matchMedia("(prefers-color-scheme: dark)");
  onMediaChange = (e) => (systemDark.value = e.matches);
  media.addEventListener("change", onMediaChange);
  systemDark.value = media.matches;
  startPolling();
  void tick();
}

export function disposeTheme() {
  stopPolling();
  unwatch?.();
  unwatch = null;
  if (media && onMediaChange) {
    media.removeEventListener("change", onMediaChange);
    media = null;
    onMediaChange = null;
  }
}

function startPolling() {
  if (pollId == null) pollId = window.setInterval(() => void tick(), POLL_MS);
}

function stopPolling() {
  if (pollId != null) {
    clearInterval(pollId);
    pollId = null;
  }
}

/** 轮询与监听共用的入口：文件出现则停轮询并转监听，否则保持 system */
async function tick() {
  if (!(await exists(SETTINGS_REL, { baseDir: BaseDirectory.Home }))) {
    preference.value = "system";
    return;
  }
  stopPolling();
  if (unwatch == null) {
    try {
      // 防抖 watch
      unwatch = await watchFile(SETTINGS_REL, () => void applyFileTheme(), {
        baseDir: BaseDirectory.Home,
        delayMs: 100,
      });
    } catch {
      startPolling(); // 监听建立失败则退回轮询
      return;
    }
  }
  // 监听建立后读取
  await applyFileTheme();
}

async function applyFileTheme() {
  try {
    preference.value = parseThemePreference(await readTextFile(SETTINGS_REL, { baseDir: BaseDirectory.Home }));
  } catch {
    // 文件写入中途读取失败：保留上次值，等下一次事件
  }
}
