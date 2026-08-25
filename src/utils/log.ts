import { warn, debug, trace, info, error } from "@tauri-apps/plugin-log";

/**
 * 将前端 console 日志同时转发到 Tauri 日志插件。
 * 原始 console 调用继续执行，因此日志仍会显示在 DevTools 中；
 * 日志插件再负责写入终端标准输出和会话日志文件。
 */
function forwardConsole(
  fnName: "log" | "debug" | "info" | "warn" | "error",
  logger: (message: string) => Promise<void>,
) {
  const original = console[fnName];
  console[fnName] = (message) => {
    original(message);
    void logger(message);
  };
}

forwardConsole("log", trace);
forwardConsole("debug", debug);
forwardConsole("info", info);
forwardConsole("warn", warn);
forwardConsole("error", error);
