import { computed, ref } from "vue";

const REMOTE_STORAGE_KEY = "deepseek-harness.remote-connection";
const LOCAL_STORAGE_KEY = "deepseek-harness.local-port";

export type ConnectionProtocol = "http" | "https";

interface RemoteSettings {
  protocol: ConnectionProtocol;
  host: string;
  port: string;
}

function readRemoteSettings(): RemoteSettings {
  try {
    const raw = localStorage.getItem(REMOTE_STORAGE_KEY);
    if (!raw) return { protocol: "http", host: "127.0.0.1", port: "3080" };
    const value = JSON.parse(raw) as Partial<RemoteSettings>;
    return {
      protocol: value.protocol === "https" ? "https" : "http",
      host: typeof value.host === "string" ? value.host : "127.0.0.1",
      port: typeof value.port === "string" ? value.port : "3080",
    };
  } catch {
    return { protocol: "http", host: "127.0.0.1", port: "3080" };
  }
}

function readLocalPort(): string {
  try {
    return localStorage.getItem(LOCAL_STORAGE_KEY) ?? "3080";
  } catch {
    return "3080";
  }
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
  const initialRemote = readRemoteSettings();
  const protocol = ref<ConnectionProtocol>(initialRemote.protocol);
  const host = ref(initialRemote.host);
  const remotePort = ref(initialRemote.port);
  const localPort = ref(readLocalPort());
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
    localStorage.setItem(
      REMOTE_STORAGE_KEY,
      JSON.stringify({ protocol: protocol.value, host: normalizedHost.value, port: remotePort.value }),
    );
  }

  function saveLocal(): void {
    localStorage.setItem(LOCAL_STORAGE_KEY, localPort.value);
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
