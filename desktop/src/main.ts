import App from "./App.svelte";
import NotificationPopup from "./lib/NotificationPopup.svelte";
import { mount } from "svelte";

// The notification-popup window loads this same entry point with
// ?popup=1 — mount the small banner UI instead of the full app shell.
const isPopup = new URLSearchParams(location.search).has("popup");

const app = mount(isPopup ? NotificationPopup : App, {
  target: document.getElementById("app")!,
});

export default app;
