import { useLocalStorage } from "@vueuse/core";
import { computed, ref } from "vue";

const REMOTE_STORAGE_KEY = "deepseek-harness.remote-connection";
const LOCAL_STORAGE_KEY = "deepseek-harness.local-port";

export type ConnectionProtocol = "http" | "https";

interface RemoteSettings {
  protocol: ConnectionProtocol;
  host: string;
  port: string;
}

const DEFAULT_REMOTE_SETTINGS: RemoteSettings = {
  protocol: "http",
  host: "192.168.1.1",
  port: "3080",
};
const DEFAULT_LOCAL_PORT = "3080";

function normalizeRemoteSettings(value: Partial<RemoteSettings> | null): RemoteSettings {
  return {
    protocol: value?.protocol === "https" ? "https" : "http",
    host: typeof value?.host === "string" ? value.host : DEFAULT_REMOTE_SETTINGS.host,
    port: typeof value?.port === "string" ? value.port : DEFAULT_REMOTE_SETTINGS.port,
  };
}

function isValidPort(value: string): boolean {
  if (!/^[0-9]+$/.test(value)) return false;
  const port = Number(value);
  return Number.isInteger(port) && port >= 1 && port <= 65535;
}

function isValidHost(value: string): boolean {
  if (value.toLowerCase() === "localhost") return true;
  const parts = value.split(".");
  if (parts.length !== 4 || parts.some((part) => !/^\d+$/.test(part))) return false;
  if (parts.some((part) => part.length > 1 && part.startsWith("0"))) return false;
  return parts.every((part) => Number(part) >= 0 && Number(part) <= 255);
}

export function useConnectionForm() {
  const storedRemote = useLocalStorage<RemoteSettings>(REMOTE_STORAGE_KEY, DEFAULT_REMOTE_SETTINGS, {
    mergeDefaults: true,
  });
  const storedLocalPort = useLocalStorage(LOCAL_STORAGE_KEY, DEFAULT_LOCAL_PORT);
  const initialRemote = normalizeRemoteSettings(storedRemote.value);
  const protocol = ref<ConnectionProtocol>(initialRemote.protocol);
  const host = ref(initialRemote.host);
  const remotePort = ref(initialRemote.port);
  const localPort = ref(storedLocalPort.value);
  const remoteAttempted = ref(false);
  const localAttempted = ref(false);

  const validProtocol = computed(() => protocol.value === "http" || protocol.value === "https");
  const validHost = computed(() => isValidHost(host.value));
  const validRemotePort = computed(() => isValidPort(remotePort.value));
  const validLocalPort = computed(() => isValidPort(localPort.value));
  const validRemote = computed(() => validProtocol.value && validHost.value && validRemotePort.value);

  const normalizedHost = computed(() => host.value.toLowerCase());
  const remotePortNumber = computed(() => Number(remotePort.value));
  const localPortNumber = computed(() => Number(localPort.value));

  function saveRemote(): void {
    storedRemote.value = {
      protocol: protocol.value,
      host: normalizedHost.value,
      port: remotePort.value,
    };
  }

  function saveLocal(): void {
    storedLocalPort.value = localPort.value;
  }

  function markRemoteAttempted(): void {
    remoteAttempted.value = true;
  }

  function markLocalAttempted(): void {
    localAttempted.value = true;
  }

  return {
    protocol,
    host,
    remotePort,
    localPort,
    remoteAttempted,
    localAttempted,
    validProtocol,
    validHost,
    validRemotePort,
    validLocalPort,
    validRemote,
    normalizedHost,
    remotePortNumber,
    localPortNumber,
    saveRemote,
    saveLocal,
    markRemoteAttempted,
    markLocalAttempted,
  };
}
