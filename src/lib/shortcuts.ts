import { useEffect } from "react";

import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";

import { NAVIGATION } from "../components/shell/AppSidebar";
import { inTauri } from "./api";
import { navigate } from "./router";
import { setSetting, useSettings } from "./settings";
import { newHabit, newTask, openPalette, showShortcuts, ui } from "./ui";

function moveInList(step: number) {
  const titles = Array.from(document.querySelectorAll<HTMLElement>("[data-task-title]"));
  if (!titles.length) return;
  const row = document.activeElement?.closest("[data-task-row]");
  const current = row ? titles.findIndex((title) => row.contains(title)) : -1;
  titles[Math.max(0, Math.min(titles.length - 1, current + step))]?.focus();
}

function completeFocused() {
  const row = document.activeElement?.closest("[data-task-row]");
  row?.querySelector<HTMLElement>("[role=checkbox]")?.click();
}

function isTyping(target: EventTarget | null) {
  const element = target as HTMLElement | null;
  return Boolean(element?.closest("input, textarea, select, [contenteditable]"));
}

export function useShortcuts() {
  const { sidebarCollapsed } = useSettings();

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const control = event.ctrlKey || event.metaKey;
      const key = event.key.toLowerCase();

      if (control && key === "k") {
        event.preventDefault();
        const { palette } = ui.get();
        if (palette) ui.set((state) => ({ ...state, palette: null }));
        else openPalette("commands");
      } else if (control && key === "n") {
        event.preventDefault();
        if (event.shiftKey) newHabit();
        else newTask();
      } else if (control && key === "b") {
        event.preventDefault();
        void setSetting("sidebarCollapsed", !sidebarCollapsed);
      } else if (control && key === ",") {
        event.preventDefault();
        navigate("settings");
      } else if (control && key === "f") {
        event.preventDefault();
        openPalette("search");
      } else if (control && /^[1-7]$/.test(key)) {
        event.preventDefault();
        const item = NAVIGATION[Number(key) - 1];
        if (item) navigate(item.page);
      } else if (control && key === "q" && inTauri) {
        event.preventDefault();
        void invoke("quit");
      } else if (control && key === "w" && inTauri) {
        event.preventDefault();
        void getCurrentWindow().close();
      } else if (event.key === "F1") {
        event.preventDefault();
        showShortcuts(true);
      } else if (!control && !event.altKey && !isTyping(event.target)) {
        if (event.key === "/") {
          event.preventDefault();
          openPalette("search");
        } else if (key === "j") {
          moveInList(1);
        } else if (key === "k") {
          moveInList(-1);
        } else if (key === "x") {
          completeFocused();
        }
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [sidebarCollapsed]);
}
