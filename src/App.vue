<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useDshStore } from "./stores/dsh";
import { useEnvStore, displayVersion } from "./stores/env";
import { useTheme } from "./composables/useTheme";
import { useConnectRemote } from "./composables/useConnectRemote";
import { useInstallDsh } from "./composables/useInstallDsh";
import { hideToTray, openLogs } from "./utils/ipc";

function reportMessage(error: unknown): string {
  if (typeof error === "object" && error !== null && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  return String(error);
}

function reportError(tag: string, error: unknown): void {
  const message = reportMessage(error);
  console.error(`[${tag}]`, error);
  alert(`[${tag}] ${message}`);
}

// 1-3. 版本号统一由 useEnvStore 管理
const env = useEnvStore();
const nodeDisplay = computed(() => displayVersion(env.nodeVer));
const npmDisplay = computed(() => displayVersion(env.npmVer));
const dshDisplay = computed(() => displayVersion(env.dshVer));
const latestDshDisplay = computed(() => displayVersion(env.latestDshVer));
const latestAppDisplay = computed(() => displayVersion(env.latestAppVer));
const appDisplay = computed(() => displayVersion(env.appVer));

// 4. 主题（单例，自动监听）
const { theme } = useTheme();

// 5. dsh 生命周期（pinia）
const store = useDshStore();
const port = ref(3000);

watch(
  () => store.phase,
  (next, prev) => console.log(`[dsh:phase] ${prev} -> ${next}`),
);

async function startDsh(): Promise<void> {
  try {
    const address = await store.start(Number(port.value));
    console.log("[start_dsh] ok:", address);
  } catch (error) {
    reportError("start_dsh", error);
  }
}

async function stopDsh(): Promise<void> {
  try {
    await store.stop();
    console.log("[stop_dsh] ok");
  } catch (error) {
    reportError("stop_dsh", error);
  }
}

async function restartDsh(): Promise<void> {
  try {
    const address = await store.restart(Number(port.value));
    console.log("[restart_dsh] ok:", address);
  } catch (error) {
    reportError("restart_dsh", error);
  }
}

// 6. 远程连接
const { protocol, host, port: remotePort, error: remoteError, connect } = useConnectRemote();
const remotePortText = computed({
  get: () => (remotePort.value ?? "").toString(),
  set: (text: string) => {
    remotePort.value = text === "" ? null : Number(text);
  },
});

const remoteAddress = ref<string | null>(null);

async function connectRemoteTest(): Promise<void> {
  try {
    remoteAddress.value = await connect();
    console.log("[connect_remote] ok:", remoteAddress.value);
  } catch (error) {
    remoteAddress.value = null;
    reportError("connect_remote", error);
  }
}

// 7. 安装 dsh（spawn，输出进 console）
const installNpmjs = useInstallDsh(false);
const installNpmjsRunning = installNpmjs.running;
const installNpmmirror = useInstallDsh(true);
const installNpmmirrorRunning = installNpmmirror.running;

async function doInstallNpmjs(): Promise<void> {
  try {
    await installNpmjs.start();
    console.log("[install:npmjs] spawned");
  } catch (error) {
    reportError("install:npmjs", error);
  }
}

async function doInstallNpmmirror(): Promise<void> {
  try {
    await installNpmmirror.start();
    console.log("[install:npmmirror] spawned");
  } catch (error) {
    reportError("install:npmmirror", error);
  }
}

// 8. 工具
async function doOpenLogs(): Promise<void> {
  try {
    await openLogs();
    console.log("[open_logs] ok");
  } catch (error) {
    reportError("open_logs", error);
  }
}

async function doHideToTray(): Promise<void> {
  try {
    await hideToTray();
    console.log("[hide_to_tray] ok");
  } catch (error) {
    reportError("hide_to_tray", error);
  }
}

onMounted(async () => {
  try {
    await store.bindEvents();
    console.log("[dsh] dsh_exited 监听已注册");
  } catch (error) {
    reportError("bind_events", error);
  }
});
</script>

<template>
  <main class="test-page">
    <h1>DSH GUI 桥接层测试</h1>

    <section>
      <h2>1. 环境版本</h2>
      <p><button @click="env.getNodeVer()">node -v</button> {{ nodeDisplay }}</p>
      <p><button @click="env.getNpmVer()">npm -v</button> {{ npmDisplay }}</p>
      <p><button @click="env.getDshVer()">dsh -V</button> {{ dshDisplay }}</p>
    </section>

    <section>
      <h2>2. 最新版本</h2>
      <p>
        dsh 最新版本
        <button @click="env.getLatestDshVer()">官方源</button>
        <button @click="env.getLatestDshVerWithMirror()">镜像源</button>
        {{ latestDshDisplay }}
      </p>
      <p>
        App 最新版本
        <button @click="env.getLatestAppVer()">查询</button>
        {{ latestAppDisplay }}
      </p>
    </section>

    <section>
      <h2>3. App 版本</h2>
      <p><button @click="env.getAppVer()">查询</button> {{ appDisplay }}</p>
    </section>

    <section>
      <h2>4. 主题</h2>
      <p>当前主题：{{ theme }}（修改 $HOME/.dsh/settings.yaml 后自动刷新）</p>
    </section>

    <section>
      <h2>5. dsh 生命周期</h2>
      <p>
        PORT <input v-model.number="port" type="number" min="1" max="65535" />
        <button :disabled="!store.canStart" @click="startDsh()">启动</button>
        <button :disabled="!store.isRunning" @click="stopDsh()">停止</button>
        <button :disabled="!store.isRunning" @click="restartDsh()">重启</button>
      </p>
      <p>phase: {{ store.phase }} | address: {{ store.address ?? "—" }}</p>
      <p>isRunning: {{ store.isRunning }} | isBusy: {{ store.isBusy }} | canStart: {{ store.canStart }}</p>
      <p v-if="store.lastError">上次错误: [{{ store.lastError.code }}] {{ store.lastError.message }}</p>
    </section>

    <section>
      <h2>6. 远程连接</h2>
      <p>
        <select v-model="protocol">
          <option value="http">http</option>
          <option value="https">https</option>
        </select>
        <input v-model="host" placeholder="localhost" />
        <input v-model="remotePortText" type="number" placeholder="3000" min="1" max="65535" />
        <button @click="connectRemoteTest()">连接</button>
      </p>
      <p>
        {{ remoteAddress ?? (remoteError ? `[${remoteError.code}] ${remoteError.message}` : "—") }}
      </p>
    </section>

    <section>
      <h2>7. 安装 dsh</h2>
      <p>
        <button :disabled="installNpmjsRunning" @click="doInstallNpmjs()">通过官方源安装</button>
        {{ installNpmjsRunning ? "运行中" : "空闲" }}
      </p>
      <p>
        <button :disabled="installNpmmirrorRunning" @click="doInstallNpmmirror()">通过 npmmirror 安装</button>
        {{ installNpmmirrorRunning ? "运行中" : "空闲" }}
      </p>
      <p>安装输出实时打印在 DevTools / 终端 console。</p>
    </section>

    <section>
      <h2>8. 工具</h2>
      <p><button @click="doOpenLogs()">打开日志目录</button></p>
      <p><button @click="doHideToTray()">隐藏到托盘</button></p>
    </section>
  </main>
</template>

<style scoped>
.test-page {
  max-width: 720px;
  margin: 0 auto;
  padding: 24px 16px 64px;
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  color: #0f0f0f;
}

section {
  border: 1px solid #ddd;
  border-radius: 8px;
  padding: 12px 16px;
  margin-bottom: 16px;
}

h1 {
  font-size: 22px;
}

h2 {
  font-size: 16px;
  margin: 0 0 8px;
}

p {
  margin: 6px 0;
}

button {
  margin-right: 6px;
  cursor: pointer;
}

button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

input {
  margin-right: 6px;
  min-width: 90px;
}

@media (prefers-color-scheme: dark) {
  .test-page {
    color: #f6f6f6;
  }

  section {
    border-color: #444;
  }
}
</style>
