<template>
  <header
    class="flex h-9 min-h-9 select-none items-center border-b border-base-300 bg-base-100 pl-2"
    data-tauri-drag-region
  >
    <div v-if="isMacOS" class="flex h-full shrink-0 items-center gap-3 pl-2.5 pr-4" data-tauri-drag-region>
      <button
        class="mac-traffic-light mac-close btn btn-xs zoom-55 btn-circle border-0 p-0"
        type="button"
        aria-label="关闭"
        title="关闭"
        @click="hideWindow"
      ></button>
      <button
        class="mac-traffic-light mac-minimize btn btn-xs zoom-55 btn-circle border-0 p-0"
        type="button"
        aria-label="最小化"
        title="最小化"
        @click="minimize"
      ></button>
      <button
        class="mac-traffic-light mac-maximize btn btn-xs zoom-55 btn-circle border-0 p-0"
        type="button"
        :aria-label="isMaximized ? '还原' : '全屏'"
        :title="isMaximized ? '还原' : '全屏'"
        @click="toggleMaximize"
      ></button>
    </div>
    <div class="flex h-full min-w-0 flex-1 items-end overflow-x-auto overflow-y-hidden" data-tauri-drag-region>
      <div role="tablist" class="tabs tabs-lift tabs-sm min-w-max">
        <button
          v-for="tab in tabsStore.tabs"
          :key="tab.id"
          role="tab"
          type="button"
          class="tab max-w-64 gap-1"
          :class="{ 'tab-active': tabsStore.activeTabId === tab.id }"
          :aria-selected="tabsStore.activeTabId === tab.id"
          :title="tab.url || tab.displayName"
          @click="selectTab(tab)"
          @mousedown.middle.prevent.stop="closeTab(tab)"
        >
          <img
            :src="tab.id === 'home' ? appLogo : deepseekIcon"
            class="size-4 ml-1 shrink-0"
            alt=""
            aria-hidden="true"
          />
          <span class="max-w-52 truncate" :class="{ 'font-bold': tabsStore.activeTabId === tab.id }">
            {{ tab.displayName }}
          </span>
          <button
            v-if="tab.closable"
            class="btn btn-xs btn-circle btn-ghost shrink-0"
            :class="{ 'opacity-50': tabsStore.activeTabId !== tab.id }"
            type="button"
            aria-label="关闭标签页"
            @click.stop="closeTab(tab)"
          >
            <CloseIcon class="size-4" aria-hidden="true" />
          </button>
          <span v-else class="btn btn-xs btn-circle invisible shrink-0" aria-hidden="true"></span>
        </button>
      </div>
    </div>

    <div class="flex h-full shrink-0 items-stretch">
      <button
        class="btn btn-ghost h-full min-h-0 w-10 rounded-none p-0 hover:bg-base-300"
        type="button"
        :aria-label="`APP主题：${themeLabel}`"
        :title="`APP主题：${themeLabel}`"
        @click="cycleTheme"
      >
        <IconSun v-if="preference === 'light'" class="size-4" aria-hidden="true" />
        <IconMoon v-else-if="preference === 'dark'" class="size-4" aria-hidden="true" />
        <IconSystem v-else class="size-4" aria-hidden="true" />
      </button>
      <template v-if="isWindows">
        <button
          class="btn btn-ghost h-full min-h-0 w-10 rounded-none p-0 hover:bg-base-300"
          type="button"
          aria-label="最小化"
          @click="minimize"
        >
          <IconMinimize class="size-4" aria-hidden="true" />
        </button>
        <button
          class="btn btn-ghost h-full min-h-0 w-10 rounded-none p-0 hover:bg-base-300"
          type="button"
          :aria-label="isMaximized ? '还原' : '最大化'"
          @click="toggleMaximize"
        >
          <IconRestore v-if="isMaximized" class="size-4" aria-hidden="true" />
          <IconMaximize v-else class="size-4" aria-hidden="true" />
        </button>
        <button
          class="btn btn-ghost h-full min-h-0 w-10 rounded-none p-0 hover:bg-error hover:text-error-content"
          type="button"
          aria-label="隐藏到托盘"
          @click="hideWindow"
        >
          <IconClose class="size-4" aria-hidden="true" />
        </button>
      </template>
    </div>
  </header>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { platform } from "@tauri-apps/plugin-os";
import { getCurrentWindow } from "@tauri-apps/api/window";
import CloseIcon from "~icons/material-symbols/close";
import IconMinimize from "~icons/mingcute/minimize-fill";
import IconMaximize from "~icons/mdi/maximize";
import IconRestore from "~icons/pixel/window-restore";
import IconSun from "~icons/gravity-ui/sun";
import IconMoon from "~icons/solar/moon-line-duotone";
import IconSystem from "~icons/material-symbols/light-mode-auto";
import IconClose from "~icons/mingcute/close-line";
import deepseekIcon from "@/assets/deepseek.svg";
import appLogo from "@/assets/logo_mini.png";
import { useTabsStore, type BrowserTab } from "@/stores/tabs";
import { useTheme, type ThemePreference } from "@/composables/useTheme";
import { hideToTray } from "@/ipc/ipc";
import { getErrorMessage, useToast } from "@/composables/useToast";
import { logger } from "@/utils/log";

const tabsStore = useTabsStore();
const toast = useToast();
const appWindow = getCurrentWindow();

const { preference, cycleTheme } = useTheme();

const THEME_LABELS: Record<ThemePreference, string> = {
  light: "日间模式",
  dark: "夜间模式",
  system: "系统模式",
};
const themeLabel = computed(() => THEME_LABELS[preference.value] ?? "系统模式");

const p = platform();
const isMacOS = p === "macos";
const isWindows = p === "windows";

const isMaximized = ref(false);
let unlistenResized: (() => void) | undefined;

async function refreshMaximizeState(): Promise<void> {
  try {
    isMaximized.value = await appWindow.isMaximized();
  } catch (error) {
    // 仅影响图标展示,查询失败时保留上次状态即可
    logger.warn("window", "查询窗口最大化状态失败（仅影响图标显示）:", error);
  }
}

onMounted(async () => {
  await refreshMaximizeState();
  // 最大化/还原在 Windows 上必然伴随窗口尺寸变化(WM_SIZE),因此用 resize 事件驱动刷新
  unlistenResized = await appWindow.onResized(() => void refreshMaximizeState());
});

onUnmounted(() => {
  unlistenResized?.();
});

async function selectTab(tab: BrowserTab): Promise<void> {
  try {
    await tabsStore.activateTab(tab);
  } catch (error) {
    toast.error(getErrorMessage(error));
  }
}

async function closeTab(tab: BrowserTab): Promise<void> {
  try {
    await tabsStore.closeTab(tab);
  } catch (error) {
    toast.error(getErrorMessage(error));
  }
}

async function minimize(): Promise<void> {
  try {
    await appWindow.minimize();
  } catch (error) {
    logger.error("window", "窗口最小化失败:", error);
    toast.error(getErrorMessage(error));
  }
}

async function toggleMaximize(): Promise<void> {
  try {
    await appWindow.toggleMaximize();
  } catch (error) {
    logger.error("window", "窗口最大化/还原失败:", error);
    toast.error(getErrorMessage(error));
  }
}

async function hideWindow(): Promise<void> {
  try {
    await hideToTray();
  } catch (error) {
    toast.error(getErrorMessage(error));
  }
}
</script>

<style lang="scss" scoped>
.mac-traffic-light {
  color: #1f2937;
  border-color: transparent;
  &.mac-close {
    background-color: #ff5f57 !important;
  }
  &.mac-minimize {
    background-color: #febc2e !important;
  }
  &.mac-maximize {
    background-color: #28c840 !important;
  }
  &:hover {
    transform: scale(1.2);
  }
}
</style>
