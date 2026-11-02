import { FolderKanban, Hash, Keyboard, type LucideIcon, Moon, PanelLeft, Plus, Repeat2, Search, Target, CheckCircle2 } from "lucide-react";
import { type KeyboardEvent, useEffect, useMemo, useRef, useState } from "react";

import { rpc } from "../../lib/api";
import { navigate } from "../../lib/router";
import { setSetting, useSettings } from "../../lib/settings";
import type { SearchResults } from "../../lib/types";
import { closePalette, newHabit, newProject, newTask, openHabit, openTask, showShortcuts } from "../../lib/ui";
import { Dialog } from "../ui/Dialog";
import { NAVIGATION } from "./AppSidebar";

interface Entry {
  id: string;
  group: string;
  label: string;
  icon: LucideIcon;
  hint?: string;
  run: () => void;
}

function useCommands(): Entry[] {
  const { theme, sidebarCollapsed } = useSettings();
  return useMemo(() => {
    const command = (label: string, icon: LucideIcon, run: () => void, hint?: string): Entry => ({
      id: label,
      group: "Commands",
      label,
      icon,
      run,
      hint,
    });
    return [
      command("Create task", Plus, () => newTask(), "Ctrl+N"),
      command("Create habit", Plus, newHabit, "Ctrl+Shift+N"),
      command("Create project", Plus, newProject),
      command("Start focus session", Target, () => navigate("focus")),
      ...NAVIGATION.map(({ page, label, icon }) => command(`Open ${label}`, icon, () => navigate(page))),
      command("Open Settings", PanelLeft, () => navigate("settings"), "Ctrl+,"),
      command(theme === "dark" ? "Use light theme" : "Use dark theme", Moon, () =>
        void setSetting("theme", theme === "dark" ? "light" : "dark"),
      ),
      command(sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar", PanelLeft, () =>
        void setSetting("sidebarCollapsed", !sidebarCollapsed),
      "Ctrl+B"),
      command("Show keyboard shortcuts", Keyboard, () => showShortcuts(true), "F1"),
    ];
  }, [theme, sidebarCollapsed]);
}

function resultEntries(results: SearchResults | null): Entry[] {
  if (!results) return [];
  return [
    ...results.tasks.map((task): Entry => ({
      id: `task-${task.id}`,
      group: "Tasks",
      label: task.title,
      icon: CheckCircle2,
      run: () => openTask(task.id),
    })),
    ...results.habits.map((habit): Entry => ({
      id: `habit-${habit.id}`,
      group: "Habits",
      label: habit.name,
      icon: Repeat2,
      run: () => openHabit(habit.id),
    })),
    ...results.projects.map((project): Entry => ({
      id: `project-${project.id}`,
      group: "Projects",
      label: project.name,
      icon: FolderKanban,
      run: () => navigate("tasks", { project: String(project.id) }),
    })),
    ...results.tags.map((tag): Entry => ({
      id: `tag-${tag.id}`,
      group: "Tags",
      label: tag.name,
      icon: Hash,
      run: () => navigate("tasks", { tag: tag.name }),
    })),
  ];
}

export function CommandPalette({ mode }: { mode: "commands" | "search" }) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResults | null>(null);
  const [active, setActive] = useState(0);
  const list = useRef<HTMLDivElement>(null);
  const commands = useCommands();
  const text = query.trim().toLowerCase();

  useEffect(() => {
    if (!text) {
      setResults(null);
      return;
    }
    let current = true;
    const timer = window.setTimeout(() => {
      void rpc<SearchResults>("search.query", { query: text }).then((value) => {
        if (current) setResults(value);
      });
    }, 90);
    return () => {
      current = false;
      window.clearTimeout(timer);
    };
  }, [text]);

  const entries = useMemo(() => {
    const matching = commands.filter((entry) => entry.label.toLowerCase().includes(text));
    if (mode === "search") return text ? [...resultEntries(results), ...matching] : [];
    return text ? [...matching, ...resultEntries(results)] : matching;
  }, [commands, mode, results, text]);

  useEffect(() => setActive(0), [text, results]);
  useEffect(() => {
    list.current?.querySelector("[aria-selected=true]")?.scrollIntoView({ block: "nearest" });
  }, [active]);

  const run = (entry: Entry | undefined) => {
    if (!entry) return;
    closePalette();
    entry.run();
  };

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === "ArrowDown") setActive((index) => Math.min(entries.length - 1, index + 1));
    else if (event.key === "ArrowUp") setActive((index) => Math.max(0, index - 1));
    else if (event.key === "Enter") run(entries[active]);
    else return;
    event.preventDefault();
  };

  let lastGroup = "";
  return (
    <Dialog title={mode === "search" ? "Search" : "Command palette"} onClose={closePalette} width="max-w-[560px]" bare align="top">
      <div className="flex items-center gap-3 border-b border-line px-4">
        <Search size={18} aria-hidden className="text-ink-2" />
        <input
          data-autofocus
          role="combobox"
          aria-expanded
          aria-controls="palette-list"
          aria-activedescendant={entries[active]?.id}
          aria-label={mode === "search" ? "Search" : "Search or type a command"}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={onKeyDown}
          placeholder={mode === "search" ? "Search tasks, habits, projects and tags" : "Type a command or search"}
          className="h-13 flex-1 bg-transparent text-heading text-ink outline-none placeholder:text-ink-3"
        />
      </div>
      <div ref={list} id="palette-list" role="listbox" aria-label="Results" className="max-h-[380px] overflow-y-auto p-1.5">
        {entries.map((entry, index) => {
          const heading = entry.group !== lastGroup ? entry.group : null;
          lastGroup = entry.group;
          const Icon = entry.icon;
          return (
            <div key={entry.id}>
              {heading && <p className="px-2.5 pb-1 pt-2 text-meta font-medium text-ink-2">{heading}</p>}
              <div
                id={entry.id}
                role="option"
                aria-selected={index === active}
                onMouseMove={() => setActive(index)}
                onClick={() => run(entry)}
                style={index === active ? { background: "var(--hover)" } : undefined}
                className="flex h-9 items-center gap-2.5 rounded-sm px-2.5 text-body"
              >
                <Icon size={16} aria-hidden className="shrink-0 text-ink-2" />
                <span className="flex-1 truncate">{entry.label}</span>
                {entry.hint && <span className="text-meta text-ink-2">{entry.hint}</span>}
              </div>
            </div>
          );
        })}
        {text && entries.length === 0 && <p className="px-3 py-6 text-center text-label text-ink-2">No results for "{query}"</p>}
        {!text && mode === "search" && <p className="px-3 py-6 text-center text-label text-ink-2">Type to search everything in Dayweave</p>}
      </div>
    </Dialog>
  );
}
