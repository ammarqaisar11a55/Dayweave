import { Archive, ArchiveRestore, Check, ChevronLeft, ChevronRight, Pencil, Trash2, X } from "lucide-react";
import { useEffect, useState } from "react";

import { addMonths, formatLongDate, formatMonth, formatWeekday, monthGrid, parseDate, today } from "../../lib/date";
import { percent } from "../../lib/format";
import { iconFor, softStyle } from "../../lib/palette";
import { useSettings } from "../../lib/settings";
import { act, useRpc } from "../../lib/store";
import type { DayState, Habit, HistoryDay } from "../../lib/types";
import { editHabit, openHabit } from "../../lib/ui";
import { Button, IconButton } from "../ui/Button";
import { confirm, Dialog } from "../ui/Dialog";
import { Input } from "../ui/Form";
import { frequencyLabel } from "./HabitCard";
import { Heatmap } from "./Heatmap";

const DAY_STYLE: Partial<Record<DayState, string>> = {
  done: "bg-accent text-accent-ink font-semibold",
  partial: "bg-accent-soft text-accent-text",
  missed: "text-danger bg-danger/10",
  pending: "ring-1 ring-inset ring-ink-3",
};

const DAY_LABEL: Record<DayState, string> = {
  done: "done",
  partial: "partly done",
  missed: "missed",
  pending: "due today",
  not_scheduled: "not scheduled",
  inactive: "not active",
};

function MonthHistory({ habit }: { habit: Habit }) {
  const { weekStart } = useSettings();
  const [month, setMonth] = useState(today());
  const [selected, setSelected] = useState<string | null>(null);
  const rows = monthGrid(month, weekStart);
  const from = rows[0]?.[0] ?? month;
  const to = rows[rows.length - 1]?.[6] ?? month;
  const { data } = useRpc<HistoryDay[]>("habits.history", { id: habit.id, from, to });
  const days = new Map((data ?? []).map((day) => [day.date, day]));
  const shownMonth = parseDate(month).getMonth();
  const entry = selected ? days.get(selected) : undefined;

  return (
    <div className="flex flex-wrap gap-6">
      <div className="w-[280px] shrink-0">
        <div className="mb-2 flex items-center justify-between">
          <span className="text-label font-semibold">{formatMonth(month)}</span>
          <div className="flex">
            <IconButton size="sm" icon={ChevronLeft} label="Previous month" onClick={() => setMonth(addMonths(month, -1))} />
            <IconButton size="sm" icon={ChevronRight} label="Next month" onClick={() => setMonth(addMonths(month, 1))} />
          </div>
        </div>
        <div className="grid grid-cols-7 gap-1">
          {rows[0]?.map((date) => (
            <span key={date} className="text-center text-meta text-ink-2">
              {formatWeekday(date).slice(0, 2)}
            </span>
          ))}
          {rows.flat().map((date) => {
            const day = days.get(date);
            const state = day?.state ?? "inactive";
            const outside = parseDate(date).getMonth() !== shownMonth;
            const clickable = state !== "inactive";
            return (
              <button
                key={date}
                type="button"
                disabled={!clickable}
                aria-label={`${formatLongDate(date)}: ${DAY_LABEL[state]}`}
                aria-pressed={selected === date}
                onClick={() => setSelected(date)}
                className={`tabular relative flex h-9 items-center justify-center rounded-sm text-label transition-colors duration-100 ${DAY_STYLE[state] ?? (outside ? "text-ink-3" : "text-ink-2")} ${selected === date ? "outline outline-2 outline-offset-1 outline-accent" : ""} ${outside ? "opacity-50" : ""}`}
              >
                {parseDate(date).getDate()}
                {day?.note && <span aria-hidden className="absolute bottom-1 size-1 rounded-full bg-current" />}
              </button>
            );
          })}
        </div>
      </div>
      <div className="min-w-[220px] flex-1">
        {selected && entry ? (
          <DayEditor key={`${selected}-${entry.state}`} habit={habit} day={entry} />
        ) : (
          <p className="pt-9 text-label text-ink-2">Choose a date to change it or add a note.</p>
        )}
      </div>
    </div>
  );
}

function DayEditor({ habit, day }: { habit: Habit; day: HistoryDay }) {
  const [note, setNote] = useState(day.note);
  const [value, setValue] = useState(day.value === null ? "" : String(day.value));
  const logged = day.state === "done" || day.state === "partial" || (habit.kind === "quit" && day.state === "missed");
  const log = (extra: object) => act("habits.log", { id: habit.id, date: day.date, ...extra });

  return (
    <div className="flex flex-col gap-2 pt-9">
      <p className="text-label font-semibold">{formatLongDate(day.date)}</p>
      <p className="text-label capitalize text-ink-2">{DAY_LABEL[day.state]}</p>
      {habit.kind === "quantity" && (
        <div className="flex items-center gap-2">
          <Input
            type="number"
            min={0}
            step="any"
            value={value}
            aria-label={`Amount${habit.unit ? ` in ${habit.unit}` : ""}`}
            placeholder={`Target ${habit.quantityTarget}`}
            onChange={(event) => setValue(event.target.value)}
            onBlur={() => value !== "" && Number(value) !== day.value && void log({ value: Number(value) })}
            className="w-28"
          />
          <span className="text-label text-ink-2">{habit.unit}</span>
        </div>
      )}
      <Input
        value={note}
        aria-label="Note"
        placeholder="Add a note"
        onChange={(event) => setNote(event.target.value)}
        onBlur={() => note !== day.note && void log({ note })}
      />
      <div className="flex gap-2">
        {habit.kind === "build" && !logged && (
          <Button size="sm" variant="primary" icon={Check} onClick={() => void log({})}>
            Mark done
          </Button>
        )}
        {habit.kind === "quit" && !logged && (
          <Button size="sm" onClick={() => void log({ slip: true })}>
            Log a slip
          </Button>
        )}
        {logged && (
          <Button size="sm" onClick={() => void act("habits.undo", { id: habit.id, date: day.date })}>
            {habit.kind === "quit" ? "Remove slip" : "Undo"}
          </Button>
        )}
      </div>
    </div>
  );
}

const WEEK = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

function Stat({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="tabular text-section font-semibold">{value}</p>
      <p className="text-meta text-ink-2">{label}</p>
    </div>
  );
}

export function HabitDetail({ id }: { id: number }) {
  const { confirmDelete } = useSettings();
  const { data: habit, error } = useRpc<Habit>("habits.get", { id });

  useEffect(() => {
    if (error?.code === "not_found") openHabit(null);
  }, [error]);

  if (!habit) return null;
  const Icon = iconFor(habit.icon);
  const { stats } = habit;
  const unit = stats.streakUnit;

  const remove = async () => {
    if (confirmDelete) {
      const confirmed = await confirm({
        title: "Delete this habit?",
        body: `"${habit.name}" and its whole history will be deleted. This can't be undone. Archive it instead to keep the history.`,
        confirmLabel: "Delete habit",
      });
      if (!confirmed) return;
    }
    openHabit(null);
    await act("habits.delete", { id: habit.id }, "Habit deleted");
  };

  return (
    <Dialog title={habit.name} onClose={() => openHabit(null)} width="max-w-[760px]" bare>
      <div className="flex flex-col gap-6 p-6">
        <header className="flex items-start gap-3">
          <span style={softStyle(habit.color)} className="inline-flex size-12 shrink-0 items-center justify-center rounded-md">
            <Icon size={24} aria-hidden />
          </span>
          <div className="min-w-0 flex-1">
            <h3 className="text-section font-semibold">{habit.name}</h3>
            <p className="text-label text-ink-2">{habit.description || frequencyLabel(habit)}</p>
          </div>
          <div className="flex">
            <IconButton size="sm" icon={Pencil} label="Edit habit" onClick={() => { openHabit(null); editHabit(habit); }} />
            {habit.archivedAt ? (
              <IconButton size="sm" icon={ArchiveRestore} label="Restore habit" onClick={() => void act("habits.restore", { id }, "Habit restored")} />
            ) : (
              <IconButton size="sm" icon={Archive} label="Archive habit" onClick={() => { openHabit(null); void act("habits.archive", { id }, "Habit archived"); }} />
            )}
            <IconButton size="sm" icon={Trash2} label="Delete habit" onClick={() => void remove()} />
            <IconButton size="sm" icon={X} label="Close" onClick={() => openHabit(null)} />
          </div>
        </header>

        <div className="grid grid-cols-5 gap-4">
          <Stat label={`Current streak, ${unit}`} value={String(stats.currentStreak)} />
          <Stat label={`Best streak, ${unit}`} value={String(stats.bestStreak)} />
          <Stat label="Completion" value={percent(stats.completionRate)} />
          <Stat label="Last 7 days" value={percent(stats.weeklyConsistency)} />
          <Stat label="Last 30 days" value={percent(stats.monthlyConsistency)} />
        </div>

        {habit.frequency !== "weekly_target" && (
          <section aria-label="Weekly schedule">
            <h4 className="mb-2 text-label font-semibold">Weekly schedule</h4>
            <div className="flex gap-1">
              {WEEK.map((day, index) => {
                const scheduled = habit.weekdays.includes(index);
                return (
                  <span key={day} className={`inline-flex h-7 flex-1 items-center justify-center gap-1 rounded-sm text-meta ${scheduled ? "bg-accent-soft font-medium text-accent-text" : "bg-sunken text-ink-3"}`}>
                    {scheduled && <Check size={12} aria-hidden />}
                    {day}
                  </span>
                );
              })}
            </div>
          </section>
        )}

        <section>
          <h4 className="mb-2 text-label font-semibold">History</h4>
          <MonthHistory habit={habit} />
        </section>

        <section>
          <h4 className="mb-2 text-label font-semibold">Activity</h4>
          <Heatmap habitId={habit.id} />
        </section>
      </div>
    </Dialog>
  );
}
