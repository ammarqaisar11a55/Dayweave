import {
  Archive,
  ArchiveRestore,
  CalendarClock,
  Copy,
  Layers,
  ListChecks,
  MoreHorizontal,
  Pencil,
  Repeat,
  RotateCcw,
  Tag,
  Target,
  Trash2,
} from "lucide-react";

import { colorVar } from "../../lib/palette";
import { useSettings } from "../../lib/settings";
import {
  archiveTask,
  deleteTask,
  duplicateTask,
  focusOnTask,
  quickDates,
  reschedule,
  restoreTask,
  toggleComplete,
} from "../../lib/taskActions";
import type { Task } from "../../lib/types";
import { openTask, ui } from "../../lib/ui";
import { DeadlineBadge, PRIORITY, PriorityBadge } from "../Badges";
import { IconButton } from "../ui/Button";
import { DoneCheck } from "../ui/Form";
import { Menu, type MenuItem } from "../ui/Menu";

function menuItems(task: Task, confirmDelete: boolean): MenuItem[] {
  const remove: MenuItem = {
    label: "Delete",
    icon: Trash2,
    danger: true,
    onSelect: () => void deleteTask(task, confirmDelete),
  };
  if (task.status === "archived") {
    return [
      { label: "Restore", icon: ArchiveRestore, onSelect: () => void restoreTask(task.id) },
      { separator: true },
      remove,
    ];
  }
  const open = task.status !== "completed";
  return [
    { label: "Edit", icon: Pencil, onSelect: () => openTask(task.id) },
    ...(open
      ? [
          ...quickDates().map(
            ({ label, date }): MenuItem => ({
              label: `Move to ${label.toLowerCase()}`,
              icon: CalendarClock,
              onSelect: () => void reschedule(task.id, date, task.dueTime),
            }),
          ),
          { label: "Focus on this task", icon: Target, onSelect: () => focusOnTask(task.id) } as MenuItem,
        ]
      : [{ label: "Reopen", icon: RotateCcw, onSelect: () => void toggleComplete(task) } as MenuItem]),
    { label: "Duplicate", icon: Copy, onSelect: () => void duplicateTask(task.id) },
    { label: "Archive", icon: Archive, onSelect: () => void archiveTask(task.id) },
    { separator: true },
    remove,
  ];
}

export function TaskRow({ task, showProject = true }: { task: Task; showProject?: boolean }) {
  const { taskDensity, confirmDelete } = useSettings();
  const selected = ui.use().taskPanel === task.id;
  const compact = taskDensity === "compact";
  const completed = task.status === "completed";

  const details = [
    showProject && task.projectName && (
      <span key="project" className="inline-flex min-w-0 items-center gap-1.5">
        <span aria-hidden className="size-2 shrink-0 rounded-full" style={{ background: colorVar(task.projectColor) }} />
        <span className="truncate">{task.projectName}</span>
      </span>
    ),
    task.subtaskTotal > 0 && (
      <span key="subtasks" className="tabular inline-flex items-center gap-1">
        <ListChecks size={13} aria-hidden />
        <span className="sr-only">Subtasks</span>
        {task.subtaskDone}/{task.subtaskTotal}
      </span>
    ),
    task.recurrenceId && (
      <span key="repeat" className="inline-flex items-center gap-1">
        <Repeat size={13} aria-hidden />
        Repeats
      </span>
    ),
    task.category && (
      <span key="category" className="inline-flex items-center gap-1">
        <Layers size={13} aria-hidden />
        {task.category}
      </span>
    ),
    task.tags.length > 0 && (
      <span key="tags" className="inline-flex min-w-0 items-center gap-1">
        <Tag size={13} aria-hidden />
        <span className="truncate">{task.tags.map((tag) => tag.name).join(", ")}</span>
      </span>
    ),
  ].filter(Boolean);

  return (
    <div
      data-task-row
      onClick={() => openTask(task.id)}
      style={task.overdue ? { boxShadow: "inset 2px 0 0 var(--danger)" } : undefined}
      className={`group animate-settle flex items-center gap-3 px-4 transition-colors duration-150 ${compact ? "py-1.5" : "py-2.5"} ${selected ? "bg-accent-soft" : "interactive"}`}
    >
      <DoneCheck
        checked={completed}
        onChange={() => void toggleComplete(task)}
        label={completed ? `Reopen ${task.title}` : `Complete ${task.title}`}
        color={task.priority === "urgent" ? "var(--danger)" : task.priority === "high" ? "var(--warning)" : undefined}
        size={compact ? 18 : 22}
      />
      <div className="min-w-0 flex-1">
        <button
          type="button"
          data-task-title
          onClick={(event) => {
            event.stopPropagation();
            openTask(task.id);
          }}
          className={`block max-w-full truncate rounded-[4px] text-left text-body transition-colors duration-200 ${completed ? "text-ink-2 line-through decoration-ink-3" : "text-ink"}`}
        >
          {task.title}
        </button>
        {!compact && details.length > 0 && (
          <div className="mt-0.5 flex min-w-0 items-center gap-3 text-meta text-ink-2">{details}</div>
        )}
      </div>
      <DeadlineBadge task={task} />
      {task.priority !== "medium" && <PriorityBadge priority={task.priority} />}
      <span className="sr-only">{task.priority === "medium" ? `${PRIORITY.medium.label} priority` : ""}</span>
      <div onClick={(event) => event.stopPropagation()} className="opacity-0 transition-opacity duration-100 focus-within:opacity-100 group-hover:opacity-100">
        <Menu
          label={`Actions for ${task.title}`}
          items={menuItems(task, confirmDelete)}
          trigger={<IconButton size="sm" icon={MoreHorizontal} label="More actions" />}
        />
      </div>
    </div>
  );
}
