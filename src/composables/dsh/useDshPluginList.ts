import { BaseDirectory, readTextFile } from "@tauri-apps/plugin-fs";
import { load, CORE_SCHEMA, defineScalarTag } from "js-yaml";
import { Command } from "@tauri-apps/plugin-shell";
import { logger } from "@/utils/log";

/** 已安装的 dsh 插件条目(name 为 dependencies 对象的 key) */
export interface DshPlugin {
  id: string;
  name: string;
  version: string;
}

/** 将不关注的 Cordis JavaScript 标签按普通字符串读取，避免阻塞整个 YAML 文件解析 */
const cordisJsTag = defineScalarTag("tag:yaml.org,2002:js", {
  resolve: (source) => source,
  identify: (value) => typeof value === "string",
});
const cordisYamlSchema = CORE_SCHEMA.withTags(cordisJsTag);

/**
 * 解析 `dsh plugin --profile web list --json` 输出，提取 dependencies 条目的
 * name/version（id 后续从 Cordis 配置补充；unsavedDependencies 与 path 不取）；
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
    const dependency = value as { version?: unknown };
    return typeof dependency?.version === "string" ? [{ id: "", name, version: dependency.version }] : [];
  });
}

/** 从 Cordis patch 文件的 insert 条目中提取第一个合法插件 ID */
function parseCordisPluginId(yaml: string): string {
  const data: unknown = load(yaml, { schema: cordisYamlSchema });
  if (!Array.isArray(data)) return "";

  for (const patch of data) {
    const inserts = (patch as { insert?: unknown } | null)?.insert;
    if (!Array.isArray(inserts)) continue;

    for (const insert of inserts) {
      const id = (insert as { id?: unknown } | null)?.id;
      if (typeof id === "string" && id.trim() !== "") return id;
    }
  }
  return "";
}

/** 读取单个插件的 Cordis patch 文件；单个插件失败时保留空 ID */
async function readCordisPluginId(name: string): Promise<string> {
  const path = `.dsh/profiles/web/node_modules/${name}/cordis.patch.yml`;
  try {
    const yaml = await readTextFile(path, { baseDir: BaseDirectory.Home });
    const id = parseCordisPluginId(yaml);
    if (!id) {
      logger.warn("useDshPluginList", `插件 ${name} 的 Cordis 配置缺少合法 insert.id`);
    }
    return id;
  } catch (cause) {
    logger.warn("useDshPluginList", `读取插件 ${name} 的 Cordis 配置失败:`, cause);
    return "";
  }
}

/** 为已解析的插件列表补充 Cordis 配置中的插件 ID */
async function enrichDshPluginIds(plugins: DshPlugin[]): Promise<DshPlugin[]> {
  return Promise.all(
    plugins.map(async (plugin) => ({
      ...plugin,
      id: await readCordisPluginId(plugin.name),
    })),
  );
}

/**
 * 执行 dsh plugin --profile web list --json 获取当前已安装插件列表。
 * 无缓存，每次调用都重新执行命令。
 * dsh 命令不存在、命令运行出错、JSON 解析出错时记日志并返回空列表；
 * 单个插件的 Cordis 配置读取失败时保留该插件并将 id 设为空字符串
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
      return await enrichDshPluginIds(parseDshPlugins(result.stdout));
    } catch (cause) {
      logger.warn("useDshPluginList", "解析插件列表失败:", cause);
      return [];
    }
  } catch (cause) {
    logger.warn("useDshPluginList", "执行 dsh plugin list 失败:", cause);
    return [];
  }
}
