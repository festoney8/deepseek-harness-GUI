<template>
  <header
    class="flex h-9 min-h-9 select-none items-center border-b border-base-300 bg-base-100 pl-2 shadow-sm"
    data-tauri-drag-region
    @dblclick.self="toggleMaximize"
  >
    <div
      class="flex h-full min-w-0 flex-1 items-end overflow-x-auto overflow-y-hidden"
      data-tauri-drag-region
      @dblclick.self="toggleMaximize"
    >
      <div role="tablist" class="tabs tabs-lift tabs-sm min-w-max">
        <button
          v-for="tab in tabsStore.tabs"
          :key="tab.id"
          role="tab"
          type="button"
          class="tab max-w-64 gap-2"
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
          <span
            v-if="tab.closable"
            class="btn btn-xs btn-circle btn-ghost shrink-0"
            :class="{ 'opacity-50': tabsStore.activeTabId !== tab.id }"
            role="button"
            aria-label="关闭标签页"
            @click.stop="closeTab(tab)"
          >
            <CloseIcon class="size-4" aria-hidden="true" />
          </span>
        </button>
      </div>
    </div>

    <div class="flex h-full shrink-0 items-stretch" @dblclick.stop>
      <button
        class="btn btn-ghost h-full min-h-0 w-12 rounded-none p-0 hover:bg-base-300"
        type="button"
        aria-label="最小化"
        @click="minimize"
      >
        <IconMinimize class="size-4" aria-hidden="true" />
      </button>
      <button
        class="btn btn-ghost h-full min-h-0 w-12 rounded-none p-0 hover:bg-base-300"
        type="button"
        aria-label="最大化或还原"
        @click="toggleMaximize"
      >
        <IconMaximize class="size-4" aria-hidden="true" />
      </button>
      <button
        class="btn btn-ghost h-full min-h-0 w-12 rounded-none p-0 hover:bg-error hover:text-error-content"
        type="button"
        aria-label="隐藏到托盘"
        @click="hideWindow"
      >
        <IconClose class="size-4" aria-hidden="true" />
      </button>
    </div>
  </header>
</template>

<script setup lang="ts">
import { getCurrentWindow } from "@tauri-apps/api/window";
import CloseIcon from "~icons/material-symbols/close";
import IconMinimize from "~icons/mingcute/minimize-fill";
import IconMaximize from "~icons/mdi/maximize";
import IconClose from "~icons/mingcute/close-line";
import deepseekIcon from "../../assets/deepseek.svg";
import appLogo from "../../assets/logo_mini.png";
import { useTabsStore, type BrowserTab } from "../../stores/tabs";
import { hideToTray } from "../../ipc/ipc";
import { getErrorMessage, useToast } from "../../composables/useToast";

const tabsStore = useTabsStore();
const toast = useToast();
const appWindow = getCurrentWindow();

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
    toast.error(getErrorMessage(error));
  }
}

async function toggleMaximize(): Promise<void> {
  try {
    await appWindow.toggleMaximize();
  } catch (error) {
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
