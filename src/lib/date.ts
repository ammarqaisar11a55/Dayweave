// Calendar dates travel as 'YYYY-MM-DD' text, matching the core. They are built from local parts,
// because `new Date('2026-11-02')` means midnight UTC and can land on the previous local day.

export function parseDate(text: string): Date {
  const [year = 1970, month = 1, day = 1] = text.split("-").map(Number);
  return new Date(year, month - 1, day);
}

export function toDateText(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${date.getFullYear()}-${month}-${day}`;
}

export function today(): string {
  return toDateText(new Date());
}

export function addDays(text: string, days: number): string {
  const date = parseDate(text);
  date.setDate(date.getDate() + days);
  return toDateText(date);
}

export function addMonths(text: string, months: number): string {
  const date = parseDate(text);
  const day = date.getDate();
  date.setDate(1);
  date.setMonth(date.getMonth() + months);
  const last = new Date(date.getFullYear(), date.getMonth() + 1, 0).getDate();
  date.setDate(Math.min(day, last));
  return toDateText(date);
}

/** Whole calendar days from `from` to `to`, unaffected by daylight saving. */
export function daysBetween(from: string, to: string): number {
  const utc = (text: string) => {
    const date = parseDate(text);
    return Date.UTC(date.getFullYear(), date.getMonth(), date.getDate());
  };
  return Math.round((utc(to) - utc(from)) / 86_400_000);
}

const WEEK_START = { monday: 1, sunday: 0, saturday: 6 };
export type WeekStart = keyof typeof WEEK_START;

export function startOfWeek(text: string, weekStart: WeekStart): string {
  const offset = (parseDate(text).getDay() - WEEK_START[weekStart] + 7) % 7;
  return addDays(text, -offset);
}

export function weekDates(text: string, weekStart: WeekStart): string[] {
  const first = startOfWeek(text, weekStart);
  return Array.from({ length: 7 }, (_, index) => addDays(first, index));
}

/** The full weeks covering a month, as rows of dates. */
export function monthGrid(text: string, weekStart: WeekStart): string[][] {
  const date = parseDate(text);
  const first = toDateText(new Date(date.getFullYear(), date.getMonth(), 1));
  const last = toDateText(new Date(date.getFullYear(), date.getMonth() + 1, 0));
  const rows: string[][] = [];
  let cursor = startOfWeek(first, weekStart);
  while (cursor <= last) {
    rows.push(Array.from({ length: 7 }, (_, index) => addDays(cursor, index)));
    cursor = addDays(cursor, 7);
  }
  return rows;
}

const format = (options: Intl.DateTimeFormatOptions) => new Intl.DateTimeFormat(undefined, options);

const longDate = format({ weekday: "long", day: "numeric", month: "long" });
const shortDate = format({ day: "numeric", month: "short" });
const yearDate = format({ day: "numeric", month: "short", year: "numeric" });
const monthYear = format({ month: "long", year: "numeric" });
const weekdayShort = format({ weekday: "short" });

export const formatLongDate = (text: string) => longDate.format(parseDate(text));
export const formatMonth = (text: string) => monthYear.format(parseDate(text));
export const formatWeekday = (text: string) => weekdayShort.format(parseDate(text));

export function formatShortDate(text: string, reference = today()): string {
  const sameYear = text.slice(0, 4) === reference.slice(0, 4);
  return (sameYear ? shortDate : yearDate).format(parseDate(text));
}

export function formatTime(time: string, clock: "12h" | "24h"): string {
  if (clock === "24h") return time;
  const [hours = 0, minutes = 0] = time.split(":").map(Number);
  const suffix = hours < 12 ? "AM" : "PM";
  return `${hours % 12 || 12}:${String(minutes).padStart(2, "0")} ${suffix}`;
}

export function formatInstant(text: string): string {
  return format({ day: "numeric", month: "short", hour: "numeric", minute: "2-digit" }).format(new Date(text));
}

export function greeting(hour: number): string {
  if (hour < 5) return "Good night";
  if (hour < 12) return "Good morning";
  if (hour < 18) return "Good afternoon";
  return "Good evening";
}

export function formatDuration(seconds: number): string {
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} min`;
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  return rest ? `${hours} h ${rest} min` : `${hours} h`;
}

export function formatClock(seconds: number): string {
  const safe = Math.max(0, Math.round(seconds));
  const hours = Math.floor(safe / 3600);
  const minutes = Math.floor((safe % 3600) / 60);
  const rest = String(safe % 60).padStart(2, "0");
  if (hours) return `${hours}:${String(minutes).padStart(2, "0")}:${rest}`;
  return `${String(minutes).padStart(2, "0")}:${rest}`;
}
