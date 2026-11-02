import { Clock, X } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { formatTime } from "../../lib/date";
import { useSettings } from "../../lib/settings";
import { Input } from "./Form";
import { Popover } from "./Popover";

const SLOTS = Array.from({ length: 48 }, (_, index) => {
  const hours = String(Math.floor(index / 2)).padStart(2, "0");
  return `${hours}:${index % 2 ? "30" : "00"}`;
});

/** Accepts '9', '9:30', '0930', '17', '5pm' or '5:30 pm'. */
export function parseTimeInput(text: string): string | null {
  const match = text.trim().toLowerCase().match(/^(\d{1,2})(?::?(\d{2}))?\s*(am|pm)?$/);
  if (!match) return null;
  let hours = Number(match[1]);
  const minutes = Number(match[2] ?? 0);
  const suffix = match[3];
  if (suffix) {
    if (hours < 1 || hours > 12) return null;
    hours = (hours % 12) + (suffix === "pm" ? 12 : 0);
  }
  if (hours > 23 || minutes > 59) return null;
  return `${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}`;
}

export function TimePicker({
  value,
  onChange,
  label,
  placeholder = "No time",
  disabled = false,
  clearable = true,
  className = "",
}: {
  value: string | null;
  onChange: (value: string | null) => void;
  label: string;
  placeholder?: string;
  disabled?: boolean;
  clearable?: boolean;
  className?: string;
}) {
  const { timeFormat } = useSettings();
  const [open, setOpen] = useState(false);
  const [typed, setTyped] = useState("");
  const anchor = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const parsed = parseTimeInput(typed);

  useEffect(() => {
    if (!open) return;
    setTyped("");
    input.current?.focus();
    const nearest = value ? `${value.slice(0, 2)}:${value.slice(3) < "30" ? "00" : "30"}` : "09:00";
    list.current?.querySelector(`[data-slot="${nearest}"]`)?.scrollIntoView({ block: "center" });
  }, [open, value]);

  const choose = (time: string) => {
    onChange(time);
    setOpen(false);
    anchor.current?.focus();
  };

  return (
    <div className={`relative inline-flex min-w-0 ${className}`}>
      <button
        ref={anchor}
        type="button"
        disabled={disabled}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-label={value ? `${label}: ${formatTime(value, timeFormat)}` : label}
        onClick={() => setOpen((state) => !state)}
        className={`inline-flex h-9 w-full min-w-0 items-center gap-2 control rounded-md px-3 text-left transition-colors duration-150 disabled:pointer-events-none disabled:opacity-45 ${value ? "text-ink" : "text-ink-3"}`}
      >
        <Clock size={15} aria-hidden className="shrink-0 text-ink-2" />
        <span className="truncate">{value ? formatTime(value, timeFormat) : placeholder}</span>
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
        <Popover anchor={anchor} onClose={() => setOpen(false)} label={label} className="w-40">
          <div className="p-1.5">
            <Input
              ref={input}
              value={typed}
              onChange={(event) => setTyped(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && parsed) {
                  event.preventDefault();
                  choose(parsed);
                }
              }}
              placeholder="Type a time"
              aria-label={`Type ${label.toLowerCase()}`}
              className="h-8 text-label"
            />
          </div>
          <div ref={list} className="max-h-52 overflow-y-auto p-1 pt-0">
            {SLOTS.map((slot) => (
              <button
                key={slot}
                type="button"
                data-slot={slot}
                onClick={() => choose(slot)}
                className={`interactive tabular flex h-8 w-full items-center rounded-sm px-2.5 text-body ${slot === value ? "font-semibold text-accent-text" : "text-ink"}`}
              >
                {formatTime(slot, timeFormat)}
              </button>
            ))}
          </div>
        </Popover>
      )}
    </div>
  );
}
