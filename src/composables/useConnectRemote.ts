import { ref } from "vue";
import { connectRemote, type IpcError } from "../ipc/ipc";

/**
 * 远程连接表单状态与 connect 动作的 composite。
 */
export function useConnectRemote() {
  // 远程协议（http/https）
  const protocol = ref<"http" | "https">("http");
  // 远程主机名
  const host = ref("");
  // 远程端口
  const port = ref<number | null>(null);
  // 连接是否进行中
  const loading = ref(false);
  // 最近一次连接失败的错误
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
