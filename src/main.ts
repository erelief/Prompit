import { createApp } from "vue";
import { invoke } from "@tauri-apps/api/core";
import App from "./App.vue";
import router from "./router";
import i18n from "./i18n";
import { loadConfig, loadSkillsLites, appConfig, enableConfigAutosave } from "./stores/config";
import { initTheme } from "./composables/useTheme";
import "./style.css";

const app = createApp(App);
app.use(router);
app.use(i18n);

function applyRouteTheme(path: string) {
  const isSettings = path === "/settings" || path === "/settings/dictionary" || path === "/onboarding" || path === "/startup-reminder";
  const bg = isSettings ? "var(--color-bg)" : "transparent";
  document.documentElement.style.background = bg;
  document.body.style.background = bg;
  document.body.style.overflow = isSettings ? "auto" : "hidden";
  document.getElementById("app")!.style.background = bg;
}

router.afterEach((to) => {
  applyRouteTheme(to.path);
});

router.isReady().then(async () => {
  // Load config first so theme is known before first paint
  await loadConfig();
  // Enable debounced auto-save after initial load so the initial values
  // don't trigger an immediate write-back, and all views share one save path.
  enableConfigAutosave();
  initTheme();
  await loadSkillsLites();

  // Mark onboarding complete for returning users
  // (OnboardingState defaults to false on each launch; only set to true
  // inside Onboarding.vue. Without this, the global shortcut is blocked.)
  if (appConfig.providers.length > 0) {
    invoke("set_onboarding_complete");
  }

  // Decide initial route based on state.
  // The startup reminder is gated on BOTH the persisted user preference
  // (show_startup_reminder) AND a process-level flag from the backend
  // (has_shown_startup_reminder). The latter resets only when the process
  // exits, so a wake-triggered WebView reload (lid close/open on a laptop)
  // — which re-runs this whole startup sequence — won't re-show the reminder,
  // even though the frontend state is freshly initialized.
  if (appConfig.providers.length === 0) {
    // First-run: show onboarding
    router.replace("/onboarding");
  } else if (
    appConfig.show_startup_reminder &&
    !(await invoke<boolean>("has_shown_startup_reminder"))
  ) {
    // Non-first-run with reminder enabled and not yet shown this session
    router.replace("/startup-reminder");
    invoke("mark_startup_reminder_shown");
  }
  // else: stays on "/" (floating-input, window hidden)

  applyRouteTheme(router.currentRoute.value.path);
  app.mount("#app");

  // The window content is now mounted and can render. Reveal the tray icon
  // (kept hidden during the IPC-heavy startup above) so any user interaction
  // lands on a fully-initialized window instead of an empty bordered one.
  invoke("set_tray_visible", { visible: true });

  // Show window immediately for onboarding
  if (appConfig.providers.length === 0) {
    invoke("show_onboarding_window");
  }
});
