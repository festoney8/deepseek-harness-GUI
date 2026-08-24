import { ref } from "vue";
import { Command } from "@tauri-apps/plugin-shell";
import { logger } from "../utils/log";

// 安装 pnpm 使用的 npm 镜像源
const NPM_MIRROR_REGISTRY = "https://registry.npmmirror.com";
// 已注册到 Tauri capability（shell:allow-spawn）的 npx 安装命令名
const PNPM_INSTALL_CMD = "npx-install-pnpm";

/**
 * 给安装了 node 但没有 pnpm 的机器安装 pnpm。
 * 通过 capability 逻辑命令固定运行 `npx --registry=<npmmirror> -y get-pnpm`，
 * 以 spawn 模式流式打印全部输出与报错，并在退出码非 0 时 reject。
 * start() 已在运行时直接返回。
 */
export function useInstallPnpm() {
  const running = ref(false);

  async function start(): Promise<void> {
    if (running.value) return;
    running.value = true;

    const command = Command.create(PNPM_INSTALL_CMD, [`--registry=${NPM_MIRROR_REGISTRY}`, "-y", "get-pnpm"]);

    return new Promise<void>((resolve, reject) => {
      let settled = false;
      const settle = (error?: Error) => {
        if (settled) return;
        settled = true;
        running.value = false;
        if (error) reject(error);
        else resolve();
      };

      command.stdout.on("data", (line) => logger.info("pnpm-install:stdout", line));
      command.stderr.on("data", (line) => logger.info("pnpm-install:stderr", line));
      command.on("close", ({ code, signal }) => {
        logger.info("pnpm-install:close", { code, signal });
        if (code === 0) settle();
        else settle(new Error(`pnpm 安装失败：退出码 ${code ?? "未知"}`));
      });
      command.on("error", (cause) => {
        logger.error("pnpm-install:error", cause);
        settle(new Error(String(cause)));
      });
      void command.spawn().catch((cause) => {
        logger.error("pnpm-install:spawn", cause);
        settle(new Error(String(cause)));
      });
    });
  }

  return { running, start };
}
