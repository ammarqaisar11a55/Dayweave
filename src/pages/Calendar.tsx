import { Bell, Check, ChevronLeft, ChevronRight, Circle, Flag, type LucideIcon, Plus, Repeat2, Timer } from "lucide-react";
import { type DragEvent, useState } from "react";

import { PageHeader } from "../components/shell/PageHeader";
import { Button, IconButton } from "../components/ui/Button";
import { EmptyState } from "../components/ui/EmptyState";
import { Segmented } from "../components/ui/Form";
import {
  addDays,
  addMonths,
  formatDuration,
  formatLongDate,
  formatMonth,
  formatShortDate,
  formatTime,
  formatWeekday,
  monthGrid,
  parseDate,
  today,
  weekDates,
} from "../lib/date";
import { navigate } from "../lib/router";
import { useSettings } from "../lib/settings";
import { reschedule } from "../lib/taskActions";
import { useRpc } from "../lib/store";
import type { CalendarDay, CalendarItem } from "../lib/types";
import { newTask, openHabit, openTask } from "../lib/ui";

type Mode = "month" | "week" | "day";

const KINDS: Record<CalendarItem["kind"], { label: string; icon: LucideIcon; color: string }> = {
  deadline: { label: "Deadline", icon: Flag, color: "var(--chart-deadline)" },
  task: { label: "Task starting", icon: Circle, color: "var(--chart-task)" },
  habit: { label: "Habit", icon: Repeat2, color: "var(--chart-habit)" },
  focus: { label: "Focus session", icon: Timer, color: "var(--chart-focus)" },
  reminder: { label: "Reminder", icon: Bell, color: "var(--ink-2)" },
};

function openItem(item: CalendarItem) {
  if (item.kind === "habit") openHabit(item.id);
  else if (item.kind === "focus") navigate("focus");
  else openTask(item.id);
}

function Chip({ item, showTime }: { item: CalendarItem; showTime: boolean }) {
  const { timeFormat } = useSettings();
  const { label, icon: Icon, color } = KINDS[item.kind];
  const movable = item.kind === "deadline" && !item.completed;
  return (
    <button
      type="button"
      draggable={movable}
      onDragStart={(event: DragEvent) => {
        event.dataTransfer.setData("text/plain", JSON.stringify({ id: item.id, time: item.time }));
        event.dataTransfer.effectAllowed = "move";
      }}
      onClick={(event) => {
        event.stopPropagation();
        openItem(item);
      }}
      title={`${label}: ${item.title}`}
      className={`interactive flex h-[22px] w-full min-w-0 items-center gap-1.5 rounded-[6px] px-1 text-left text-meta ${item.completed && item.kind !== "focus" ? "text-ink-2 line-through" : item.overdue ? "text-danger" : "text-ink"}`}
    >
      <Icon size={12} aria-hidden style={{ color }} className="shrink-0" />
      <span className="sr-only">{label}:</span>
      {showTime && item.time && <span className="tabular shrink-0 text-ink-2">{formatTime(item.time, timeFormat)}</span>}
      <span className="truncate">{item.title}</span>
    </button>
  );
}

function useDrop(date: string) {
  const [over, setOver] = useState(false);
  return {
    over,
    handlers: {
      onDragOver: (event: DragEvent) => {
        event.preventDefault();
        setOver(true);
      },
      onDragLeave: () => setOver(false),
      onDrop: (event: DragEvent) => {
        event.preventDefault();
        setOver(false);
        const data = event.dataTransfer.getData("text/plain");
        if (!data) return;
        const { id, time } = JSON.parse(data) as { id: number; time: string | null };
        void reschedule(id, date, time);
      },
    },
  };
}

const MONTH_LIMIT = 3;

/** Habits and focus time as small counts, so they can't crowd out the day's deadlines. */
function DayTally({ day }: { day: CalendarDay }) {
  const complete = day.habitsDone >= day.habitsDue;
  const minutes = Math.round(day.focusSeconds / 60);
  return (
    <span className="tabular flex min-w-0 items-center gap-1.5 text-meta text-ink-2">
      {minutes > 0 && (
        <span className="inline-flex items-center gap-0.5">
          <Timer size={11} aria-hidden style={{ color: KINDS.focus.color }} />
          <span className="sr-only">Minutes focused:</span>
          {minutes}
        </span>
      )}
      {day.habitsDue > 0 && (
        <span className={`inline-flex items-center gap-0.5 ${complete ? "text-success" : ""}`}>
          {complete ? <Check size={11} aria-hidden /> : <Repeat2 size={11} aria-hidden />}
          <span className="sr-only">Habits done:</span>
          {day.habitsDone}/{day.habitsDue}
        </span>
      )}
    </span>
  );
}

function CalendarCell({
  date,
  day,
  month,
  selected,
  onSelect,
}: {
  date: string;
  day: CalendarDay | undefined;
  month: number;
  selected: boolean;
  onSelect: () => void;
}) {
  const { over, handlers } = useDrop(date);
  const isToday = date === today();
  const outside = parseDate(date).getMonth() !== month;
  const items = day?.items.filter((item) => item.kind === "deadline" || item.kind === "task") ?? [];
  const hidden = items.length - MONTH_LIMIT;
  return (
    <div
      {...handlers}
      role="gridcell"
      aria-selected={selected}
      onClick={onSelect}
      className={`flex min-h-0 min-w-0 flex-col gap-0.5 overflow-hidden p-1.5 transition-colors duration-100 ${over ? "bg-accent-soft" : selected ? "bg-[var(--hover)]" : "hover:bg-[var(--hover)]"}`}
    >
      <div className="flex items-center justify-between px-0.5">
        <button
          type="button"
          aria-label={formatLongDate(date)}
          aria-current={isToday ? "date" : undefined}
          onClick={onSelect}
          className={`tabular inline-flex size-6 items-center justify-center rounded-full text-label ${isToday ? "bg-accent font-semibold text-accent-ink" : outside ? "text-ink-3" : "text-ink"}`}
        >
          {parseDate(date).getDate()}
        </button>
        {day && date <= today() && <DayTally day={day} />}
      </div>
      {items.slice(0, MONTH_LIMIT).map((item, index) => (
        <Chip key={`${item.kind}-${item.id}-${index}`} item={item} showTime={false} />
      ))}
      {hidden > 0 && <span className="px-1 text-meta text-ink-2">{hidden} more</span>}
    </div>
  );
}

function MonthView({ cursor, days, selected, onSelect }: ViewProps) {
  const { weekStart } = useSettings();
  const rows = monthGrid(cursor, weekStart);
  const month = parseDate(cursor).getMonth();
  return (
    <div role="grid" aria-label={formatMonth(cursor)} className="surface flex min-h-[560px] flex-1 flex-col overflow-hidden">
      <div role="row" className="grid grid-cols-7 border-b border-line">
        {rows[0]?.map((date) => (
          <span key={date} role="columnheader" className="py-2 text-center text-meta font-medium text-ink-2">
            {formatWeekday(date)}
          </span>
        ))}
      </div>
      <div className="grid flex-1 divide-y divide-[var(--line)]" style={{ gridTemplateRows: `repeat(${rows.length}, minmax(0, 1fr))` }}>
        {rows.map((row) => (
          <div key={row[0]} role="row" className="grid grid-cols-7 divide-x divide-[var(--line)]">
            {row.map((date) => (
              <CalendarCell key={date} date={date} day={days.get(date)} month={month} selected={selected === date} onSelect={() => onSelect(date)} />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}

function WeekColumn({ date, day, selected, onSelect }: { date: string; day: CalendarDay | undefined; selected: boolean; onSelect: () => void }) {
  const { over, handlers } = useDrop(date);
  const isToday = date === today();
  return (
    <div {...handlers} onClick={onSelect} className={`flex min-w-0 flex-col gap-1 p-2 transition-colors duration-100 ${over ? "bg-accent-soft" : selected ? "bg-[var(--hover)]" : ""}`}>
      <button type="button" onClick={onSelect} className="flex flex-col items-center gap-0.5 pb-1.5" aria-label={formatLongDate(date)}>
        <span className="text-meta text-ink-2">{formatWeekday(date)}</span>
        <span className={`tabular inline-flex size-7 items-center justify-center rounded-full text-body ${isToday ? "bg-accent font-semibold text-accent-ink" : ""}`}>
          {parseDate(date).getDate()}
        </span>
      </button>
      {day?.items.map((item, index) => <Chip key={`${item.kind}-${item.id}-${index}`} item={item} showTime />)}
    </div>
  );
}

function WeekView({ cursor, days, selected, onSelect }: ViewProps) {
  const { weekStart } = useSettings();
  return (
    <div className="surface grid min-h-[560px] flex-1 grid-cols-7 divide-x divide-[var(--line)] overflow-hidden">
      {weekDates(cursor, weekStart).map((date) => (
        <WeekColumn key={date} date={date} day={days.get(date)} selected={selected === date} onSelect={() => onSelect(date)} />
      ))}
    </div>
  );
}

interface ViewProps {
  cursor: string;
  days: Map<string, CalendarDay>;
  selected: string;
  onSelect: (date: string) => void;
}

/** The selected date's agenda, grouped by kind. It is also the whole of the day view. */
function Agenda({ date, day, wide = false }: { date: string; day: CalendarDay | undefined; wide?: boolean }) {
  const { timeFormat } = useSettings();
  const groups = (Object.keys(KINDS) as CalendarItem["kind"][])
    .map((kind) => ({ kind, items: day?.items.filter((item) => item.kind === kind) ?? [] }))
    .filter((group) => group.items.length > 0);
  return (
    <aside aria-label={`Agenda for ${formatLongDate(date)}`} className={`surface flex min-h-0 flex-col ${wide ? "flex-1" : "w-[280px] shrink-0 max-[1100px]:w-full"}`}>
      <div className="flex items-center justify-between gap-2 px-4 pb-2 pt-3">
        <h2 className="truncate text-heading font-semibold">{formatLongDate(date)}</h2>
        <Button size="sm" variant="ghost" icon={Plus} onClick={() => newTask({ dueDate: date })} aria-label={`Add a task on ${formatShortDate(date)}`}>
          Task
        </Button>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-3">
        {groups.length === 0 && <EmptyState compact icon={Circle} title="Nothing planned" body="Add a task to this date." />}
        {groups.map(({ kind, items }) => {
          const { label, icon: Icon, color } = KINDS[kind];
          return (
            <section key={kind} aria-label={label} className="mb-2">
              <h3 className="flex items-center gap-1.5 px-2 py-1 text-meta font-medium text-ink-2">
                <Icon size={12} aria-hidden style={{ color }} />
                {label}
              </h3>
              <ul>
                {items.map((item, index) => (
                  <li key={`${item.id}-${index}`}>
                    <button
                      type="button"
                      onClick={() => openItem(item)}
                      className="interactive flex w-full items-center gap-2 rounded-sm px-2 py-1.5 text-left text-body"
                    >
                      <span className={`min-w-0 flex-1 truncate ${item.completed && kind !== "focus" ? "text-ink-2 line-through" : item.overdue ? "text-danger" : ""}`}>{item.title}</span>
                      {item.overdue && kind !== "habit" && <span className="text-meta text-danger">Overdue</span>}
                      {item.overdue && kind === "habit" && <span className="text-meta text-danger">Missed</span>}
                      {item.seconds !== null && <span className="tabular text-meta text-ink-2">{formatDuration(item.seconds)}</span>}
                      {item.time && <span className="tabular text-meta text-ink-2">{formatTime(item.time, timeFormat)}</span>}
                      {item.completed && kind === "habit" && <Check size={14} aria-label="Done" className="text-success" />}
                    </button>
                  </li>
                ))}
              </ul>
            </section>
          );
        })}
      </div>
    </aside>
  );
}

export function Calendar() {
  const { weekStart } = useSettings();
  const [mode, setMode] = useState<Mode>("month");
  const [cursor, setCursor] = useState(today());
  const [selected, setSelected] = useState(today());

  const range = (() => {
    if (mode === "day") return [cursor, cursor];
    if (mode === "week") {
      const week = weekDates(cursor, weekStart);
      return [week[0], week[6]];
    }
    const rows = monthGrid(cursor, weekStart);
    return [rows[0]?.[0], rows[rows.length - 1]?.[6]];
  })();
  const { data } = useRpc<CalendarDay[]>("calendar.range", { from: range[0], to: range[1] });
  const days = new Map((data ?? []).map((day) => [day.date, day]));

  const step = (direction: number) => {
    const next = mode === "month" ? addMonths(cursor, direction) : addDays(cursor, direction * (mode === "week" ? 7 : 1));
    setCursor(next);
    setSelected(next);
  };

  const heading = () => {
    if (mode === "month") return formatMonth(cursor);
    if (mode === "day") return formatLongDate(cursor);
    const week = weekDates(cursor, weekStart);
    return `${formatShortDate(week[0] ?? cursor)} to ${formatShortDate(week[6] ?? cursor)}`;
  };

  const selectedDay = days.get(selected);

  return (
    <div className="animate-page mx-auto flex h-full max-w-[1320px] flex-col">
      <PageHeader
        title={heading()}
        actions={
          <>
            <Segmented
              label="Calendar view"
              value={mode}
              onChange={setMode}
              options={[
                { value: "month", label: "Month" },
                { value: "week", label: "Week" },
                { value: "day", label: "Day" },
              ]}
            />
            <Button
              onClick={() => {
                setCursor(today());
                setSelected(today());
              }}
            >
              Today
            </Button>
            <IconButton icon={ChevronLeft} label="Previous" onClick={() => step(-1)} />
            <IconButton icon={ChevronRight} label="Next" onClick={() => step(1)} />
          </>
        }
      />
      <div className="mb-3 flex flex-wrap gap-x-4 gap-y-1 text-meta text-ink-2" aria-hidden>
        {(["deadline", "task", "habit", "focus", "reminder"] as const).map((kind) => {
          const { label, icon: Icon, color } = KINDS[kind];
          return (
            <span key={kind} className="inline-flex items-center gap-1.5">
              <Icon size={12} style={{ color }} />
              {label}
            </span>
          );
        })}
      </div>
      <div className="flex min-h-0 flex-1 gap-4 max-[1100px]:flex-col">
        {mode === "month" && <MonthView cursor={cursor} days={days} selected={selected} onSelect={setSelected} />}
        {mode === "week" && <WeekView cursor={cursor} days={days} selected={selected} onSelect={setSelected} />}
        {mode === "day" ? <Agenda date={cursor} day={days.get(cursor)} wide /> : <Agenda date={selected} day={selectedDay} />}
      </div>
    </div>
  );
}
