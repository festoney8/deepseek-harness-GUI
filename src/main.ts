import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { logger } from "./utils/log";
import "./style.css";

const pinia = createPinia();
const app = createApp(App);

app.use(pinia);

// 全局兜底：任何未捕获的组件异常与 Promise rejection 至少留下一行日志
app.config.errorHandler = (error, _instance, info) => {
  logger.error("vue", "未捕获的组件错误:", error, "组件信息:", info);
};
window.addEventListener("unhandledrejection", (event) => {
  logger.error("unhandled", "未处理的 Promise rejection:", event.reason);
});

app.mount("#app");
logger.info("app", "前端应用已挂载");
