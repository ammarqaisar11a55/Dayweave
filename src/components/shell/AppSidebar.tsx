import {
  BarChart3,
  CalendarDays,
  CheckCircle2,
  FolderKanban,
  Info,
  LayoutDashboard,
  type LucideIcon,
  PanelLeftClose,
  PanelLeftOpen,
  Repeat2,
  Settings,
  Timer,
} from "lucide-react";

import { formatClock } from "../../lib/date";
import { useCountdown } from "../../lib/focus";
import { navigate, type Page, useRoute } from "../../lib/router";
import { setSetting, useSettings } from "../../lib/settings";
import { useRpc } from "../../lib/store";
import type { AppInfo, Badges } from "../../lib/types";
import { Tooltip } from "../ui/Tooltip";

interface Item {
  page: Page;
  label: string;
  icon: LucideIcon;
}

export const NAVIGATION: Item[] = [
  { page: "overview", label: "Overview", icon: LayoutDashboard },
  { page: "tasks", label: "Tasks", icon: CheckCircle2 },
  { page: "projects", label: "Projects", icon: FolderKanban },
  { page: "habits", label: "Habits", icon: Repeat2 },
  { page: "calendar", label: "Calendar", icon: CalendarDays },
  { page: "analytics", label: "Analytics", icon: BarChart3 },
  { page: "focus", label: "Focus", icon: Timer },
];

const SECONDARY: Item[] = [
  { page: "settings", label: "Settings", icon: Settings },
  { page: "about", label: "About", icon: Info },
];

function NavLink({ item, collapsed, trailing }: { item: Item; collapsed: boolean; trailing?: React.ReactNode }) {
  const { page } = useRoute();
  const active = page === item.page;
  const Icon = item.icon;
  return (
    <Tooltip text={item.label} placement="right" disabled={!collapsed}>
      <a
        href={`#/${item.page}`}
        aria-label={collapsed ? item.label : undefined}
        aria-current={active ? "page" : undefined}
        className={`relative flex h-9 items-center gap-3 overflow-hidden rounded-md px-[11px] text-body transition-colors duration-150 ${active ? "bg-accent-soft font-medium text-accent-text" : "interactive text-ink-2 hover:text-ink"}`}
      >
        <Icon size={18} aria-hidden className="shrink-0" />
        {!collapsed && <span className="flex-1 truncate">{item.label}</span>}
        {!collapsed && trailing}
      </a>
    </Tooltip>
  );
}

function Mark() {
  return (
    <svg viewBox="0 0 512 512" aria-hidden className="size-7 shrink-0">
      <rect x="24" y="24" width="464" height="464" rx="104" fill="#E95420" />
      <g fill="#FFF3EC">
        <rect x="112" y="112" width="190" height="84" rx="42" />
        <rect x="316" y="112" width="84" height="190" rx="42" />
        <rect x="210" y="316" width="190" height="84" rx="42" />
        <rect x="112" y="210" width="84" height="190" rx="42" />
      </g>
    </svg>
  );
}

export function AppSidebar() {
  const { sidebarCollapsed: collapsed } = useSettings();
  const { data: badges } = useRpc<Badges>("overview.badges");
  const { data: info } = useRpc<AppInfo>("app.info");
  const focus = badges?.focus ?? null;
  const remaining = useCountdown(focus);
  const toggle = () => void setSetting("sidebarCollapsed", !collapsed);

  const trailing = (page: Page) => {
    if (page === "tasks" && badges?.overdue) {
      return (
        <span className="tabular text-meta font-medium text-danger">
          {badges.overdue}
          <span className="sr-only"> overdue</span>
        </span>
      );
    }
    if (page === "focus" && focus) {
      return (
        <span className="tabular text-meta font-medium text-accent-text">
          {focus.status === "paused" ? "Paused" : formatClock(remaining)}
        </span>
      );
    }
    return null;
  };

  return (
    <nav
      aria-label="Main"
      style={{ width: collapsed ? 64 : 224 }}
      className="flex h-full shrink-0 flex-col bg-sidebar px-3 py-3 transition-[width] duration-200 ease-weave [box-shadow:inset_-1px_0_0_var(--line)]"
    >
      <button
        type="button"
        onClick={() => navigate("overview")}
        aria-label="Dayweave, go to Overview"
        className="mb-5 flex h-10 items-center gap-2.5 rounded-md px-[6px]"
      >
        <Mark />
        {!collapsed && <span className="text-heading font-semibold tracking-tight">Dayweave</span>}
      </button>

      <div className="flex flex-1 flex-col gap-0.5">
        {NAVIGATION.map((item) => (
          <NavLink key={item.page} item={item} collapsed={collapsed} trailing={trailing(item.page)} />
        ))}
      </div>

      <div className="flex flex-col gap-0.5">
        {SECONDARY.map((item) => (
          <NavLink key={item.page} item={item} collapsed={collapsed} />
        ))}
        <Tooltip text={collapsed ? "Expand sidebar (Ctrl+B)" : "Collapse sidebar (Ctrl+B)"} placement="right">
          <button
            type="button"
            onClick={toggle}
            aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
            aria-expanded={!collapsed}
            className="interactive flex h-9 items-center gap-3 overflow-hidden rounded-md px-[11px] text-ink-2 hover:text-ink"
          >
            {collapsed ? <PanelLeftOpen size={18} aria-hidden className="shrink-0" /> : <PanelLeftClose size={18} aria-hidden className="shrink-0" />}
            {!collapsed && <span className="flex-1 truncate text-left text-body">Collapse</span>}
          </button>
        </Tooltip>
        {!collapsed && info && <p className="tabular mt-2 px-[11px] text-meta text-ink-3">Version {info.version}</p>}
      </div>
    </nav>
  );
}
