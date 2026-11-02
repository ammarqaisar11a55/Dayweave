import { useEffect, useRef } from "react";

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { HabitDetail } from "./components/habit/HabitDetail";
import { HabitDialog } from "./components/habit/HabitDialog";
import { ProjectDialog } from "./components/ProjectDialog";
import { AppSidebar } from "./components/shell/AppSidebar";
import { AppToolbar } from "./components/shell/AppToolbar";
import { CommandPalette } from "./components/shell/CommandPalette";
import { ShortcutsDialog } from "./components/shell/ShortcutsDialog";
import { TaskDialog } from "./components/task/TaskDialog";
import { TaskPanel } from "./components/task/TaskPanel";
import { ConfirmDialog } from "./components/ui/Dialog";
import { Toasts } from "./components/ui/Toast";
import { inTauri } from "./lib/api";
import { navigate, type Page, useRoute } from "./lib/router";
import { SettingsProvider } from "./lib/settings";
import { useShortcuts } from "./lib/shortcuts";
import { refreshData } from "./lib/store";
import { newHabit, newTask, openHabit, openTask, ui } from "./lib/ui";
import { About } from "./pages/About";
import { Analytics } from "./pages/Analytics";
import { Calendar } from "./pages/Calendar";
import { Focus } from "./pages/Focus";
import { Habits } from "./pages/Habits";
import { Overview } from "./pages/Overview";
import { Projects } from "./pages/Projects";
import { Settings } from "./pages/Settings";
import { Tasks } from "./pages/Tasks";

const PAGES: Record<Page, () => React.ReactNode> = {
  overview: Overview,
  tasks: Tasks,
  projects: Projects,
  habits: Habits,
  calendar: Calendar,
  analytics: Analytics,
  focus: Focus,
  settings: Settings,
  about: About,
};

function runShellAction(action: string) {
  if (action === "new-task") newTask();
  else if (action === "new-habit") newHabit();
  else if (action === "start-focus") navigate("focus");
}

/** Connects the interface to the desktop shell: launcher actions, and changes made outside the window. */
function useShell() {
  useEffect(() => {
    if (!inTauri) return;
    const stops = [
      listen("data-changed", refreshData),
      listen<string>("shell-action", (event) => runShellAction(event.payload)),
      listen<{ taskId: number | null; habitId: number | null; kind: string }>("open-notice", ({ payload }) => {
        if (payload.kind === "task" && payload.taskId) openTask(payload.taskId);
        else if (payload.kind === "habit" && payload.habitId) openHabit(payload.habitId);
        else if (payload.kind === "focus") navigate("focus");
      }),
    ];
    void invoke<string | null>("interface_ready").then((action) => {
      if (action) runShellAction(action);
    });
    return () => stops.forEach((stop) => void stop.then((unlisten) => unlisten()));
  }, []);
}

function Layout() {
  const { page } = useRoute();
  const state = ui.use();
  const scroller = useRef<HTMLElement>(null);
  const Current = PAGES[page];
  useShortcuts();
  useShell();

  useEffect(() => {
    scroller.current?.scrollTo(0, 0);
  }, [page]);

  return (
    <div className="flex h-full">
      <AppSidebar />
      <div className="relative flex min-w-0 flex-1">
        <div className="flex min-w-0 flex-1 flex-col">
          <AppToolbar />
          <main ref={scroller} className="relative min-h-0 flex-1 overflow-y-auto px-8 pb-10 pt-3">
            <Current key={page} />
          </main>
        </div>
        {state.taskPanel !== null && <TaskPanel key={state.taskPanel} id={state.taskPanel} />}
      </div>

      {state.taskDialog && <TaskDialog defaults={state.taskDialog} />}
      {state.habitDialog && <HabitDialog {...state.habitDialog} />}
      {state.habitDetail !== null && <HabitDetail id={state.habitDetail} />}
      {state.projectDialog && <ProjectDialog {...state.projectDialog} />}
      {state.palette && <CommandPalette mode={state.palette} />}
      {state.shortcuts && <ShortcutsDialog />}
      <ConfirmDialog />
      <Toasts />
    </div>
  );
}

export function App() {
  return (
    <SettingsProvider>
      <Layout />
    </SettingsProvider>
  );
}
