import { confirm } from "../components/ui/Dialog";
import { addDays, formatShortDate, today } from "./date";
import { navigate } from "./router";
import { act, mutate, showToast } from "./store";
import { openTask } from "./ui";
import type { Reminder, Task, TaskInput } from "./types";

export function toTaskInput(task: Task): TaskInput {
  return {
    title: task.title,
    description: task.description,
    status: task.status,
    priority: task.priority,
    category: task.category,
    projectId: task.projectId,
    habitId: task.habitId,
    startDate: task.startDate,
    dueDate: task.dueDate,
    dueTime: task.dueTime,
    estimatedMinutes: task.estimatedMinutes,
    tags: task.tags.map((tag) => tag.name),
    reminders: task.reminders.map(
      ({ kind, offsetMinutes, remindAt }): Reminder => ({ kind, offsetMinutes, remindAt }),
    ),
    recurrence: task.recurrence,
  };
}

export async function toggleComplete(task: Pick<Task, "id" | "status">) {
  if (task.status === "completed") {
    await act("tasks.reopen", { id: task.id }, "Task reopened");
    return;
  }
  const result = await mutate<{ next: Task | null }>("tasks.complete", { id: task.id }).catch(() => null);
  if (!result) return;
  const message = result.next?.dueDate
    ? `Task completed. Next on ${formatShortDate(result.next.dueDate)}`
    : "Task completed";
  showToast(message, { action: { label: "Undo", run: () => void act("tasks.reopen", { id: task.id }) } });
}

export function reschedule(id: number, dueDate: string | null, dueTime: string | null = null) {
  const label = dueDate ? `Moved to ${formatShortDate(dueDate)}` : "Date removed";
  return act("tasks.reschedule", { id, dueDate, dueTime }, label);
}

export const quickDates = () => [
  { label: "Today", date: today() },
  { label: "Tomorrow", date: addDays(today(), 1) },
  { label: "Next week", date: addDays(today(), 7) },
];

export function duplicateTask(id: number) {
  return mutate<Task>("tasks.duplicate", { id }).then(
    (copy) => {
      showToast("Task duplicated");
      openTask(copy.id);
    },
    () => undefined,
  );
}

export function archiveTask(id: number) {
  return act("tasks.archive", { id }).then(() =>
    showToast("Task archived", { action: { label: "Undo", run: () => void act("tasks.restore", { id }) } }),
  );
}

export function restoreTask(id: number) {
  return act("tasks.restore", { id }, "Task restored");
}

export async function deleteTask(task: Pick<Task, "id" | "title">, confirmFirst: boolean) {
  if (confirmFirst) {
    const confirmed = await confirm({
      title: "Delete this task?",
      body: `"${task.title}" and its subtasks will be deleted. This can't be undone. Archive it instead to keep it out of the way.`,
      confirmLabel: "Delete task",
    });
    if (!confirmed) return;
  }
  openTask(null);
  await act("tasks.delete", { id: task.id }, "Task deleted");
}

export function focusOnTask(id: number) {
  navigate("focus", { task: String(id) });
}
