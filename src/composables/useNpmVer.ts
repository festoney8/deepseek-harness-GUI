import { ref } from "vue";
import { Command } from "@tauri-apps/plugin-shell";

/** 通过 capability 逻辑命令 npm-version 执行 npm -V 并取版本号。 */
export function useNpmVer() {
  const version = ref<string | null>(null);
  const error = ref<string | null>(null);
  const checking = ref(false);

  async function check() {
    checking.value = true;
    error.value = null;
    try {
      const result = await Command.create("npm-version", ["-V"]).execute();
      if (result.code === 0) {
        version.value = result.stdout.trim() || null;
      } else {
        error.value = result.stderr.trim() || `npm -V 退出码 ${result.code}`;
      }
    } catch (cause) {
      error.value = String(cause);
    } finally {
      checking.value = false;
    }
  }

  return { version, error, checking, check };
}
