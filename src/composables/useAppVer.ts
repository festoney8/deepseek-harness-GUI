import { ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";

/**
 * 读取当前 App 版本号（对应 tauri.conf.json 中的 version）。
 * 获取失败时返回 undefined。
 */
export function useAppVer() {
  const version = ref<string | undefined>(undefined);
  const error = ref<string | null>(null);
  const checking = ref(false);

  async function check(): Promise<string | undefined> {
    checking.value = true;
    error.value = null;
    try {
      version.value = await getVersion();
      return version.value;
    } catch (cause) {
      console.warn("useAppVer 获取版本失败:", cause);
      version.value = undefined;
      error.value = cause instanceof Error ? cause.message : String(cause);
      return undefined;
    } finally {
      checking.value = false;
    }
  }

  return { version, error, checking, check };
}
