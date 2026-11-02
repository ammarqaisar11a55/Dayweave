import { BarChart3, Table2 } from "lucide-react";
import { type ReactNode, useState } from "react";
import {
  Bar,
  BarChart,
  CartesianGrid,
  Line,
  LineChart,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";

import { PRIORITY } from "../components/Badges";
import { PageHeader, Section } from "../components/shell/PageHeader";
import { IconButton } from "../components/ui/Button";
import { EmptyState } from "../components/ui/EmptyState";
import { Segmented } from "../components/ui/Form";
import { formatDuration, formatShortDate } from "../lib/date";
import { percent } from "../lib/format";
import { colorVar, iconFor, softStyle } from "../lib/palette";
import { useRpc } from "../lib/store";
import type { Analytics as Data, HabitSummary } from "../lib/types";

const AXIS = { fontSize: 11, fill: "var(--ink-2)" };
const HEIGHT = 200;

interface TooltipEntry {
  dataKey?: unknown;
  name?: unknown;
  value?: unknown;
  color?: string;
}

interface ChartTooltipProps {
  unit: (value: number) => string;
  active?: boolean;
  payload?: readonly TooltipEntry[];
  label?: unknown;
}

function ChartTooltip({ active, payload, label, unit }: ChartTooltipProps) {
  if (!active || !payload?.length) return null;
  return (
    <div className="float rounded-sm px-2.5 py-1.5 text-meta">
      <p className="text-ink-2">{formatShortDate(String(label))}</p>
      {payload.map((entry) => (
        <p key={String(entry.dataKey)} className="tabular flex items-center gap-1.5 font-medium text-ink">
          <span aria-hidden className="size-2 rounded-full" style={{ background: entry.color }} />
          {payload.length > 1 && <span className="font-normal text-ink-2">{String(entry.name)}</span>}
          {unit(Number(entry.value ?? 0))}
        </p>
      ))}
    </div>
  );
}

function Stat({ label, value, note }: { label: string; value: string; note: string }) {
  return (
    <div className="surface p-4">
      <p className="text-label text-ink-2">{label}</p>
      <p className="tabular mt-1 text-page font-[650]">{value}</p>
      <p className="mt-0.5 truncate text-meta text-ink-2">{note}</p>
    </div>
  );
}

/** A chart with a table of the same numbers, so nothing depends on seeing the plot. */
function ChartCard({
  title,
  summary,
  columns,
  rows,
  empty = false,
  children,
}: {
  title: string;
  summary: string;
  columns: string[];
  rows: (string | number)[][];
  empty?: boolean;
  children: ReactNode;
}) {
  const [table, setTable] = useState(false);
  return (
    <section aria-label={title} className="surface min-w-0 p-4">
      <div className="mb-3 flex items-start justify-between gap-2">
        <div>
          <h3 className="text-heading font-semibold">{title}</h3>
          <p className="text-meta text-ink-2">{summary}</p>
        </div>
        <IconButton
          size="sm"
          icon={table ? BarChart3 : Table2}
          label={table ? "Show as a chart" : "Show as a table"}
          onClick={() => setTable((state) => !state)}
        />
      </div>
      {empty ? (
        <p className="flex items-center justify-center text-label text-ink-2" style={{ height: HEIGHT }}>
          Nothing recorded in this period
        </p>
      ) : table ? (
        <div className="overflow-y-auto" style={{ height: HEIGHT }}>
          <table className="w-full text-left text-meta">
            <thead className="sticky top-0 bg-card text-ink-2">
              <tr>
                {columns.map((column) => (
                  <th key={column} className="py-1 font-medium">
                    {column}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody className="tabular">
              {rows.map((row, index) => (
                <tr key={index} className="border-t border-line">
                  {row.map((cell, cellIndex) => (
                    <td key={cellIndex} className="py-1">
                      {cell}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <div aria-hidden style={{ height: HEIGHT }}>
          {children}
        </div>
      )}
    </section>
  );
}

const tickDate = (value: string) => formatShortDate(value);

function BarPlot({
  data,
  x,
  series,
  unit,
}: {
  data: object[];
  x: string;
  series: { key: string; name: string; color: string }[];
  unit: (value: number) => string;
}) {
  const stacked = series.length > 1;
  return (
    <ResponsiveContainer width="100%" height="100%">
      <BarChart data={data} margin={{ top: 4, right: 0, bottom: 0, left: 0 }} barCategoryGap="28%">
        <CartesianGrid vertical={false} stroke="var(--line)" strokeDasharray="0" />
        <XAxis dataKey={x} tickFormatter={tickDate} tick={AXIS} axisLine={{ stroke: "var(--line)" }} tickLine={false} minTickGap={32} />
        <YAxis tick={AXIS} axisLine={false} tickLine={false} width={34} allowDecimals={false} />
        <Tooltip content={<ChartTooltip unit={unit} />} cursor={{ fill: "var(--hover)" }} />
        {series.map(({ key, name, color }, index) => (
          <Bar
            key={key}
            dataKey={key}
            name={name}
            fill={color}
            stackId={stacked ? "total" : undefined}
            maxBarSize={22}
            radius={!stacked || index === series.length - 1 ? [4, 4, 0, 0] : 0}
            stroke={stacked ? "var(--card)" : undefined}
            strokeWidth={stacked ? 1 : 0}
            isAnimationActive={false}
          />
        ))}
      </BarChart>
    </ResponsiveContainer>
  );
}

function Legend({ items }: { items: { label: string; color: string }[] }) {
  return (
    <div className="mt-2 flex gap-4 text-meta text-ink-2">
      {items.map(({ label, color }) => (
        <span key={label} className="inline-flex items-center gap-1.5">
          <span aria-hidden className="size-2.5 rounded-[3px]" style={{ background: color }} />
          {label}
        </span>
      ))}
    </div>
  );
}

function HabitList({ habits }: { habits: HabitSummary[] }) {
  return (
    <ul className="surface row-list overflow-hidden">
      {habits.map((habit) => {
        const Icon = iconFor(habit.icon);
        return (
          <li key={habit.id} className="flex items-center gap-2.5 px-3 py-2">
            <span style={softStyle(habit.color)} className="inline-flex size-7 shrink-0 items-center justify-center rounded-sm">
              <Icon size={14} aria-hidden />
            </span>
            <span className="min-w-0 flex-1 truncate text-body">{habit.name}</span>
            <span className="tabular text-label text-ink-2">{percent(habit.monthlyConsistency)}</span>
          </li>
        );
      })}
    </ul>
  );
}

export function Analytics() {
  const [days, setDays] = useState("90");
  const { data } = useRpc<Data>("analytics.get", { days: Number(days) });
  if (!data) return null;

  const hasData = data.tasksCompleted + data.tasksPending > 0 || data.habits.length > 0 || data.focusSeconds > 0;
  const weeks = data.weekly;
  const thisWeek = weeks[weeks.length - 1];
  const lastWeek = weeks[weeks.length - 2];
  const change = (thisWeek?.tasksCompleted ?? 0) - (lastWeek?.tasksCompleted ?? 0);
  const habitWeeks = weeks.filter((week) => week.habitRate !== null).map((week) => ({
    week: week.week,
    rate: Math.round((week.habitRate ?? 0) * 100),
  }));
  const focusDays = data.daily.map((day) => ({ date: day.date, minutes: Math.round(day.focusSeconds / 60) }));
  const focusRows = data.days > 90 ? weeks.map((week) => ({ date: week.week, minutes: Math.round(week.focusSeconds / 60) })) : focusDays;
  const totalPriorities = data.priorities.reduce((sum, item) => sum + item.count, 0);

  return (
    <div className="animate-page mx-auto max-w-[1100px]">
      <PageHeader
        title="Analytics"
        actions={
          <Segmented
            label="Period"
            value={days}
            onChange={setDays}
            options={[
              { value: "30", label: "30 days" },
              { value: "90", label: "90 days" },
              { value: "365", label: "Year" },
            ]}
          />
        }
      />
      {!hasData ? (
        <div className="surface">
          <EmptyState icon={BarChart3} title="Nothing to show yet" body="Complete tasks, log habits and run focus sessions, and your trends will appear here." />
        </div>
      ) : (
        <div className="flex flex-col gap-6">
          <div className="grid grid-cols-4 gap-3 max-[1000px]:grid-cols-2">
            <Stat label="Task completion" value={percent(data.taskCompletionRate)} note={`${data.tasksCompleted} done, ${data.tasksOverdue} overdue, ${data.tasksPending} open`} />
            <Stat label="Habit consistency" value={percent(data.habitCompletionRate)} note="Share of scheduled habits met" />
            <Stat label="Focus time" value={formatDuration(data.focusSeconds)} note={`${data.focusSessions} sessions completed`} />
            <Stat
              label="Weekly productivity"
              value={`${thisWeek?.tasksCompleted ?? 0} tasks`}
              note={change === 0 ? "The same as last week" : `${Math.abs(change)} ${change > 0 ? "more" : "fewer"} than last week`}
            />
          </div>

          <div className="grid grid-cols-2 gap-3 max-[1000px]:grid-cols-1">
            <ChartCard
              title="Weekly task completion"
              summary="Tasks finished each week"
              columns={["Week of", "Tasks"]}
              rows={weeks.map((week) => [formatShortDate(week.week), week.tasksCompleted])}
              empty={data.tasksCompleted === 0}
            >
              <BarPlot
                data={weeks}
                x="week"
                series={[{ key: "tasksCompleted", name: "Tasks", color: "var(--chart-task)" }]}
                unit={(value) => `${value} tasks`}
              />
            </ChartCard>
            <ChartCard
              title="Habit consistency"
              summary="Share of scheduled habits met each week"
              columns={["Week of", "Met"]}
              rows={habitWeeks.map((week) => [formatShortDate(week.week), `${week.rate}%`])}
              empty={habitWeeks.length === 0}
            >
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={habitWeeks} margin={{ top: 4, right: 4, bottom: 0, left: 0 }}>
                  <CartesianGrid vertical={false} stroke="var(--line)" />
                  <XAxis dataKey="week" tickFormatter={tickDate} tick={AXIS} axisLine={{ stroke: "var(--line)" }} tickLine={false} minTickGap={32} />
                  <YAxis domain={[0, 100]} ticks={[0, 50, 100]} tickFormatter={(value: number) => `${value}%`} tick={AXIS} axisLine={false} tickLine={false} width={34} />
                  <Tooltip content={<ChartTooltip unit={(value) => `${value}%`} />} cursor={{ stroke: "var(--ink-3)" }} />
                  <Line type="monotone" dataKey="rate" name="Met" stroke="var(--chart-habit)" strokeWidth={2} dot={{ r: 3, fill: "var(--chart-habit)", stroke: "var(--card)", strokeWidth: 2 }} activeDot={{ r: 5 }} isAnimationActive={false} />
                </LineChart>
              </ResponsiveContainer>
            </ChartCard>
            <ChartCard
              title="Focus time"
              summary={data.days > 90 ? "Minutes focused each week" : "Minutes focused each day"}
              columns={[data.days > 90 ? "Week of" : "Date", "Minutes"]}
              rows={focusRows.map((row) => [formatShortDate(row.date), row.minutes])}
              empty={data.focusSeconds === 0}
            >
              <BarPlot
                data={focusRows}
                x="date"
                series={[{ key: "minutes", name: "Focus", color: "var(--chart-focus)" }]}
                unit={(value) => formatDuration(value * 60)}
              />
            </ChartCard>
            <div className="min-w-0">
              <ChartCard
                title="Completion history"
                summary="Tasks finished and habits met each day"
                columns={["Date", "Tasks", "Habits"]}
                rows={data.daily.map((day) => [formatShortDate(day.date), day.tasksCompleted, day.habitsDone])}
                empty={data.daily.every((day) => day.tasksCompleted + day.habitsDone === 0)}
              >
                <BarPlot
                  data={data.daily}
                  x="date"
                  series={[
                    { key: "tasksCompleted", name: "Tasks", color: "var(--chart-task)" },
                    { key: "habitsDone", name: "Habits", color: "var(--chart-habit)" },
                  ]}
                  unit={(value) => String(value)}
                />
              </ChartCard>
              <Legend
                items={[
                  { label: "Tasks", color: "var(--chart-task)" },
                  { label: "Habits", color: "var(--chart-habit)" },
                ]}
              />
            </div>
          </div>

          <div className="grid grid-cols-3 gap-4 max-[1000px]:grid-cols-1">
            <Section title="Open tasks by priority">
              <div className="surface flex flex-col gap-3 p-4">
                {totalPriorities === 0 && <p className="text-label text-ink-2">No open tasks.</p>}
                {data.priorities.map(({ priority, count }) => {
                  const { label, icon: Icon, className } = PRIORITY[priority];
                  return (
                    <div key={priority}>
                      <div className="mb-1 flex justify-between text-label">
                        <span className={`inline-flex items-center gap-1 ${className}`}>
                          <Icon size={14} aria-hidden />
                          {label}
                        </span>
                        <span className="tabular text-ink-2">{count}</span>
                      </div>
                      <div className="h-1.5 rounded-full bg-sunken">
                        <div className="h-full rounded-full bg-[var(--ink-3)]" style={{ width: `${(count / totalPriorities) * 100}%` }} />
                      </div>
                    </div>
                  );
                })}
              </div>
            </Section>
            <Section title="Most consistent">
              {data.mostConsistent.length ? <HabitList habits={data.mostConsistent} /> : <p className="text-label text-ink-2">No habits yet.</p>}
            </Section>
            <Section title="Needs attention">
              {data.needsImprovement.length ? <HabitList habits={data.needsImprovement} /> : <p className="surface p-4 text-label text-ink-2">Every habit is above 70% this month.</p>}
            </Section>
          </div>

          {data.projects.length > 0 && (
            <Section title="Projects">
              <ul className="surface row-list overflow-hidden">
                {data.projects.map((project) => (
                  <li key={project.id} className="flex items-center gap-3 px-4 py-2.5">
                    <span className="w-48 truncate text-body">{project.name}</span>
                    <div className="h-1.5 flex-1 rounded-full bg-sunken">
                      <div className="h-full rounded-full" style={{ width: `${project.completion * 100}%`, background: colorVar(project.color) }} />
                    </div>
                    <span className="tabular w-24 text-right text-label text-ink-2">
                      {project.done}/{project.total} ({percent(project.completion)})
                    </span>
                  </li>
                ))}
              </ul>
            </Section>
          )}

          {data.habits.length > 0 && (
            <Section title="Habits">
              <div className="surface overflow-x-auto">
                <table className="w-full text-body">
                  <thead>
                    <tr className="text-left text-meta text-ink-2">
                      <th className="px-4 py-2 font-medium">Habit</th>
                      <th className="px-2 py-2 text-right font-medium">Current streak</th>
                      <th className="px-2 py-2 text-right font-medium">Best streak</th>
                      <th className="px-2 py-2 text-right font-medium">Completion</th>
                      <th className="px-2 py-2 text-right font-medium">7 days</th>
                      <th className="px-4 py-2 text-right font-medium">30 days</th>
                    </tr>
                  </thead>
                  <tbody className="tabular">
                    {data.habits.map((habit) => (
                      <tr key={habit.id} className="border-t border-line">
                        <td className="px-4 py-2">{habit.name}</td>
                        <td className="px-2 py-2 text-right">{habit.currentStreak} {habit.streakUnit}</td>
                        <td className="px-2 py-2 text-right">{habit.bestStreak} {habit.streakUnit}</td>
                        <td className="px-2 py-2 text-right">{percent(habit.completionRate)}</td>
                        <td className="px-2 py-2 text-right">{percent(habit.weeklyConsistency)}</td>
                        <td className="px-4 py-2 text-right">{percent(habit.monthlyConsistency)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </Section>
          )}
        </div>
      )}
    </div>
  );
}
