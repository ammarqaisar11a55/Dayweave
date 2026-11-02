import { CalendarDays, ChevronLeft, ChevronRight, X } from "lucide-react";
import { type KeyboardEvent, useEffect, useRef, useState } from "react";

import {
  addDays,
  addMonths,
  formatMonth,
  formatShortDate,
  formatWeekday,
  monthGrid,
  parseDate,
  today,
} from "../../lib/date";
import { useSettings } from "../../lib/settings";
import { IconButton } from "./Button";
import { Popover } from "./Popover";

interface DatePickerProps {
  value: string | null;
  onChange: (value: string | null) => void;
  label: string;
  placeholder?: string;
  clearable?: boolean;
  className?: string;
  size?: "md" | "sm";
}

export function DatePicker({
  value,
  onChange,
  label,
  placeholder = "No date",
  clearable = true,
  className = "",
  size = "md",
}: DatePickerProps) {
  const [open, setOpen] = useState(false);
  const anchor = useRef<HTMLButtonElement>(null);
  const height = size === "md" ? "h-9 px-3" : "h-8 px-2.5 text-label";
  return (
    <div className={`relative inline-flex min-w-0 ${className}`}>
      <button
        ref={anchor}
        type="button"
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-label={value ? `${label}: ${formatShortDate(value)}` : label}
        onClick={() => setOpen((state) => !state)}
        className={`inline-flex w-full min-w-0 items-center gap-2 control rounded-md text-left transition-colors duration-150 ${height} ${value ? "text-ink" : "text-ink-3"}`}
      >
        <CalendarDays size={15} aria-hidden className="shrink-0 text-ink-2" />
        <span className="truncate">{value ? formatShortDate(value) : placeholder}</span>
      </button>
      {clearable && value && (
        <button
          type="button"
          aria-label={`Clear ${label.toLowerCase()}`}
          onClick={() => onChange(null)}
          className="interactive absolute right-1 top-1/2 inline-flex size-6 -translate-y-1/2 items-center justify-center rounded-sm text-ink-2"
        >
          <X size={13} aria-hidden />
        </button>
      )}
      {open && (
        <Popover anchor={anchor} onClose={() => setOpen(false)} label={label}>
          <Calendar
            value={value}
            onChange={(date) => {
              onChange(date);
              setOpen(false);
              anchor.current?.focus();
            }}
          />
        </Popover>
      )}
    </div>
  );
}

export function Calendar({ value, onChange }: { value: string | null; onChange: (value: string) => void }) {
  const { weekStart } = useSettings();
  const [focused, setFocused] = useState(value ?? today());
  const grid = useRef<HTMLDivElement>(null);
  const rows = monthGrid(focused, weekStart);
  const month = parseDate(focused).getMonth();

  useEffect(() => {
    grid.current?.querySelector<HTMLElement>("[tabindex='0']")?.focus();
  }, [focused]);

  const onKeyDown = (event: KeyboardEvent) => {
    const moves: Record<string, string> = {
      ArrowLeft: addDays(focused, -1),
      ArrowRight: addDays(focused, 1),
      ArrowUp: addDays(focused, -7),
      ArrowDown: addDays(focused, 7),
      PageUp: addMonths(focused, -1),
      PageDown: addMonths(focused, 1),
    };
    const next = moves[event.key];
    if (next) {
      event.preventDefault();
      setFocused(next);
    }
  };

  const quick = [
    { label: "Today", date: today() },
    { label: "Tomorrow", date: addDays(today(), 1) },
    { label: "Next week", date: addDays(today(), 7) },
  ];
  return (
    <div className="w-[284px] p-3">
      <div className="mb-2 flex items-center justify-between">
        <span className="pl-1 text-body font-semibold">{formatMonth(focused)}</span>
        <div className="flex">
          <IconButton size="sm" icon={ChevronLeft} label="Previous month" onClick={() => setFocused(addMonths(focused, -1))} />
          <IconButton size="sm" icon={ChevronRight} label="Next month" onClick={() => setFocused(addMonths(focused, 1))} />
        </div>
      </div>
      <div ref={grid} role="grid" aria-label={formatMonth(focused)} onKeyDown={onKeyDown}>
        <div role="row" className="grid grid-cols-7">
          {rows[0]?.map((date) => (
            <span key={date} role="columnheader" className="pb-1 text-center text-meta text-ink-2">
              {formatWeekday(date).slice(0, 2)}
            </span>
          ))}
        </div>
        {rows.map((row) => (
          <div key={row[0]} role="row" className="grid grid-cols-7">
            {row.map((date) => {
              const selected = date === value;
              const isToday = date === today();
              const outside = parseDate(date).getMonth() !== month;
              return (
                <button
                  key={date}
                  type="button"
                  role="gridcell"
                  aria-selected={selected}
                  aria-label={formatShortDate(date)}
                  aria-current={isToday ? "date" : undefined}
                  tabIndex={date === focused ? 0 : -1}
                  onClick={() => onChange(date)}
                  className={`tabular m-0.5 flex h-8 items-center justify-center rounded-sm text-label transition-colors duration-100 ${
                    selected
                      ? "bg-accent font-semibold text-accent-ink"
                      : `interactive ${outside ? "text-ink-3" : "text-ink"} ${isToday ? "font-semibold text-accent-text" : ""}`
                  }`}
                >
                  {parseDate(date).getDate()}
                </button>
              );
            })}
          </div>
        ))}
      </div>
      <div className="mt-2 flex gap-1 border-t border-line pt-2">
        {quick.map(({ label, date }) => (
          <button
            key={label}
            type="button"
            onClick={() => onChange(date)}
            className="interactive flex-1 rounded-sm py-1 text-meta font-medium text-ink-2"
          >
            {label}
          </button>
        ))}
      </div>
    </div>
  );
}
