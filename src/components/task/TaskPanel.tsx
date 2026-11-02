import { Archive, ArchiveRestore, Check, Copy, RotateCcw, Target, Trash2, X } from "lucide-react";
import { useEffect, useState } from "react";

import { formatInstant } from "../../lib/date";
import { useSettings } from "../../lib/settings";
import { mutate, useRpc } from "../../lib/store";
import {
  archiveTask,
  deleteTask,
  duplicateTask,
  focusOnTask,
  restoreTask,
  toggleComplete,
  toTaskInput,
} from "../../lib/taskActions";
import type { Habit, Priority, Task, TaskInput, TaskStatus } from "../../lib/types";
import { openTask } from "../../lib/ui";
import { DeadlineBadge } from "../Badges";
import { Button, IconButton } from "../ui/Button";
import { DatePicker } from "../ui/DatePicker";
import { Field, Input, Textarea } from "../ui/Form";
import { type Option, Select } from "../ui/Select";
import { TimePicker } from "../ui/TimePicker";
import { CategoryInput, ReminderEditor, RepeatEditor, TagInput } from "./fields";
import { SubtaskList } from "./SubtaskList";
import { PRIORITY_OPTIONS, useProjectOptions } from "./TaskDialog";

const STATUS_OPTIONS: Option<TaskStatus>[] = [
  { value: "todo", label: "To do" },
  { value: "in_progress", label: "In progress" },
  { value: "completed", label: "Completed" },
  { value: "archived", label: "Archived" },
];

/** Every edit saves as it is made, so there is no Save button to forget. */
function Editor({ task }: { task: Task }) {
  const { confirmDelete } = useSettings();
  const projects = useProjectOptions();
  const { data: habits } = useRpc<Habit[]>("habits.list");
  const [draft, setDraft] = useState<TaskInput>(() => toTaskInput(task));
  const [revision, setRevision] = useState(task.updatedAt);
  if (task.updatedAt !== revision) {
    setRevision(task.updatedAt);
    setDraft(toTaskInput(task));
  }

  const save = (patch: Partial<TaskInput>) => {
    const next = { ...draft, ...patch };
    setDraft(next);
    void mutate("tasks.update", { id: task.id, ...next, title: next.title.trim() || task.title }).catch(() => {
      setDraft(toTaskInput(task));
    });
  };

  const changeStatus = (status: TaskStatus) => {
    if (status === task.status) return;
    if (task.status === "archived") {
      void restoreTask(task.id);
    } else if (status === "archived") {
      void archiveTask(task.id);
    } else if (status === "completed" || task.status === "completed") {
      void toggleComplete(task);
    } else {
      save({ status });
    }
  };

  const habitOptions: Option<string>[] = [
    { value: "", label: "No habit" },
    ...(habits ?? []).map((habit) => ({ value: String(habit.id), label: habit.name })),
  ];
  const completed = task.status === "completed";
  const archived = task.status === "archived";

  return (
    <>
      <div className="flex-1 overflow-y-auto px-5 pb-5">
        <Textarea
          value={draft.title}
          aria-label="Task title"
          minRows={1}
          onChange={(event) => setDraft({ ...draft, title: event.target.value })}
          onBlur={() => draft.title.trim() !== task.title && save({})}
          className="-mx-3 !w-[calc(100%+24px)] !bg-transparent text-section font-semibold focus:!bg-sunken"
        />
        <div className="mb-4 mt-1 flex min-h-6 items-center gap-2">
          <DeadlineBadge task={task} />
        </div>

        <div className="grid grid-cols-2 gap-3">
          <Field label="Status">
            {() => <Select label="Status" value={task.status} options={STATUS_OPTIONS} onChange={changeStatus} className="w-full" />}
          </Field>
          <Field label="Priority">
            {() => (
              <Select<Priority>
                label="Priority"
                value={draft.priority}
                options={PRIORITY_OPTIONS}
                onChange={(priority) => save({ priority })}
                className="w-full"
              />
            )}
          </Field>
          <Field label="Deadline">
            {() => (
              <DatePicker
                value={draft.dueDate}
                onChange={(dueDate) =>
                  save({
                    dueDate,
                    dueTime: dueDate ? draft.dueTime : null,
                    recurrence: dueDate ? draft.recurrence : null,
                    reminders: dueDate ? draft.reminders : draft.reminders.filter((item) => item.kind === "at"),
                  })
                }
                label="Deadline"
                className="w-full"
              />
            )}
          </Field>
          <Field label="Time">
            {() => (
              <TimePicker
                value={draft.dueTime}
                onChange={(dueTime) => save({ dueTime })}
                label="Deadline time"
                disabled={!draft.dueDate}
                className="w-full"
              />
            )}
          </Field>
          <Field label="Start date">
            {() => <DatePicker value={draft.startDate} onChange={(startDate) => save({ startDate })} label="Start date" className="w-full" />}
          </Field>
          <Field label="Estimated minutes">
            {(id) => (
              <Input
                id={id}
                type="number"
                min={0}
                placeholder="None"
                value={draft.estimatedMinutes ?? ""}
                onChange={(event) =>
                  setDraft({ ...draft, estimatedMinutes: event.target.value ? Number(event.target.value) : null })
                }
                onBlur={() => draft.estimatedMinutes !== task.estimatedMinutes && save({})}
              />
            )}
          </Field>
          <Field label="Project">
            {() => (
              <Select
                label="Project"
                value={draft.projectId ? String(draft.projectId) : ""}
                options={projects}
                onChange={(value) => save({ projectId: value ? Number(value) : null })}
                className="w-full"
              />
            )}
          </Field>
          <Field label="Habit">
            {() => (
              <Select
                label="Associated habit"
                value={draft.habitId ? String(draft.habitId) : ""}
                options={habitOptions}
                onChange={(value) => save({ habitId: value ? Number(value) : null })}
                className="w-full"
              />
            )}
          </Field>
          <Field label="Category">
            {(id) => (
              <CategoryInput
                id={id}
                value={draft.category}
                onChange={(category) => setDraft({ ...draft, category })}
              />
            )}
          </Field>
          <Field label="Tags">{() => <TagInput value={draft.tags} onChange={(tags) => save({ tags })} />}</Field>
          <Field label="Repeat" className="col-span-2">
            {() => (
              <RepeatEditor
                value={draft.recurrence}
                onChange={(recurrence) => save({ recurrence })}
                disabled={!draft.dueDate}
              />
            )}
          </Field>
          <Field label="Reminders" className="col-span-2">
            {() => (
              <ReminderEditor
                value={draft.reminders}
                onChange={(reminders) => save({ reminders })}
                hasDeadline={Boolean(draft.dueDate)}
              />
            )}
          </Field>
          <Field label="Subtasks" className="col-span-2">
            {() => <SubtaskList task={task} />}
          </Field>
          <Field label="Description" className="col-span-2">
            {(id) => (
              <Textarea
                id={id}
                value={draft.description}
                placeholder="Add a description"
                onChange={(event) => setDraft({ ...draft, description: event.target.value })}
                onBlur={() => draft.description !== task.description && save({})}
              />
            )}
          </Field>
        </div>
        <p className="mt-4 text-meta text-ink-2">
          Created {formatInstant(task.createdAt)}
          {task.completedAt && `. Completed ${formatInstant(task.completedAt)}`}
        </p>
      </div>
      <div className="flex items-center gap-1 border-t border-line px-3 py-2">
        {archived ? (
          <Button size="sm" variant="primary" icon={ArchiveRestore} onClick={() => void restoreTask(task.id)}>
            Restore
          </Button>
        ) : (
          <Button
            size="sm"
            variant={completed ? "secondary" : "primary"}
            icon={completed ? RotateCcw : Check}
            onClick={() => void toggleComplete(task)}
          >
            {completed ? "Reopen" : "Complete"}
          </Button>
        )}
        <span className="flex-1" />
        {!completed && !archived && <IconButton size="sm" icon={Target} label="Focus on this task" onClick={() => focusOnTask(task.id)} />}
        <IconButton size="sm" icon={Copy} label="Duplicate" onClick={() => void duplicateTask(task.id)} />
        {!archived && <IconButton size="sm" icon={Archive} label="Archive" onClick={() => void archiveTask(task.id)} />}
        <IconButton size="sm" icon={Trash2} label="Delete" onClick={() => void deleteTask(task, confirmDelete)} />
      </div>
    </>
  );
}

export function TaskPanel({ id }: { id: number }) {
  const { data: task, error } = useRpc<Task>("tasks.get", { id });

  useEffect(() => {
    if (error?.code === "not_found") openTask(null);
  }, [error]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented) openTask(null);
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, []);

  return (
    <aside
      aria-label="Task details"
      className="animate-panel z-20 flex h-full w-[420px] shrink-0 flex-col border-l border-line bg-sidebar max-[1180px]:absolute max-[1180px]:right-0 max-[1180px]:top-0 max-[1180px]:shadow-[var(--shadow-float)]"
    >
      <div className="flex items-center justify-between px-5 pb-1 pt-3">
        <span className="text-label font-medium text-ink-2">Task details</span>
        <IconButton size="sm" icon={X} label="Close details" onClick={() => openTask(null)} />
      </div>
      {task && task.id === id && <Editor key={task.id} task={task} />}
    </aside>
  );
}
