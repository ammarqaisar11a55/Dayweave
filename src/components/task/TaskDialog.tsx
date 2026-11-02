import { ChevronDown } from "lucide-react";
import { type FormEvent, useState } from "react";

import { useSettings } from "../../lib/settings";
import { mutate, showToast, useRpc } from "../../lib/store";
import type { Priority, Project, Recurrence, Reminder, Task, TaskInput } from "../../lib/types";
import { closeTaskDialog } from "../../lib/ui";
import { colorVar } from "../../lib/palette";
import { PRIORITY } from "../Badges";
import { Button } from "../ui/Button";
import { DatePicker } from "../ui/DatePicker";
import { Dialog } from "../ui/Dialog";
import { Field, Input, Textarea } from "../ui/Form";
import { type Option, Select } from "../ui/Select";
import { TimePicker } from "../ui/TimePicker";
import { CategoryInput, ReminderEditor, RepeatEditor, TagInput } from "./fields";

export const PRIORITY_OPTIONS: Option<Priority>[] = (["low", "medium", "high", "urgent"] as const).map((value) => ({
  value,
  label: PRIORITY[value].label,
  icon: PRIORITY[value].icon,
}));

export function useProjectOptions(): Option<string>[] {
  const { data } = useRpc<Project[]>("projects.list");
  return [
    { value: "", label: "No project" },
    ...(data ?? []).map((project) => ({
      value: String(project.id),
      label: project.name,
      decoration: <span aria-hidden className="size-2 rounded-full" style={{ background: colorVar(project.color) }} />,
    })),
  ];
}

export function TaskDialog({ defaults }: { defaults: Partial<TaskInput> }) {
  const settings = useSettings();
  const [title, setTitle] = useState(defaults.title ?? "");
  const [description, setDescription] = useState(defaults.description ?? "");
  const [dueDate, setDueDate] = useState<string | null>(defaults.dueDate ?? null);
  const [dueTime, setDueTime] = useState<string | null>(defaults.dueTime ?? null);
  const [priority, setPriority] = useState<Priority>(defaults.priority ?? settings.defaultPriority);
  const [projectId, setProjectId] = useState(defaults.projectId ? String(defaults.projectId) : "");
  const [category, setCategory] = useState<string | null>(defaults.category ?? null);
  const [tags, setTags] = useState<string[]>(defaults.tags ?? []);
  const [startDate, setStartDate] = useState<string | null>(defaults.startDate ?? null);
  const [estimate, setEstimate] = useState("");
  const [reminders, setReminders] = useState<Reminder[] | null>(null);
  const [recurrence, setRecurrence] = useState<Recurrence | null>(null);
  const [more, setMore] = useState(false);
  const [saving, setSaving] = useState(false);
  const projects = useProjectOptions();

  // A deadline gets the default reminder until the person chooses their own.
  const defaultReminder: Reminder[] =
    dueDate && settings.defaultReminderMinutes !== null
      ? [{ kind: "before", offsetMinutes: settings.defaultReminderMinutes, remindAt: null }]
      : [];
  const effectiveReminders = reminders ?? defaultReminder;

  const save = async (keepOpen: boolean) => {
    if (saving) return;
    setSaving(true);
    const input: TaskInput = {
      title,
      description,
      priority,
      category,
      projectId: projectId ? Number(projectId) : null,
      habitId: defaults.habitId ?? null,
      startDate,
      dueDate,
      dueTime: dueDate ? dueTime : null,
      estimatedMinutes: estimate ? Number(estimate) : null,
      tags,
      reminders: effectiveReminders,
      recurrence: dueDate ? recurrence : null,
    };
    try {
      await mutate<Task>("tasks.create", input);
      showToast("Task added");
      if (keepOpen) {
        setTitle("");
        setDescription("");
      } else {
        closeTaskDialog();
      }
    } catch {
      // The toast has already explained what went wrong.
    } finally {
      setSaving(false);
    }
  };

  const submit = (event: FormEvent) => {
    event.preventDefault();
    void save(false);
  };

  return (
    <Dialog title="New task" onClose={closeTaskDialog} width="max-w-xl" align="top">
      <form
        onSubmit={submit}
        onKeyDown={(event) => {
          if (event.key === "Enter" && event.ctrlKey) {
            event.preventDefault();
            void save(true);
          }
        }}
        className="flex flex-col gap-4 pb-5"
      >
        <Input
          data-autofocus
          value={title}
          onChange={(event) => setTitle(event.target.value)}
          placeholder="What needs doing?"
          aria-label="Title"
          className="h-11 text-heading font-medium"
        />
        <Textarea
          value={description}
          onChange={(event) => setDescription(event.target.value)}
          placeholder="Add a description"
          aria-label="Description"
          minRows={2}
        />
        <div className="grid grid-cols-2 gap-3">
          <Field label="Deadline">
            {() => <DatePicker value={dueDate} onChange={setDueDate} label="Deadline" className="w-full" />}
          </Field>
          <Field label="Time">
            {() => <TimePicker value={dueTime} onChange={setDueTime} label="Deadline time" disabled={!dueDate} className="w-full" />}
          </Field>
          <Field label="Priority">
            {() => <Select label="Priority" value={priority} options={PRIORITY_OPTIONS} onChange={setPriority} className="w-full" />}
          </Field>
          <Field label="Project">
            {() => <Select label="Project" value={projectId} options={projects} onChange={setProjectId} className="w-full" />}
          </Field>
        </div>
        <Field label="Reminders">
          {() => <ReminderEditor value={effectiveReminders} onChange={setReminders} hasDeadline={Boolean(dueDate)} />}
        </Field>

        <button
          type="button"
          aria-expanded={more}
          onClick={() => setMore((state) => !state)}
          className="interactive -mx-1 inline-flex w-fit items-center gap-1 rounded-sm px-1 py-0.5 text-label font-medium text-ink-2"
        >
          <ChevronDown size={15} aria-hidden className={`transition-transform duration-150 ${more ? "rotate-180" : ""}`} />
          More options
        </button>
        {more && (
          <div className="animate-settle grid grid-cols-2 gap-3">
            <Field label="Repeat" className="col-span-2">
              {() => <RepeatEditor value={recurrence} onChange={setRecurrence} disabled={!dueDate} />}
            </Field>
            <Field label="Start date">
              {() => <DatePicker value={startDate} onChange={setStartDate} label="Start date" className="w-full" />}
            </Field>
            <Field label="Estimated minutes">
              {(id) => <Input id={id} type="number" min={0} value={estimate} onChange={(event) => setEstimate(event.target.value)} placeholder="None" />}
            </Field>
            <Field label="Category">{(id) => <CategoryInput id={id} value={category} onChange={setCategory} />}</Field>
            <Field label="Tags">{() => <TagInput value={tags} onChange={setTags} />}</Field>
          </div>
        )}

        <div className="flex items-center justify-between gap-2 pt-1">
          <span className="text-meta text-ink-2">Ctrl+Enter adds this and starts another</span>
          <div className="flex gap-2">
            <Button onClick={closeTaskDialog}>Cancel</Button>
            <Button type="submit" variant="primary" disabled={!title.trim() || saving}>
              Add task
            </Button>
          </div>
        </div>
      </form>
    </Dialog>
  );
}
