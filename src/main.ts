import { createApp } from "vue";
import { createPinia } from "pinia";
import ui from "@nuxt/ui/vue-plugin";
import App from "./App.vue";

createApp(App).use(createPinia()).mount("#app");

const app = createApp(App);
app.use(ui);
app.mount("#app");
