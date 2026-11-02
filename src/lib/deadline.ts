import { daysBetween, formatShortDate, formatTime, parseDate, toDateText } from "./date";
import type { Task, TaskStatus } from "./types";

export type DeadlineTone = "overdue" | "today" | "soon" | "later" | "done" | "none";

export interface DeadlineLabel {
  /** Short text for a badge. */
  label: string;
  /** The full sentence, for tooltips and screen readers. */
  detail: string;
  tone: DeadlineTone;
}

const plural = (count: number, unit: string) => `${count} ${unit}${count === 1 ? "" : "s"}`;

/**
 * Describes a deadline. A date-only deadline has no hour to count towards, so it is only ever
 * described in days. That rule is why this takes the time separately.
 */
export function describeDeadline(
  task: Pick<Task, "dueDate" | "dueTime"> & { status: TaskStatus },
  now: Date,
  clock: "12h" | "24h" = "24h",
): DeadlineLabel {
  const { dueDate, dueTime, status } = task;
  if (status === "completed") return { label: "Completed", detail: "Completed", tone: "done" };
  if (!dueDate) return { label: "", detail: "No deadline", tone: "none" };

  const today = toDateText(now);
  const days = daysBetween(today, dueDate);
  const time = dueTime ? formatTime(dueTime, clock) : null;
  const at = time ? `, ${time}` : "";

  if (dueTime) {
    const [hours = 0, minutes = 0] = dueTime.split(":").map(Number);
    const due = parseDate(dueDate);
    due.setHours(hours, minutes, 0, 0);
    const difference = Math.round((due.getTime() - now.getTime()) / 60_000);

    if (difference < 0) {
      const late = -difference;
      if (late < 60) {
        return overdue(plural(late, "minute"));
      }
      if (days === 0) return overdue(plural(Math.floor(late / 60), "hour"));
      return overdue(plural(-days, "day"));
    }
    if (days === 0) {
      if (difference === 0) return { label: "Now", detail: "Due now", tone: "today" };
      if (difference < 60) {
        const text = plural(difference, "minute");
        return { label: `In ${difference} min`, detail: `Due in ${text}`, tone: "today" };
      }
      const hoursLeft = Math.floor(difference / 60);
      return { label: `Today${at}`, detail: `Due in ${plural(hoursLeft, "hour")}`, tone: "today" };
    }
  } else if (days < 0) {
    return overdue(plural(-days, "day"));
  } else if (days === 0) {
    return { label: "Today", detail: "Due today", tone: "today" };
  }

  if (days === 1) return { label: `Tomorrow${at}`, detail: `Due tomorrow${at}`, tone: "soon" };
  if (days < 7) {
    return {
      label: `In ${days} days`,
      detail: `Due in ${days} days${at}`,
      tone: days <= 3 ? "soon" : "later",
    };
  }
  const date = formatShortDate(dueDate, today);
  return { label: date, detail: `Due ${date}${at}`, tone: "later" };
}

function overdue(duration: string): DeadlineLabel {
  return { label: `Overdue by ${duration}`, detail: `Overdue by ${duration}`, tone: "overdue" };
}
