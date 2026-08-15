import { load } from "js-yaml";

export type ThemePreference = "light" | "dark" | "system";

interface SettingsFile {
  "ui-theme"?: { preference?: unknown };
}

/** 从 dsh settings.yaml 文本中提取 ui-theme.preference，缺失/非法/解析失败一律 system */
export function parseThemePreference(text: string): ThemePreference {
  let data: SettingsFile;
  try {
    data = (load(text) ?? {}) as SettingsFile;
  } catch {
    return "system";
  }
  const pref = data["ui-theme"]?.preference;
  return pref === "light" || pref === "dark" ? pref : "system";
}
