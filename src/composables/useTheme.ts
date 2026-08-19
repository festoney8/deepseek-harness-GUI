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

const theme = ref<ThemePreference>("system");

/** 是否已启动监听；首次调用 useTheme() 后置为 true，此后不再重复启动。 */
let isRunning = false;
let unwatch: UnwatchFn | null = null;

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

/** 读取并解析主题；读取或解析失败时保留最近一次有效主题 */
async function refresh(): Promise<void> {
  try {
    const contents = await readTextFile(THEME_PATH, HOME_OPTIONS);
    theme.value = parseTheme(contents);
  } catch (cause) {
    console.warn("useTheme 读取主题失败:", cause);
  }
}

/** 检查配置文件是否存在；检查失败视为不存在，继续轮询 */
async function fileExists(): Promise<boolean> {
  try {
    return await exists(THEME_PATH, HOME_OPTIONS);
  } catch (cause) {
    console.warn("useTheme 检查配置文件失败:", cause);
    return false;
  }
}

/** 轮询等待文件出现，出现后先监听再首次读取；监听注册失败时隔一段时间重试 */
async function run(): Promise<void> {
  while (!(await fileExists())) {
    await sleep(POLL_INTERVAL_MS);
  }
  while (!unwatch) {
    try {
      unwatch = await watch(THEME_PATH, () => void refresh(), {
        ...HOME_OPTIONS,
        delayMs: WATCH_DELAY_MS,
      });
    } catch (cause) {
      console.warn("useTheme 注册监听失败，稍后重试:", cause);
      await sleep(POLL_INTERVAL_MS);
    }
  }
  await refresh();
}

/**
 * 单例主题状态。首次调用即开始轮询并监听 $HOME/.dsh/settings.yaml；
 * 监听与应用同生命周期，不存在停止场景，停止即应用退出。
 */
export function useTheme() {
  if (!isRunning) {
    isRunning = true;
    void run();
  }
  return { theme: readonly(theme) };
}
