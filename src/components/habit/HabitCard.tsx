import { Ban, Check, Flame, ListChecks, Minus, Plus, Undo2 } from "lucide-react";
import { useRef, useState } from "react";

import { formatWeekday } from "../../lib/date";
import { iconFor, softStyle } from "../../lib/palette";
import { act } from "../../lib/store";
import type { DayState, Habit } from "../../lib/types";
import { openHabit } from "../../lib/ui";
import { Button, IconButton } from "../ui/Button";
import { Checkbox, DoneCheck, Input } from "../ui/Form";
import { Popover } from "../ui/Popover";

const undo = (habit: Habit) => act("habits.undo", { id: habit.id });

function isMet(habit: Habit) {
  if (habit.kind === "quit") return !habit.today.slipped;
  return habit.today.state === "done";
}

export function frequencyLabel(habit: Pick<Habit, "frequency" | "weekdays" | "weeklyTarget">) {
  if (habit.frequency === "daily") return "Every day";
  if (habit.frequency === "weekly_target") {
    return habit.weeklyTarget === 1 ? "Once a week" : `${habit.weeklyTarget} times a week`;
  }
  const names = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
  const days = habit.weekdays;
  if (days.length === 5 && days.every((day) => day < 5)) return "Weekdays";
  return days.map((day) => names[day]).join(", ");
}

export function streakLabel(habit: Pick<Habit, "stats">) {
  const { currentStreak: count, streakUnit: unit } = habit.stats;
  return `${count} ${count === 1 ? unit.slice(0, -1) : unit}`;
}

function QuantityControl({ habit }: { habit: Habit }) {
  const target = habit.quantityTarget ?? 1;
  const value = habit.today.value ?? 0;
  const step = target >= 10 ? 1 : target >= 3 ? 0.5 : 0.25;
  const [editing, setEditing] = useState(false);
  const [text, setText] = useState("");
  const anchor = useRef<HTMLButtonElement>(null);
  const log = (next: number) => act("habits.log", { id: habit.id, value: Math.max(0, next) });
  const summary = `${value} of ${target}${habit.unit ? ` ${habit.unit}` : ""}`;

  return (
    <div className="flex shrink-0 items-center gap-0.5">
      <IconButton size="sm" icon={Minus} label={`Reduce ${habit.name}`} disabled={value <= 0} onClick={() => void log(value - step)} />
      <button
        ref={anchor}
        type="button"
        aria-label={`${habit.name}: ${summary}. Enter an amount`}
        onClick={() => {
          setText(value ? String(value) : "");
          setEditing(true);
        }}
        className="interactive tabular min-w-12 rounded-sm px-1 py-1 text-center text-label font-medium"
      >
        {value}
        <span className="text-ink-2">/{target}</span>
      </button>
      <IconButton size="sm" icon={Plus} label={`Add to ${habit.name}`} onClick={() => void log(value + step)} />
      {editing && (
        <Popover anchor={anchor} onClose={() => setEditing(false)} label={`Amount for ${habit.name}`} align="end" className="w-44 p-2">
          <form
            onSubmit={(event) => {
              event.preventDefault();
              if (text !== "" && !Number.isNaN(Number(text))) void log(Number(text));
              setEditing(false);
            }}
            className="flex gap-1.5"
          >
            <Input
              autoFocus
              type="number"
              min={0}
              step="any"
              value={text}
              aria-label={`Amount${habit.unit ? ` in ${habit.unit}` : ""}`}
              placeholder={habit.unit}
              onChange={(event) => setText(event.target.value)}
              className="h-8"
            />
            <Button size="sm" type="submit" variant="primary">
              Save
            </Button>
          </form>
        </Popover>
      )}
    </div>
  );
}

function ChecklistControl({ habit }: { habit: Habit }) {
  const [open, setOpen] = useState(false);
  const anchor = useRef<HTMLButtonElement>(null);
  const checked = habit.today.checkedItems;
  const done = habit.checklist.filter((item) => checked.includes(item.id)).length;
  const toggle = (id: number) => {
    const next = checked.includes(id) ? checked.filter((item) => item !== id) : [...checked, id];
    void act("habits.log", { id: habit.id, checkedItems: next });
  };
  return (
    <>
      <Button
        ref={anchor}
        size="sm"
        icon={done === habit.checklist.length ? Check : ListChecks}
        aria-label={`${habit.name}: ${done} of ${habit.checklist.length} done`}
        aria-expanded={open}
        onClick={() => setOpen((state) => !state)}
      >
        <span className="tabular">
          {done}/{habit.checklist.length}
        </span>
      </Button>
      {open && (
        <Popover anchor={anchor} onClose={() => setOpen(false)} label={habit.name} align="end" className="w-56 p-2">
          <ul className="flex flex-col gap-0.5">
            {habit.checklist.map((item) => (
              <li key={item.id} className="flex items-center gap-2 rounded-sm px-1.5 py-1">
                <Checkbox checked={checked.includes(item.id)} onChange={() => toggle(item.id)} label={item.title} />
                <span className="text-body">{item.title}</span>
              </li>
            ))}
          </ul>
        </Popover>
      )}
    </>
  );
}

function QuitControl({ habit }: { habit: Habit }) {
  if (habit.today.slipped) {
    return (
      <Button size="sm" icon={Undo2} onClick={() => void undo(habit)} aria-label={`Undo the slip for ${habit.name}`}>
        Undo
      </Button>
    );
  }
  return (
    <div className="flex shrink-0 items-center gap-1">
      <span className="inline-flex items-center gap-1 text-meta font-medium text-success">
        <Check size={13} aria-hidden />
        On track
      </span>
      <IconButton
        size="sm"
        icon={Ban}
        label={`Log a slip for ${habit.name}`}
        onClick={() => void act("habits.log", { id: habit.id, slip: true })}
      />
    </div>
  );
}

export function HabitControl({ habit }: { habit: Habit }) {
  if (habit.kind === "quantity") return <QuantityControl habit={habit} />;
  if (habit.kind === "checklist") return <ChecklistControl habit={habit} />;
  if (habit.kind === "quit") return <QuitControl habit={habit} />;
  const done = habit.today.state === "done";
  return (
    <DoneCheck
      checked={done}
      size={26}
      color={softStyle(habit.color).color as string}
      label={done ? `Undo ${habit.name}` : `Complete ${habit.name}`}
      onChange={() => void (done ? undo(habit) : act("habits.log", { id: habit.id }))}
    />
  );
}

const STRIP: Record<DayState, { className: string; label: string }> = {
  done: { className: "bg-accent", label: "Done" },
  partial: { className: "bg-[var(--heat-2)]", label: "Partly done" },
  missed: { className: "bg-danger/30", label: "Missed" },
  pending: { className: "bg-transparent ring-1 ring-inset ring-ink-3", label: "Due today" },
  not_scheduled: { className: "bg-sunken", label: "Not scheduled" },
  inactive: { className: "bg-sunken", label: "Not started" },
};

export function HabitCard({ habit, detailed = false }: { habit: Habit; detailed?: boolean }) {
  const Icon = iconFor(habit.icon);
  const met = isMet(habit);
  return (
    <article
      className={`surface flex flex-col gap-3 p-3 transition-colors duration-200 ${met && habit.today.due ? "!bg-[color-mix(in_srgb,var(--color-accent)_7%,var(--card))]" : ""}`}
    >
      <div className="flex items-center gap-3">
        <button
          type="button"
          aria-label={`Open ${habit.name}`}
          onClick={() => openHabit(habit.id)}
          style={softStyle(habit.color)}
          className="inline-flex size-10 shrink-0 items-center justify-center rounded-md"
        >
          <Icon size={20} aria-hidden />
        </button>
        <div className="min-w-0 flex-1">
          <button type="button" onClick={() => openHabit(habit.id)} className="block max-w-full truncate text-left text-body font-medium">
            {habit.name}
          </button>
          <div className="mt-0.5 flex items-center gap-2 text-meta text-ink-2">
            <span className="tabular inline-flex shrink-0 items-center gap-1 whitespace-nowrap">
              <Flame size={13} aria-hidden className={habit.stats.currentStreak > 0 ? "text-accent-text" : ""} />
              <span className="sr-only">Current streak:</span>
              {streakLabel(habit)}
            </span>
            {detailed && <span className="truncate">{frequencyLabel(habit)}</span>}
          </div>
        </div>
        <HabitControl habit={habit} />
      </div>
      {detailed && (
        <div className="flex gap-1" role="img" aria-label={`Last seven days: ${habit.recent.map((day) => STRIP[day.state].label).join(", ")}`}>
          {habit.recent.map((day) => (
            <div key={day.date} className="flex flex-1 flex-col items-center gap-1">
              <div title={STRIP[day.state].label} className={`h-1.5 w-full rounded-full ${STRIP[day.state].className}`} />
              <span aria-hidden className="text-[10px] leading-3 text-ink-2">
                {formatWeekday(day.date).slice(0, 1)}
              </span>
            </div>
          ))}
        </div>
      )}
    </article>
  );
}
