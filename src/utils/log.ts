import { error as pluginError, info as pluginInfo, warn as pluginWarn } from "@tauri-apps/plugin-log";

/** 把任意值转成可写入日志的可读文本 */
function toText(value: unknown): string {
  if (typeof value === "string") return value;
  if (typeof value === "number" || typeof value === "boolean" || typeof value === "bigint") {
    return String(value);
  }
  if (value instanceof Error) return value.stack ?? value.message;
  try {
    const json = JSON.stringify(value);
    return json === undefined ? String(value) : json;
  } catch {
    return String(value);
  }
}

/** 把 tag 与参数序列化为 "[tag] body" 日志行 */
function format(tag: string, args: unknown[]): string {
  const body = args.map(toText).join(" ");
  return body ? `[${tag}] ${body}` : `[${tag}]`;
}

/**
 * 前端统一日志入口，写入与 Rust/dsh 相同的会话日志目录（DESIGN.md §10，
 * 经 tauri-plugin-log 落入 Stdout + Folder target）
 */
const writers = { info: pluginInfo, warn: pluginWarn, error: pluginError } as const;

type Level = keyof typeof writers;

/** 写入一行日志；日志插件自身失败时回退到控制台（唯一允许 console 的例外） */
function write(level: Level, tag: string, args: unknown[]): void {
  const line = format(tag, args);
  writers[level](line).catch((cause) => {
    console.error(`[log] 写入日志失败（${level}）:`, line, cause);
  });
}

export const logger = {
  info: (tag: string, ...args: unknown[]) => write("info", tag, args),
  warn: (tag: string, ...args: unknown[]) => write("warn", tag, args),
  error: (tag: string, ...args: unknown[]) => write("error", tag, args),
};
