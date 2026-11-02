import { createContext, type ReactNode, useContext, useEffect, useLayoutEffect } from "react";

import { getCurrentWindow } from "@tauri-apps/api/window";

import { inTauri } from "./api";
import { mutate, useRpc } from "./store";
import type { Settings } from "./types";

export const DEFAULT_SETTINGS: Settings = {
  theme: "system",
  weekStart: "monday",
  timeFormat: "24h",
  taskDensity: "comfortable",
  sidebarCollapsed: false,
  reduceMotion: false,
  defaultPriority: "medium",
  defaultReminderMinutes: 30,
  confirmDelete: true,
  closeToBackground: true,
  startAtLogin: false,
  notificationsEnabled: true,
  taskNotifications: true,
  habitNotifications: true,
  focusNotifications: true,
  dateOnlyReminderTime: "09:00",
  focusMinutes: 25,
  shortBreakMinutes: 5,
  longBreakMinutes: 15,
  sessionsBeforeLongBreak: 4,
};

const SettingsContext = createContext<Settings>(DEFAULT_SETTINGS);

export const useSettings = () => useContext(SettingsContext);

export function setSetting<K extends keyof Settings>(key: K, value: Settings[K]) {
  return mutate("settings.set", { key, value }).catch(() => undefined);
}

function applyTheme(theme: Settings["theme"], systemDark: boolean) {
  const dark = theme === "dark" || (theme === "system" && systemDark);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
  if (inTauri) {
    // Keeps the system title bar in step with a theme chosen here. Null follows the desktop again.
    void getCurrentWindow().setTheme(theme === "system" ? null : theme);
  }
}

export function SettingsProvider({ children }: { children: ReactNode }) {
  const { data } = useRpc<Settings>("settings.get");
  const settings = data ?? DEFAULT_SETTINGS;
  const { theme, reduceMotion } = settings;

  // A layout effect, so the theme is in place before the shell is told it may show the window.
  useLayoutEffect(() => {
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const update = () => applyTheme(theme, query.matches);
    update();
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, [theme]);

  useEffect(() => {
    document.documentElement.dataset.motion = reduceMotion ? "reduced" : "full";
  }, [reduceMotion]);

  // Rendering waits for the real settings, so nothing can appear in the wrong theme or layout.
  if (!data) return null;
  return <SettingsContext.Provider value={settings}>{children}</SettingsContext.Provider>;
}
