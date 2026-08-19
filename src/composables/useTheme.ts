import { readonly, ref } from "vue";
import { BaseDirectory, exists, readTextFile, watch, type UnwatchFn } from "@tauri-apps/plugin-fs";
import { load } from "js-yaml";

export type ThemePreference = "light" | "dark" | "system";

const THEME_PATH = ".dsh/settings.yaml";
const HOME_OPTIONS = { baseDir: BaseDirectory.Home };
const POLL_INTERVAL_MS = 3_000;
const WATCH_DELAY_MS = 100;

interface ThemeSettings {
  "ui-theme"?: { preference?: unknown };
}

const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

/** 解析 ui-theme.preference；缺失、类型错误或值不受支持时回退 system（DESIGN.md §9.2）。 */
function parseTheme(contents: string): ThemePreference {
  let settings: unknown;
  try {
    settings = load(contents);
  } catch {
    return "system";
  }
  const preference = (settings as ThemeSettings | null)?.["ui-theme"]?.preference;
  return preference === "light" || preference === "dark" || preference === "system" ? preference : "system";
}

function createTheme() {
  const theme = ref<ThemePreference>("system");
  const watching = ref(false);

  let started = false;
  let stopped = false;
  let unwatch: UnwatchFn | null = null;
  let watchPromise: Promise<UnwatchFn> | null = null;

  /** 读取并解析主题；读取或解析失败时保留最近一次有效主题（DESIGN.md §9.2）。 */
  async function refresh(): Promise<void> {
    try {
      const contents = await readTextFile(THEME_PATH, HOME_OPTIONS);
      theme.value = parseTheme(contents);
    } catch (cause) {
      console.warn("useTheme 读取主题失败:", cause);
    }
  }

  /** 注册一次性监听（并发安全），注册完成后立即首次读取。 */
  async function ensureWatcher(): Promise<void> {
    if (unwatch) return;
    watchPromise ??= watch(THEME_PATH, () => void refresh(), {
      ...HOME_OPTIONS,
      delayMs: WATCH_DELAY_MS,
    });
    try {
      unwatch = await watchPromise;
      watching.value = true;
    } finally {
      watchPromise = null;
    }
  }

  /** 轮询等待配置文件出现，文件出现后先监听再首次读取（DESIGN.md §9.3）。 */
  async function run(): Promise<void> {
    while (!stopped) {
      let found = false;
      try {
        found = await exists(THEME_PATH, HOME_OPTIONS);
      } catch (cause) {
        console.warn("useTheme 检查配置文件失败:", cause);
      }
      if (found) break;
      await sleep(POLL_INTERVAL_MS);
    }
    if (stopped) return;
    await ensureWatcher();
    await refresh();
  }

  /** 启动轮询与监听流程；幂等，多次调用不会重复注册监听。 */
  function start(): void {
    if (started) return;
    started = true;
    void run();
  }

  /** 取消文件监听并停止轮询。应用销毁或页面显式要求时调用。 */
  function stop(): void {
    stopped = true;
    unwatch?.();
    unwatch = null;
    watching.value = false;
  }

  start();

  return { theme: readonly(theme), watching, start, stop };
}

let singleton: ReturnType<typeof createTheme> | null = null;

/** 单例主题状态；多次调用共享同一份状态与同一份文件监听。 */
export function useTheme() {
  singleton ??= createTheme();
  return singleton;
}
