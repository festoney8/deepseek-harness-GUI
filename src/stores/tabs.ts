import { computed, ref } from "vue";
import { defineStore } from "pinia";
import {
  activateWebviewTab,
  closeWebviewTab,
  createWindowWithUrl,
  hideAllWebviewTabs,
  type WebviewTab,
} from "../ipc/ipc";

export interface BrowserTab {
  id: "home" | string;
  label: string;
  url: string;
  displayName: string;
  closable: boolean;
}

const HOME_TAB: BrowserTab = {
  id: "home",
  label: "main",
  url: "",
  displayName: "DeepSeek Harness GUI",
  closable: false,
};

function fromWebviewTab(tab: WebviewTab): BrowserTab {
  return {
    id: tab.label,
    label: tab.label,
    url: tab.url,
    displayName: tab.displayName || tab.url,
    closable: true,
  };
}

export const useTabsStore = defineStore("tabs", () => {
  const tabs = ref<BrowserTab[]>([{ ...HOME_TAB }]);
  const activeTabId = ref<BrowserTab["id"]>(HOME_TAB.id);
  const activeTab = computed(() => tabs.value.find((tab) => tab.id === activeTabId.value) ?? HOME_TAB);

  async function activateTab(tab: BrowserTab): Promise<void> {
    if (tab.id === "home") {
      await hideAllWebviewTabs();
    } else {
      await activateWebviewTab(tab.label);
    }
    activeTabId.value = tab.id;
  }

  async function openDshTab(tab: WebviewTab): Promise<void> {
    const nextTab = fromWebviewTab(tab);
    const index = tabs.value.findIndex((item) => item.id === nextTab.id);
    if (index === -1) {
      tabs.value.push(nextTab);
    } else {
      tabs.value[index] = nextTab;
    }
    activeTabId.value = nextTab.id;
    console.info("tabs", `打开标签页: ${nextTab.label} ${nextTab.url}`);
  }

  async function openDshUrl(url: string): Promise<void> {
    await openDshTab(await createWindowWithUrl(url));
  }

  async function closeTab(tab: BrowserTab): Promise<void> {
    if (!tab.closable) return;
    const index = tabs.value.findIndex((item) => item.id === tab.id);
    if (index === -1) return;
    await closeWebviewTab(tab.label);
    console.info("tabs", `关闭标签页: ${tab.label}`);
    tabs.value.splice(index, 1);
    if (activeTabId.value === tab.id) {
      const nextTab = tabs.value[index] ?? tabs.value[index - 1] ?? HOME_TAB;
      await activateTab(nextTab);
    }
  }

  return {
    tabs,
    activeTab,
    activeTabId,
    activateTab,
    openDshTab,
    openDshUrl,
    closeTab,
  };
});
