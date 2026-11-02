import { useLayoutEffect, useMemo, useRef } from "react";

import { addDays, formatShortDate, parseDate, startOfWeek, today } from "../../lib/date";
import { plural } from "../../lib/format";
import { useSettings } from "../../lib/settings";
import { useRpc } from "../../lib/store";
import type { HeatmapDay } from "../../lib/types";

const CELL = 12;
const STEP = CELL + 3;
const LABEL_HEIGHT = 18;
const DAYS = 371;

const monthName = new Intl.DateTimeFormat(undefined, { month: "short" });

function describe(day: HeatmapDay) {
  const date = formatShortDate(day.date);
  if (day.due === 0) return `${date}: nothing scheduled`;
  if (day.missed) return `${date}: missed`;
  return `${date}: ${day.done} of ${plural(day.due, "habit")} done`;
}

export function Heatmap({ habitId }: { habitId: number | null }) {
  const { weekStart } = useSettings();
  const to = today();
  const from = startOfWeek(addDays(to, -DAYS + 7), weekStart);
  const { data } = useRpc<HeatmapDay[]>("habits.heatmap", { id: habitId, from, to });

  const { weeks, months, total } = useMemo(() => {
    const days = data ?? [];
    const weeks: HeatmapDay[][] = [];
    days.forEach((day, index) => {
      const week = Math.floor(index / 7);
      (weeks[week] ??= []).push(day);
    });
    const months: { label: string; x: number }[] = [];
    let previous = -1;
    weeks.forEach((week, index) => {
      const first = week[0];
      if (!first) return;
      const month = parseDate(first.date).getMonth();
      if (month !== previous) {
        previous = month;
        const last = months[months.length - 1];
        if (!last || index * STEP - last.x > 24) {
          months.push({ label: monthName.format(parseDate(first.date)), x: index * STEP });
        }
      }
    });
    return { weeks, months, total: days.reduce((sum, day) => sum + day.done, 0) };
  }, [data]);

  // When the year doesn't fit, the most recent weeks are the ones to show.
  const scroller = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    if (scroller.current) scroller.current.scrollLeft = scroller.current.scrollWidth;
  }, [weeks.length]);

  const width = weeks.length * STEP;
  return (
    <div>
      <div ref={scroller} className="overflow-x-auto pb-1">
        <svg
          role="img"
          aria-label={`Habit activity over the last year: ${plural(total, "completion")}`}
          width={width}
          height={LABEL_HEIGHT + 7 * STEP}
          className="block"
        >
          {months.map(({ label, x }) => (
            <text key={`${label}-${x}`} x={x} y={11} fontSize={11} fill="var(--ink-2)">
              {label}
            </text>
          ))}
          {weeks.map((week, column) =>
            week.map((day, row) => (
              <rect
                key={day.date}
                x={column * STEP}
                y={LABEL_HEIGHT + row * STEP}
                width={CELL}
                height={CELL}
                rx={3}
                fill={`var(--heat-${day.level})`}
                stroke={day.missed ? "var(--danger)" : "none"}
                strokeOpacity={0.55}
                strokeWidth={1}
              >
                <title>{describe(day)}</title>
              </rect>
            )),
          )}
        </svg>
      </div>
      <div className="mt-2 flex items-center justify-between text-meta text-ink-2">
        <span className="tabular">{plural(total, "completion")} in the last year</span>
        <div className="flex items-center gap-1.5" aria-hidden>
          <span>Less</span>
          {[0, 1, 2, 3, 4].map((level) => (
            <span key={level} className="size-3 rounded-[3px]" style={{ background: `var(--heat-${level})` }} />
          ))}
          <span>More</span>
          <span className="ml-2 size-3 rounded-[3px] border border-danger/60 bg-[var(--heat-0)]" />
          <span>Missed</span>
        </div>
      </div>
    </div>
  );
}
