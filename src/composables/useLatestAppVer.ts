import { ref } from "vue";
import { fetchJson } from "../utils/http";

const LATEST_RELEASE_URL = "https://api.github.com/repos/festoney8/deepseek-harness-GUI/releases/latest";

interface GithubLatestRelease {
  tag_name?: string;
}

/**
 * 查询本 App 在 GitHub 上的最新 release 版本（tag_name 字段）。
 * 网络或解析失败一律返回 undefined。
 */
export function useLatestAppVer() {
  const version = ref<string | undefined>(undefined);
  const error = ref<string | null>(null);
  const checking = ref(false);

  async function check(): Promise<string | undefined> {
    checking.value = true;
    error.value = null;
    try {
      const data = (await fetchJson(LATEST_RELEASE_URL)) as GithubLatestRelease;
      version.value = typeof data.tag_name === "string" && data.tag_name ? data.tag_name : undefined;
      if (!version.value) {
        error.value = "响应缺少 tag_name 字段";
      }
      return version.value;
    } catch (cause) {
      console.warn("useLatestAppVer 查询最新版本失败:", cause);
      version.value = undefined;
      error.value = cause instanceof Error ? cause.message : String(cause);
      return undefined;
    } finally {
      checking.value = false;
    }
  }

  return { version, error, checking, check };
}
