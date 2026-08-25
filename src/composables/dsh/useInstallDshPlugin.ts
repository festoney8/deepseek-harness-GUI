import { ref, toValue, type MaybeRef } from "vue";
import { Command } from "@tauri-apps/plugin-shell";

const DSH_PLUGIN_ADD_PREFIX = "dsh plugin --profile web add ";

/**
 * 从完整安装命令中提取插件 spec。
 * 仅接受 `dsh plugin --profile web add <plugin>`，不允许空插件名或额外参数。
 */
function parsePluginSpec(installCommand: string): string {
  if (!installCommand.startsWith(DSH_PLUGIN_ADD_PREFIX)) {
    throw new Error(`插件安装命令必须以“${DSH_PLUGIN_ADD_PREFIX}”开头`);
  }

  const pluginSpec = installCommand.slice(DSH_PLUGIN_ADD_PREFIX.length).trim();
  if (!pluginSpec) {
    throw new Error("插件安装命令缺少插件名称");
  }
  if (/\s/.test(pluginSpec)) {
    throw new Error("插件安装命令只能包含一个插件名称或包 spec");
  }

  return pluginSpec;
}

/**
 * 通过 capability 逻辑命令安装指定 DSH 插件。
 * start() 校验完整命令后，以 spawn 模式运行 dsh，并在退出码非 0 时 reject。
 */
export function useInstallDshPlugin(installCommand: MaybeRef<string>) {
  const running = ref(false);

  async function start(): Promise<void> {
    if (running.value) return;

    const pluginSpec = parsePluginSpec(toValue(installCommand));
    const command = Command.create("dsh-plugin-add", ["plugin", "--profile", "web", "add", pluginSpec]);
    running.value = true;

    return new Promise<void>((resolve, reject) => {
      let settled = false;
      const settle = (error?: Error) => {
        if (settled) return;
        settled = true;
        running.value = false;
        if (error) reject(error);
        else resolve();
      };

      command.stdout.on("data", (line) => console.info("plugin-install:stdout", line));
      command.stderr.on("data", (line) => console.info("plugin-install:stderr", line));
      command.on("close", ({ code, signal }) => {
        console.info("plugin-install:close", { code, signal });
        if (code === 0) settle();
        else settle(new Error(`DSH 插件安装失败：退出码 ${code ?? "未知"}`));
      });
      command.on("error", (cause) => {
        console.error("plugin-install:error", cause);
        settle(new Error(String(cause)));
      });
      void command.spawn().catch((cause) => {
        console.error("plugin-install:spawn", cause);
        settle(new Error(String(cause)));
      });
    });
  }

  return { running, start };
}
