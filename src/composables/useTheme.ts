import { computed, watch } from "vue";
import { useLocalStorage, usePreferredDark } from "@vueuse/core";

/** 主题偏好：亮色 / 暗色 / 跟随系统 */
export type ThemePreference = "light" | "dark" | "system";

/** 点击切换时的循环顺序：日间 → 夜间 → 系统 */
const PREFERENCE_CYCLE: readonly ThemePreference[] = ["light", "dark", "system"];

/** 当前主题偏好，localStorage 持久化，与 DSH 配置文件解耦 */
const preference = useLocalStorage<ThemePreference>("ui-theme", "system");
/** 系统深色模式偏好 */
const systemPrefersDark = usePreferredDark();

/** 偏好解析后的 daisyUI 主题名 */
const daisyTheme = computed(() =>
  preference.value === "dark" || (preference.value === "system" && systemPrefersDark.value) ? "night" : "winter",
);

// 模块级单例，导入即生效：偏好或系统主题变化时更新 <html data-theme>
watch(
  daisyTheme,
  (theme) => {
    document.documentElement.dataset.theme = theme;
  },
  { immediate: true },
);

/** 主题访问入口：读取当前偏好，或按 日间 → 夜间 → 系统 循环切换 */
export function useTheme() {
  const cycleTheme = (): void => {
    const index = PREFERENCE_CYCLE.indexOf(preference.value);
    preference.value = PREFERENCE_CYCLE[(index + 1) % PREFERENCE_CYCLE.length] ?? "system";
  };

  return {
    /** 只读当前偏好 */
    preference: computed(() => preference.value),
    /** 解析后的 daisyUI 主题名 */
    daisyTheme,
    /** 切换到下一个偏好 */
    cycleTheme,
  };
}
