import { ref } from "vue";
import { Command } from "@tauri-apps/plugin-shell";

/** 通过 capability 逻辑命令 dsh-version 执行 dsh -V 并取版本号。 */
export function useDshVer() {
  const version = ref<string | null>(null);
  const error = ref<string | null>(null);
  const checking = ref(false);

  async function check() {
    checking.value = true;
    error.value = null;
    try {
      const result = await Command.create("dsh-version", ["-V"]).execute();
      if (result.code === 0) {
        version.value = result.stdout.trim() || null;
      } else {
        error.value = result.stderr.trim() || `dsh -V 退出码 ${result.code}`;
      }
    } catch (cause) {
      error.value = String(cause);
    } finally {
      checking.value = false;
    }
  }

  return { version, error, checking, check };
}
