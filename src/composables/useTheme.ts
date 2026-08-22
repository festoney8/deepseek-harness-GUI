import { computed, onBeforeUnmount, onMounted, watch } from "vue";
import { useThemeStore } from "../stores/theme";

/** 将 DSH 的主题偏好映射到已启用的 daisyUI 主题。 */
export function useTheme() {
  const themeStore = useThemeStore();
  let mediaQuery: MediaQueryList | null = null;

  const dataTheme = computed(() => {
    if (themeStore.theme === "light") return "winter";
    if (themeStore.theme === "dark") return "night";
    return mediaQuery?.matches ? "night" : "winter";
  });

  const applyTheme = () => {
    document.documentElement.dataset.theme = dataTheme.value;
  };

  const handleSystemThemeChange = () => applyTheme();

  onMounted(() => {
    mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    mediaQuery.addEventListener("change", handleSystemThemeChange);
    applyTheme();
  });

  watch(dataTheme, applyTheme, { immediate: true });

  onBeforeUnmount(() => {
    mediaQuery?.removeEventListener("change", handleSystemThemeChange);
  });

  return { dataTheme };
}
