import { ref, type Ref } from "vue";
import { defineStore } from "pinia";
import { getVersion } from "@tauri-apps/api/app";
import { Command } from "@tauri-apps/plugin-shell";
import { logger } from "../utils/log";
import { fetchJson } from "../utils/http";

/** 官方源（npmjs）dsh 最新版本查询地址。 */
const LATEST_DSH_NPMJS = "https://registry.npmjs.org/@deepseek-ai/dsh/latest";
/** 镜像源（npmmirror）dsh 最新版本查询地址。 */
const LATEST_DSH_NPMMIRROR = "https://registry.npmmirror.com/@deepseek-ai/dsh/latest";
/** App（GitHub release）最新版本查询地址。 */
const LATEST_APP_RELEASE = "https://api.github.com/repos/festoney8/deepseek-harness-GUI/releases/latest";

/**
 * 版本号状态模型。所有版本号槽位共用同一判别联合，区分五种情况：
 * - idle     尚未发起获取
 * - checking 获取中
 * - ok       正常版本号
 * - missing  不存在（程序未安装 / 命令不可用 / 输出为空）
 * - error    获取失败（网络、权限、启动等）
 */
export type VersionState =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "ok"; version: string }
  | { kind: "missing" }
  | { kind: "error"; message: string };

/** 初始状态：尚未获取。 */
export function idleVersionState(): VersionState {
  return { kind: "idle" };
}

/** 把状态归一化为可直接渲染的展示文案。 */
export function displayVersion(state: VersionState): string {
  switch (state.kind) {
    case "idle":
      return "—";
    case "checking":
      return "获取中…";
    case "ok":
      return state.version;
    case "missing":
      return "未安装";
    case "error":
      return state.message;
  }
}

/** shell 插件 rejection 中代表“命令不存在”的常见文案。 */
const NOT_FOUND_PATTERNS =
  /ObjectNotFound|CommandNotFoundException|ENOENT|command not found|no such file or directory|is not recognized|不是内部或外部命令/i;

/** 判断错误文案是否表示“命令不存在”。 */
function looksLikeMissingCommand(message: string): boolean {
  return NOT_FOUND_PATTERNS.test(message);
}

/**
 * 通过 capability 逻辑命令执行版本检查（node -v / npm -v / dsh -V）。
 * 退出码 0 且输出非空 → ok；输出为空 → missing；非 0 退出 → error；
 * 插件 rejection 中提示命令不存在 → missing，其余 → error。
 */
async function getShellVersion(commandName: string, args: string[]): Promise<VersionState> {
  try {
    const result = await Command.create(commandName, args).execute();
    if (result.code !== 0) {
      const detail = result.stderr.trim() || `退出码 ${result.code}`;
      return { kind: "error", message: detail };
    }
    const version = result.stdout.trim();
    return version ? { kind: "ok", version } : { kind: "missing" };
  } catch (cause) {
    const message = cause instanceof Error ? cause.message : String(cause);
    if (looksLikeMissingCommand(message)) return { kind: "missing" };
    logger.warn(commandName, "版本检查失败:", cause);
    return { kind: "error", message };
  }
}

/** 查询远程 latest 版本（GitHub release / npm registry），提取指定字段。 */
async function fetchLatestVersion(url: string, field: string): Promise<VersionState> {
  try {
    const data = (await fetchJson(url)) as Record<string, unknown>;
    const value = data[field];
    if (typeof value === "string" && value) {
      return { kind: "ok", version: value };
    }
    return { kind: "error", message: `响应缺少 ${field} 字段` };
  } catch (cause) {
    const message = cause instanceof Error ? cause.message : String(cause);
    logger.warn("fetchLatestVersion", "查询最新版本失败:", cause);
    return { kind: "error", message };
  }
}

/** 读取当前 App 版本（对应 tauri.conf.json 的 version）。 */
async function getAppVersion(): Promise<VersionState> {
  try {
    return { kind: "ok", version: await getVersion() };
  } catch (cause) {
    const message = cause instanceof Error ? cause.message : String(cause);
    logger.warn("getAppVersion", "获取版本失败:", cause);
    return { kind: "error", message };
  }
}

/**
 * 统一管理全部版本号：本地环境版本（node/npm/dsh）、网络最新版本（dsh/App）
 * 与 App 自身版本。每个槽位共用 VersionState 状态模型，
 * 区分 获取中 / 正常值 / 不存在 / 获取失败。
 */
export const useEnvStore = defineStore("env", () => {
  /** 本地 node 版本。 */
  const nodeVer = ref<VersionState>(idleVersionState());
  /** 本地 npm 版本。 */
  const npmVer = ref<VersionState>(idleVersionState());
  /** 本地 dsh 版本。 */
  const dshVer = ref<VersionState>(idleVersionState());

  /** 官方源 dsh 最新版本。 */
  const latestDshVer = ref<VersionState>(idleVersionState());
  /** 镜像源 dsh 最新版本。 */
  const latestDshVerWithMirror = ref<VersionState>(idleVersionState());
  /** App（GitHub release）最新版本。 */
  const latestAppVer = ref<VersionState>(idleVersionState());

  /** App 自身版本。 */
  const appVer = ref<VersionState>(idleVersionState());

  /** 执行异步版本获取并把结果写入对应槽位；获取中不重复触发。 */
  async function runGet(slot: Ref<VersionState>, action: () => Promise<VersionState>): Promise<void> {
    if (slot.value.kind === "checking") return;
    slot.value = { kind: "checking" };
    slot.value = await action();
  }

  /** 刷新本地 node 版本。 */
  async function getNodeVer(): Promise<void> {
    await runGet(nodeVer, () => getShellVersion("node-version", ["-v"]));
  }

  /** 刷新本地 npm 版本。 */
  async function getNpmVer(): Promise<void> {
    await runGet(npmVer, () => getShellVersion("npm-version", ["-v"]));
  }

  /** 刷新本地 dsh 版本。 */
  async function getDshVer(): Promise<void> {
    await runGet(dshVer, () => getShellVersion("dsh-version", ["-V"]));
  }

  /** 刷新官方源 dsh 最新版本。 */
  async function getLatestDshVer(): Promise<void> {
    await runGet(latestDshVer, () => fetchLatestVersion(LATEST_DSH_NPMJS, "version"));
  }

  /** 刷新镜像源 dsh 最新版本。 */
  async function getLatestDshVerWithMirror(): Promise<void> {
    await runGet(latestDshVerWithMirror, () => fetchLatestVersion(LATEST_DSH_NPMMIRROR, "version"));
  }

  /** 刷新 App（GitHub release）最新版本。 */
  async function getLatestAppVer(): Promise<void> {
    await runGet(latestAppVer, () => fetchLatestVersion(LATEST_APP_RELEASE, "tag_name"));
  }

  /** 刷新 App 自身版本。 */
  async function getAppVer(): Promise<void> {
    await runGet(appVer, getAppVersion);
  }

  return {
    nodeVer,
    npmVer,
    dshVer,
    latestDshVer,
    latestDshVerWithMirror,
    latestAppVer,
    appVer,
    getNodeVer,
    getNpmVer,
    getDshVer,
    getLatestDshVer,
    getLatestDshVerWithMirror,
    getLatestAppVer,
    getAppVer,
  };
});
