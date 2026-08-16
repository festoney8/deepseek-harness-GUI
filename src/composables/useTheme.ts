import { computed, ref, watch } from "vue";
import { BaseDirectory, exists, readTextFile, watch as watchFile } from "@tauri-apps/plugin-fs";
import { parseThemePreference, type ThemePreference } from "../utils/parseThemePreference";

export type ResolvedTheme = "light" | "dark";

const SETTINGS_REL = ".dsh/settings.yaml";
const POLL_MS = 3_000;
const preference = ref<ThemePreference>("system");
const systemDark = ref(false);

export const theme = computed<ResolvedTheme>(() =>
  preference.value === "system" ? (systemDark.value ? "dark" : "light") : preference.value,
);

watch(
  [preference, theme],
  ([preferred, resolved]) => {
    const root = document.documentElement;
    root.classList.remove("dark");
    if (preferred === "system") {
      root.removeAttribute("data-theme");
    } else {
      root.dataset.theme = preferred;
    }
    root.style.colorScheme = resolved;
  },
  { immediate: true },
);

let pollId: number | null = null;
let unwatch: (() => void) | null = null;
let media: MediaQueryList | null = null;
let onMediaChange: ((event: MediaQueryListEvent) => void) | null = null;

export function initTheme() {
  if (pollId != null || unwatch != null) return;
  media = window.matchMedia("(prefers-color-scheme: dark)");
  onMediaChange = (event) => (systemDark.value = event.matches);
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

async function tick() {
  if (!(await exists(SETTINGS_REL, { baseDir: BaseDirectory.Home }))) {
    preference.value = "system";
    return;
  }
  stopPolling();
  if (unwatch == null) {
    try {
      unwatch = await watchFile(SETTINGS_REL, () => void applyFileTheme(), {
        baseDir: BaseDirectory.Home,
        delayMs: 100,
      });
    } catch {
      startPolling();
      return;
    }
  }
  await applyFileTheme();
}

async function applyFileTheme() {
  try {
    preference.value = parseThemePreference(await readTextFile(SETTINGS_REL, { baseDir: BaseDirectory.Home }));
  } catch {
    // 配置文件写入过程中读取失败时，等待下一次监听事件。
  }
}
