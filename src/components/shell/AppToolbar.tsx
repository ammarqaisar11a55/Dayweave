import { Plus, Search } from "lucide-react";

import { newTask, openPalette } from "../../lib/ui";
import { Button } from "../ui/Button";
import { NotificationCenter } from "./NotificationCenter";

export function AppToolbar() {
  return (
    <div className="flex h-14 shrink-0 items-center gap-3 px-8">
      <button
        type="button"
        onClick={() => openPalette("search")}
        aria-label="Search tasks, habits and projects"
        className="group flex h-9 w-full max-w-md items-center gap-2 rounded-md bg-card px-3 text-body text-ink-2 shadow-[inset_0_0_0_1px_var(--line)] transition-colors duration-150 hover:text-ink"
      >
        <Search size={16} aria-hidden />
        <span className="flex-1 text-left">Search</span>
        <kbd className="rounded-[5px] bg-sunken px-1.5 py-0.5 font-sans text-meta text-ink-2">Ctrl K</kbd>
      </button>
      <span className="flex-1" />
      <NotificationCenter />
      <Button variant="primary" icon={Plus} onClick={() => newTask()}>
        New task
      </Button>
    </div>
  );
}
