import { Command } from "@tauri-apps/plugin-shell";
import { logger } from "../utils/log";

/** 已安装的 dsh 插件条目(name 为 dependencies 对象的 key) */
export interface DshPlugin {
  name: string;
  from: string;
  version: string;
}

/**
 * 解析 `dsh plugin --profile web list --json` 输出，提取 dependencies 条目的
 * name/from/version（unsavedDependencies 与 path 不取）；
 * 结构异常（JSON 解析失败、非数组、无 dependencies）时抛错，由调用方兜底
 */
function parseDshPlugins(stdout: string): DshPlugin[] {
  const data: unknown = JSON.parse(stdout);
  const profile = Array.isArray(data) ? data[0] : undefined;
  const dependencies = (profile as { dependencies?: Record<string, unknown> } | undefined)?.dependencies;
  if (!dependencies || typeof dependencies !== "object") {
    throw new Error("输出缺少 dependencies 对象");
  }
  return Object.entries(dependencies).flatMap(([name, value]) => {
    const dependency = value as { from?: unknown; version?: unknown };
    return typeof dependency?.from === "string" && typeof dependency?.version === "string"
      ? [{ name, from: dependency.from, version: dependency.version }]
      : [];
  });
}

/**
 * 执行 dsh plugin --profile web list --json 获取当前已安装插件列表。
 * 无缓存，每次调用都重新执行命令。
 * dsh 命令不存在、命令运行出错、JSON 解析出错时记日志并返回空列表
 */
export async function useDshPluginList(): Promise<DshPlugin[]> {
  try {
    const result = await Command.create("dsh-plugin-list", ["plugin", "--profile", "web", "list", "--json"]).execute();
    if (result.code !== 0) {
      const detail = result.stderr.trim() || `退出码 ${result.code}`;
      logger.warn("useDshPluginList", "列出插件失败:", detail);
      return [];
    }
    try {
      return parseDshPlugins(result.stdout);
    } catch (cause) {
      logger.warn("useDshPluginList", "解析插件列表失败:", cause);
      return [];
    }
  } catch (cause) {
    logger.warn("useDshPluginList", "执行 dsh plugin list 失败:", cause);
    return [];
  }
}
