import { Bell, Plus, Repeat, X } from "lucide-react";
import { useRef, useState } from "react";

import { formatDateTime } from "../../lib/format";
import { useRpc } from "../../lib/store";
import type { Recurrence, Reminder } from "../../lib/types";
import { DatePicker } from "../ui/DatePicker";
import { Input } from "../ui/Form";
import { Menu, type MenuItem } from "../ui/Menu";
import { Popover } from "../ui/Popover";
import { Select } from "../ui/Select";
import { Button, IconButton } from "../ui/Button";
import { TimePicker } from "../ui/TimePicker";

const REMINDER_PRESETS = [
  { minutes: 0, label: "At deadline" },
  { minutes: 10, label: "10 minutes before" },
  { minutes: 30, label: "30 minutes before" },
  { minutes: 60, label: "1 hour before" },
  { minutes: 1440, label: "1 day before" },
];

export function reminderLabel(reminder: Reminder): string {
  if (reminder.kind === "at") return reminder.remindAt ? formatDateTime(reminder.remindAt) : "Custom";
  const preset = REMINDER_PRESETS.find((item) => item.minutes === reminder.offsetMinutes);
  if (preset) return preset.label;
  const minutes = reminder.offsetMinutes ?? 0;
  if (minutes % 1440 === 0) return `${minutes / 1440} days before`;
  if (minutes % 60 === 0) return `${minutes / 60} hours before`;
  return `${minutes} minutes before`;
}

export function ReminderEditor({
  value,
  onChange,
  hasDeadline,
}: {
  value: Reminder[];
  onChange: (value: Reminder[]) => void;
  hasDeadline: boolean;
}) {
  const [custom, setCustom] = useState(false);
  const [date, setDate] = useState<string | null>(null);
  const [time, setTime] = useState<string | null>("09:00");
  const anchor = useRef<HTMLDivElement>(null);

  const add = (reminder: Reminder) => {
    const exists = value.some(
      (item) => item.kind === reminder.kind && item.offsetMinutes === reminder.offsetMinutes && item.remindAt === reminder.remindAt,
    );
    if (!exists) onChange([...value, reminder]);
  };

  const presets: MenuItem[] = hasDeadline
    ? REMINDER_PRESETS.map(({ minutes, label }) => ({
        label,
        onSelect: () => add({ kind: "before", offsetMinutes: minutes, remindAt: null }),
      }))
    : [];

  return (
    <div className="flex flex-wrap items-center gap-1.5">
      {value.map((reminder, index) => (
        <span key={index} className="control inline-flex h-8 items-center gap-1.5 rounded-sm pl-2.5 pr-1 text-label">
          <Bell size={13} aria-hidden className="text-ink-2" />
          {reminderLabel(reminder)}
          <button
            type="button"
            aria-label={`Remove reminder: ${reminderLabel(reminder)}`}
            onClick={() => onChange(value.filter((_, position) => position !== index))}
            className="interactive inline-flex size-5 items-center justify-center rounded-[5px] text-ink-2"
          >
            <X size={12} aria-hidden />
          </button>
        </span>
      ))}
      <div ref={anchor}>
        <Menu
          label="Add a reminder"
          align="start"
          items={[
            ...presets,
            ...(presets.length ? ([{ separator: true }] as MenuItem[]) : []),
            { label: "Custom date and time", onSelect: () => setCustom(true) },
          ]}
          trigger={
            <Button size="sm" variant="ghost" icon={Plus}>
              Add reminder
            </Button>
          }
        />
      </div>
      {custom && (
        <Popover anchor={anchor} onClose={() => setCustom(false)} label="Custom reminder" className="w-64 p-3">
          <div className="flex flex-col gap-2">
            <DatePicker value={date} onChange={setDate} label="Reminder date" clearable={false} placeholder="Choose a date" />
            <TimePicker value={time} onChange={setTime} label="Reminder time" clearable={false} />
            <Button
              variant="primary"
              size="sm"
              disabled={!date || !time}
              onClick={() => {
                add({ kind: "at", offsetMinutes: null, remindAt: `${date}T${time}` });
                setCustom(false);
              }}
            >
              Add reminder
            </Button>
          </div>
        </Popover>
      )}
    </div>
  );
}

const WEEKDAYS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

export function WeekdayPicker({ value, onChange }: { value: number[]; onChange: (value: number[]) => void }) {
  return (
    <div role="group" aria-label="Days of the week" className="flex gap-1">
      {WEEKDAYS.map((day, index) => {
        const selected = value.includes(index);
        return (
          <button
            key={day}
            type="button"
            aria-pressed={selected}
            onClick={() => onChange(selected ? value.filter((item) => item !== index) : [...value, index].sort())}
            className={`h-8 flex-1 rounded-sm text-meta font-medium transition-colors duration-150 ${selected ? "bg-accent text-accent-ink" : "control"}`}
          >
            {day}
          </button>
        );
      })}
    </div>
  );
}

const UNITS = { daily: "day", weekly: "week", monthly: "month", yearly: "year" };

export function repeatLabel(rule: Recurrence): string {
  const unit = UNITS[rule.frequency];
  const base = rule.interval === 1 ? `Every ${unit}` : `Every ${rule.interval} ${unit}s`;
  if (rule.frequency === "weekly" && rule.weekdays.length) {
    return `${base} on ${rule.weekdays.map((day) => WEEKDAYS[day]).join(", ")}`;
  }
  return base;
}

export function RepeatEditor({
  value,
  onChange,
  disabled,
}: {
  value: Recurrence | null;
  onChange: (value: Recurrence | null) => void;
  disabled: boolean;
}) {
  const [editing, setEditing] = useState(false);
  const anchor = useRef<HTMLButtonElement>(null);
  const rule = value ?? { frequency: "weekly", interval: 1, weekdays: [], endDate: null, maxCount: null };
  const update = (patch: Partial<Recurrence>) => onChange({ ...rule, ...patch });
  const presets: MenuItem[] = (["daily", "weekly", "monthly", "yearly"] as const).map((frequency) => ({
    label: frequency[0]?.toUpperCase() + frequency.slice(1),
    onSelect: () => onChange({ frequency, interval: 1, weekdays: [], endDate: null, maxCount: null }),
  }));

  return (
    <div className="flex min-w-0 items-center gap-1">
      <Menu
        label="Repeat"
        align="start"
        items={[
          { label: "Does not repeat", onSelect: () => onChange(null) },
          { separator: true },
          ...presets,
          { separator: true },
          { label: "Custom", onSelect: () => (onChange(rule), setEditing(true)) },
        ]}
        trigger={
          <button
            ref={anchor}
            type="button"
            disabled={disabled}
            className={`control inline-flex h-9 min-w-0 flex-1 items-center gap-2 rounded-md px-3 text-left transition-colors duration-150 disabled:pointer-events-none disabled:opacity-45 ${value ? "" : "text-ink-3"}`}
          >
            <Repeat size={15} aria-hidden className="shrink-0 text-ink-2" />
            <span className={`truncate ${value ? "text-ink" : "text-ink-3"}`}>
              {value ? repeatLabel(value) : "Does not repeat"}
            </span>
          </button>
        }
      />
      {value && <IconButton icon={X} label="Stop repeating" onClick={() => onChange(null)} />}
      {editing && (
        <Popover anchor={anchor} onClose={() => setEditing(false)} label="Custom repeat" className="w-80 p-4">
          <div className="flex flex-col gap-3">
            <div className="flex items-center gap-2">
              <span className="text-label text-ink-2">Every</span>
              <Input
                type="number"
                min={1}
                max={999}
                aria-label="Repeat interval"
                value={rule.interval}
                onChange={(event) => update({ interval: Math.max(1, Number(event.target.value) || 1) })}
                className="w-20"
              />
              <Select
                label="Repeat unit"
                value={rule.frequency}
                onChange={(frequency) => update({ frequency })}
                className="flex-1"
                options={[
                  { value: "daily", label: rule.interval === 1 ? "day" : "days" },
                  { value: "weekly", label: rule.interval === 1 ? "week" : "weeks" },
                  { value: "monthly", label: rule.interval === 1 ? "month" : "months" },
                  { value: "yearly", label: rule.interval === 1 ? "year" : "years" },
                ]}
              />
            </div>
            {rule.frequency === "weekly" && (
              <WeekdayPicker value={rule.weekdays} onChange={(weekdays) => update({ weekdays })} />
            )}
            <div className="flex items-center gap-2">
              <span className="text-label text-ink-2">Ends</span>
              <DatePicker
                value={rule.endDate}
                onChange={(endDate) => update({ endDate })}
                label="End date"
                placeholder="Never"
                className="flex-1"
              />
            </div>
            <Button size="sm" variant="primary" onClick={() => setEditing(false)}>
              Done
            </Button>
          </div>
        </Popover>
      )}
    </div>
  );
}

export function TagInput({ value, onChange }: { value: string[]; onChange: (value: string[]) => void }) {
  const [text, setText] = useState("");
  const { data: known } = useRpc<{ name: string }[]>("tasks.tags");
  const suggestions = (known ?? [])
    .map((tag) => tag.name)
    .filter((name) => !value.includes(name) && text && name.toLowerCase().includes(text.toLowerCase()))
    .slice(0, 5);

  const commit = (name: string) => {
    const clean = name.trim().replace(/^#/, "");
    if (clean && !value.some((tag) => tag.toLowerCase() === clean.toLowerCase())) onChange([...value, clean]);
    setText("");
  };

  return (
    <div>
      <div className="flex min-h-9 flex-wrap items-center gap-1.5 rounded-md bg-sunken px-2 py-1 focus-within:ring-2 focus-within:ring-accent/70">
        {value.map((tag) => (
          <span key={tag} className="control inline-flex h-6 items-center gap-1 rounded-[6px] pl-2 pr-0.5 text-meta">
            {tag}
            <button
              type="button"
              aria-label={`Remove tag ${tag}`}
              onClick={() => onChange(value.filter((item) => item !== tag))}
              className="inline-flex size-4 items-center justify-center rounded-[4px] text-ink-2 hover:text-ink"
            >
              <X size={11} aria-hidden />
            </button>
          </span>
        ))}
        <input
          value={text}
          aria-label="Add a tag"
          placeholder={value.length ? "" : "Add a tag"}
          onChange={(event) => setText(event.target.value)}
          onKeyDown={(event) => {
            if ((event.key === "Enter" || event.key === ",") && text.trim()) {
              event.preventDefault();
              commit(text);
            } else if (event.key === "Backspace" && !text && value.length) {
              onChange(value.slice(0, -1));
            }
          }}
          onBlur={() => text.trim() && commit(text)}
          className="h-6 min-w-16 flex-1 bg-transparent text-body text-ink outline-none placeholder:text-ink-3"
        />
      </div>
      {suggestions.length > 0 && (
        <div className="mt-1 flex flex-wrap gap-1">
          {suggestions.map((name) => (
            <button
              key={name}
              type="button"
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => commit(name)}
              className="interactive rounded-[6px] px-2 py-0.5 text-meta text-ink-2"
            >
              {name}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export function CategoryInput({
  value,
  onChange,
  id,
}: {
  value: string | null;
  onChange: (value: string | null) => void;
  id?: string;
}) {
  const { data: categories } = useRpc<string[]>("tasks.categories");
  return (
    <>
      <Input
        id={id}
        list="task-categories"
        value={value ?? ""}
        placeholder="None"
        onChange={(event) => onChange(event.target.value || null)}
      />
      <datalist id="task-categories">
        {categories?.map((category) => <option key={category} value={category} />)}
      </datalist>
    </>
  );
}
