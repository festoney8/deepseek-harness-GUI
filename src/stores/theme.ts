import { computed, ref } from "vue";
import { defineStore } from "pinia";
import { BaseDirectory, exists, readTextFile, watch, type UnwatchFn } from "@tauri-apps/plugin-fs";
import { load } from "js-yaml";
import { logger } from "../utils/log";

/** 主题偏好：亮色 / 暗色 / 跟随系统 */
export type ThemePreference = "light" | "dark" | "system";

/** 主题配置文件路径（相对家目录） */
const THEME_PATH = ".dsh/settings.yaml";
/** 家目录读取选项 */
const HOME_OPTIONS = { baseDir: BaseDirectory.Home };
/** 轮询间隔（毫秒） */
const POLL_INTERVAL_MS = 3_000;
/** 监听防抖延迟（毫秒） */
const WATCH_DELAY_MS = 100;

/** 主题设置文件结构 */
interface ThemeSettings {
  "ui-theme"?: { preference?: unknown };
}

/** 等待指定毫秒数 */
const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

/** 解析 ui-theme.preference；缺失、类型错误或值不受支持时回退 system（DESIGN.md §9.2） */
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

/**
 * 主题状态（Pinia 单例，替代原 useTheme 手写单例）
 * 首次调用 start() 后轮询并监听 $HOME/.dsh/settings.yaml；
 * 监听与应用同生命周期，不存在停止场景，停止即应用退出（DESIGN.md §9）
 */
export const useThemeStore = defineStore("theme", () => {
  /** 当前主题偏好 */
  const theme = ref<ThemePreference>("system");

  /** 是否已调用 start()；防止重复启动监听 */
  const started = ref(false);
  /** 文件监听注销函数 */
  let unwatch: UnwatchFn | null = null;

  /** 读取并解析主题；读取或解析失败时保留最近一次有效主题 */
  async function refresh(): Promise<void> {
    try {
      const contents = await readTextFile(THEME_PATH, HOME_OPTIONS);
      theme.value = parseTheme(contents);
    } catch (cause) {
      logger.warn("theme", "读取主题失败:", cause);
    }
  }

  /** 检查配置文件是否存在；检查失败视为不存在，继续轮询 */
  async function fileExists(): Promise<boolean> {
    try {
      return await exists(THEME_PATH, HOME_OPTIONS);
    } catch (cause) {
      logger.warn("theme", "检查配置文件失败:", cause);
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
        logger.warn("theme", "注册监听失败，稍后重试:", cause);
        await sleep(POLL_INTERVAL_MS);
      }
    }
    await refresh();
  }

  /** 启动轮询与监听；幂等，重复调用不会重复启动 */
  function start(): void {
    if (started.value) return;
    started.value = true;
    void run();
  }

  return {
    /** 只读主题偏好，外部只能订阅，不能直接写入 */
    theme: computed(() => theme.value),
    started,
    start,
  };
});
