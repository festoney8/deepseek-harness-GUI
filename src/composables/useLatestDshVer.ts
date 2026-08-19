import { ref, toValue, type MaybeRef } from "vue";
import { fetchJson } from "../utils/http";

const LATEST_URL_NPMJS = "https://registry.npmjs.org/@deepseek-ai/dsh/latest";
const LATEST_URL_NPMMIRROR = "https://registry.npmmirror.org/@deepseek-ai/dsh/latest";

interface RegistryLatestResponse {
  version?: string;
}

/**
 * 查询 dsh 在 npm registry 的最新版本。
 * mirror 为 true（默认）时查询 npmmirror 源，否则查询官方源。
 * 网络或解析失败一律返回 undefined。
 */
export function useLatestDshVer(mirror: MaybeRef<boolean> = true) {
  const version = ref<string | undefined>(undefined);
  const error = ref<string | null>(null);
  const checking = ref(false);

  async function check(): Promise<string | undefined> {
    checking.value = true;
    error.value = null;
    const url = toValue(mirror) ? LATEST_URL_NPMMIRROR : LATEST_URL_NPMJS;
    try {
      const data = (await fetchJson(url)) as RegistryLatestResponse;
      version.value = typeof data.version === "string" && data.version ? data.version : undefined;
      if (!version.value) {
        error.value = "响应缺少 version 字段";
      }
      return version.value;
    } catch (cause) {
      console.warn("useLatestDshVer 查询最新版本失败:", cause);
      version.value = undefined;
      error.value = cause instanceof Error ? cause.message : String(cause);
      return undefined;
    } finally {
      checking.value = false;
    }
  }

  return { version, error, checking, check };
}
