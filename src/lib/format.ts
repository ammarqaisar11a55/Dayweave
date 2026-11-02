import { formatShortDate } from "./date";

/** Formats a local 'YYYY-MM-DDTHH:MM' value. */
export function formatDateTime(text: string): string {
  const [date = "", time = ""] = text.split("T");
  return `${formatShortDate(date)}, ${time}`;
}

export const percent = (ratio: number) => `${Math.round(ratio * 100)}%`;

export function plural(count: number, one: string, many = `${one}s`) {
  return `${count} ${count === 1 ? one : many}`;
}
