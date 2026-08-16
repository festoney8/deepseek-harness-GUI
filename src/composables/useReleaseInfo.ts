import { computed, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";

const LATEST_RELEASE_API = "https://api.github.com/repos/festoney8/deepseek-harness-GUI/releases/latest" as const;

export function useReleaseInfo() {
  /** 应用版本号，运行时读取（跟随 tauri.conf.json 的 version） */
  const appVersion = ref("");
  void getVersion().then((v) => {
    appVersion.value = v;
  });

  /** GitHub 最新 release 版本号（失败时显示“未知”） */
  const latestVersion = ref("检查中");
  async function fetchLatestVersion() {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), 5000);
    try {
      const res = await fetch(LATEST_RELEASE_API, { signal: controller.signal });
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = (await res.json()) as { tag_name?: string };
      latestVersion.value = data.tag_name ?? "未知";
    } catch {
      latestVersion.value = "未知";
    } finally {
      clearTimeout(timer);
    }
  }
  void fetchLatestVersion();

  /** 本工具 GitHub 最新版与当前版不一致（有新版本可升级），最新版号高亮；非版本号哨兵值（检查中/未知）不高亮 */
  const versionOutdated = computed(() => {
    const latest = latestVersion.value;
    const current = appVersion.value;
    if (!/^v?\d/.test(latest) || !/^v?\d/.test(current)) return false;
    return latest.replace(/^v/, "") !== current.replace(/^v/, "");
  });

  return { appVersion, latestVersion, versionOutdated };
}
