import { ref, toValue, type MaybeRef } from "vue";
import { Command } from "@tauri-apps/plugin-shell";
import { logger } from "../utils/log";

// 官方 npm 源
const NPM_REGISTRY = "https://registry.npmjs.org";
// npmmirror 镜像源
const NPM_MIRROR_REGISTRY = "https://registry.npmmirror.com";

/** dsh 安装可选的发布标签（dist-tag）：latest 为正式版，next 为预发布 */
export type DshInstallTag = "latest" | "next";

/**
 * 通过 capability 逻辑命令安装 dsh，spawn 模式流式打印全部输出与报错
 * mirror 为 true 时使用 npmmirror 源；installTag 指定安装 latest 还是 next 标签，
 * 体现在包名 @deepseek-ai/dsh@<tag> 上。两者都支持传入 Ref 以便页面在运行前切换
 */
export function useInstallDsh(mirror: MaybeRef<boolean>, installTag: MaybeRef<DshInstallTag> = "latest") {
  // 安装是否运行中
  const running = ref(false);

  /**
   * 开启 dsh 安装：以 spawn 方式运行固定 npm install 命令，镜像与否由 mirror 决定，
   * 安装标签由 installTag 决定（对应 npmjs / npmmirror 各自的 latest / next 能力命令）。
   * 已在运行时直接返回
   */
  async function start() {
    if (running.value) return;
    running.value = true;
    const useMirror = toValue(mirror);
    const tag = toValue(installTag);
    const registry = useMirror ? NPM_MIRROR_REGISTRY : NPM_REGISTRY;
    // 每个 registry 都拆分为 latest / next 两条固定参数的能力命令
    const scopeName = useMirror
      ? tag === "next"
        ? "npm-install-dsh-npmmirror-next"
        : "npm-install-dsh-npmmirror"
      : tag === "next"
        ? "npm-install-dsh-npmjs-next"
        : "npm-install-dsh-npmjs";
    const command = Command.create(scopeName, [
      "install",
      "-g",
      "--verbose",
      `@deepseek-ai/dsh@${tag}`,
      `--registry=${registry}`,
    ]);
    command.stdout.on("data", (line) => logger.info("install:stdout", line));
    command.stderr.on("data", (line) => logger.info("install:stderr", line));
    command.on("close", ({ code, signal }) => {
      logger.info("install:close", { code, signal });
      running.value = false;
    });
    command.on("error", (cause) => {
      logger.error("install:error", cause);
      running.value = false;
    });
    try {
      await command.spawn();
    } catch (cause) {
      running.value = false;
      throw cause;
    }
  }

  return { running, start };
}
