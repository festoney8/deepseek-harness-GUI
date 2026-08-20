import { ref } from "vue";
import { connectRemote, type IpcError } from "../ipc/ipc";

export function useConnectRemote() {
  const protocol = ref<"http" | "https">("http");
  const host = ref("");
  const port = ref<number | null>(null);
  const loading = ref(false);
  const error = ref<IpcError | null>(null);

  /** 成功返回探测通过的地址，失败 rethrow IpcError 由调用方分支处理。 */
  async function connect(): Promise<string> {
    loading.value = true;
    error.value = null;
    try {
      return await connectRemote(protocol.value, host.value, port.value ?? 0);
    } catch (cause) {
      error.value = cause as IpcError;
      throw cause;
    } finally {
      loading.value = false;
    }
  }

  return { protocol, host, port, loading, error, connect };
}
