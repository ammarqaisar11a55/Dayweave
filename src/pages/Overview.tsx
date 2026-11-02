import { CalendarCheck2, CheckCircle2, Plus, Repeat2, Target } from "lucide-react";
import { useState } from "react";

import { HabitCard } from "../components/habit/HabitCard";
import { Section } from "../components/shell/PageHeader";
import { TaskRow } from "../components/task/TaskRow";
import { Button } from "../components/ui/Button";
import { EmptyState } from "../components/ui/EmptyState";
import { Weave } from "../components/Weave";
import { formatDuration, formatLongDate, formatWeekday, greeting, today } from "../lib/date";
import { plural } from "../lib/format";
import { navigate } from "../lib/router";
import { reschedule } from "../lib/taskActions";
import { useRpc } from "../lib/store";
import type { Overview as Data } from "../lib/types";
import { newHabit, newTask } from "../lib/ui";

const VISIBLE_TASKS = 8;

function summary(data: Data): string {
  const tasksLeft = data.tasksTotal - data.tasksDone;
  const habitsLeft = data.habitsTotal - data.habitsDone;
  if (data.tasksTotal + data.habitsTotal === 0) return "Let's make today count.";
  if (tasksLeft + habitsLeft === 0) return "Everything for today is done.";
  const parts = [tasksLeft && plural(tasksLeft, "task"), habitsLeft && plural(habitsLeft, "habit")].filter(Boolean);
  return `${parts.join(" and ")} left today.`;
}

function Hero({ data }: { data: Data }) {
  const done = data.tasksDone + data.habitsDone;
  const total = data.tasksTotal + data.habitsTotal;
  const tasks = data.tasks.map((task) => ({ title: task.title, done: task.status === "completed" }));
  const habits = data.habits.map((habit) => ({
    title: habit.name,
    done: habit.kind === "quit" ? !habit.today.slipped : habit.today.state === "done",
  }));
  return (
    <header className="mb-8 flex flex-wrap items-center justify-between gap-x-10 gap-y-5">
      <div className="min-w-0">
        <p className="text-body text-ink-2">{formatLongDate(data.date)}</p>
        <h1 className="mt-1 text-[34px] font-[650] leading-[40px] tracking-[-0.022em]">{greeting(new Date().getHours())}</h1>
        <p className="mt-1.5 text-heading text-ink-2">{summary(data)}</p>
      </div>
      <div className="flex items-center gap-6">
        <dl className="flex flex-col gap-1 text-right text-label text-ink-2">
          <div>
            <dt className="sr-only">Done today</dt>
            <dd className="tabular text-page font-[650] text-ink">{total ? Math.round(data.progress * 100) : 0}%</dd>
          </div>
          <div>
            <dt className="sr-only">Tasks</dt>
            <dd className="tabular">
              {data.tasksDone} of {plural(data.tasksTotal, "task")}
            </dd>
          </div>
          <div>
            <dt className="sr-only">Habits</dt>
            <dd className="tabular">
              {data.habitsDone} of {plural(data.habitsTotal, "habit")}
            </dd>
          </div>
          {data.focusSeconds > 0 && (
            <div>
              <dt className="sr-only">Focus</dt>
              <dd className="tabular">{formatDuration(data.focusSeconds)} focused</dd>
            </div>
          )}
        </dl>
        <Weave tasks={tasks} habits={habits} label={`Today's weave: ${done} of ${total} done`} />
      </div>
    </header>
  );
}

function Week({ data }: { data: Data }) {
  const best = Math.max(1, ...data.week.map((day) => day.tasksDone + day.habitsDone));
  return (
    <div className="surface flex items-end justify-between gap-1 p-4" role="list" aria-label="This week">
      {data.week.map((day) => {
        const count = day.tasksDone + day.habitsDone;
        const isToday = day.date === data.date;
        const future = day.date > data.date;
        return (
          <div key={day.date} role="listitem" className="flex flex-1 flex-col items-center gap-1.5" aria-label={`${formatWeekday(day.date)}: ${plural(count, "item")} done`}>
            <span className="tabular text-meta text-ink-2">{future ? "" : count}</span>
            <div className="flex h-14 w-full max-w-5 items-end">
              <div
                className={`w-full rounded-t-[4px] transition-[height] duration-200 ease-weave ${future ? "" : isToday ? "bg-accent" : "bg-[var(--thread-warp)]"}`}
                style={{ height: `${Math.max(count ? 8 : 2, (count / best) * 100)}%` }}
              />
            </div>
            <span className={`text-meta ${isToday ? "font-semibold text-accent-text" : "text-ink-2"}`}>{formatWeekday(day.date).slice(0, 2)}</span>
          </div>
        );
      })}
    </div>
  );
}

export function Overview() {
  const { data } = useRpc<Data>("overview.get");
  const [showAll, setShowAll] = useState(false);
  if (!data) return null;
  const tasks = showAll ? data.tasks : data.tasks.slice(0, VISIBLE_TASKS);
  const empty = data.tasksTotal === 0 && data.habitsTotal === 0 && data.overdue.length === 0 && data.upcoming.length === 0;

  return (
    <div className="animate-page mx-auto max-w-[1180px]">
      <Hero data={data} />

      {empty ? (
        <div className="surface">
          <EmptyState
            icon={CalendarCheck2}
            title="Your day is open"
            body="Add a task with a deadline, or a habit to build. They'll show up here each day."
            action={
              <div className="flex gap-2">
                <Button variant="primary" icon={Plus} onClick={() => newTask({ dueDate: today() })}>
                  Add a task for today
                </Button>
                <Button icon={Repeat2} onClick={newHabit}>
                  Add a habit
                </Button>
              </div>
            }
          />
        </div>
      ) : (
        <div className="grid grid-cols-[minmax(0,3fr)_minmax(300px,2fr)] items-start gap-6 max-[1000px]:grid-cols-1">
          <div className="flex min-w-0 flex-col gap-6">
            {data.overdue.length > 0 && (
              <Section
                title={`Overdue (${data.overdue.length})`}
                action={
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => data.overdue.forEach((task) => void reschedule(task.id, today(), task.dueTime))}
                  >
                    Move all to today
                  </Button>
                }
              >
                <div className="surface row-list overflow-hidden">
                  {data.overdue.slice(0, 5).map((task) => (
                    <TaskRow key={task.id} task={task} />
                  ))}
                </div>
                {data.overdue.length > 5 && (
                  <Button size="sm" variant="ghost" className="mt-1" onClick={() => navigate("tasks", { view: "overdue" })}>
                    Show all {data.overdue.length}
                  </Button>
                )}
              </Section>
            )}
            <Section
              title="Today's tasks"
              action={
                <Button size="sm" variant="ghost" icon={Plus} onClick={() => newTask({ dueDate: today() })}>
                  Add
                </Button>
              }
            >
              <div className="surface row-list overflow-hidden">
                {tasks.map((task) => (
                  <TaskRow key={task.id} task={task} />
                ))}
                {data.tasks.length === 0 && <EmptyState compact icon={CheckCircle2} title="No tasks today" body="Add one, or plan ahead in Calendar." />}
              </div>
              {!showAll && data.tasks.length > VISIBLE_TASKS && (
                <Button size="sm" variant="ghost" className="mt-1" onClick={() => setShowAll(true)}>
                  Show {data.tasks.length - VISIBLE_TASKS} more
                </Button>
              )}
            </Section>
            {data.upcoming.length > 0 && (
              <Section title="Upcoming deadlines">
                <div className="surface row-list overflow-hidden">
                  {data.upcoming.map((task) => (
                    <TaskRow key={task.id} task={task} />
                  ))}
                </div>
              </Section>
            )}
          </div>
          <div className="flex min-w-0 flex-col gap-6">
            <Section
              title="Today's habits"
              action={
                <Button size="sm" variant="ghost" onClick={() => navigate("habits")}>
                  All habits
                </Button>
              }
            >
              <div className="flex flex-col gap-2">
                {data.habits.map((habit) => (
                  <HabitCard key={habit.id} habit={habit} />
                ))}
                {data.habits.length === 0 && (
                  <div className="surface">
                    <EmptyState compact icon={Repeat2} title="No habits due today" />
                  </div>
                )}
              </div>
            </Section>
            <Section title="This week">
              <Week data={data} />
            </Section>
            <Section title="Quick actions">
              <div className="grid grid-cols-2 gap-2">
                <Button icon={Plus} onClick={() => newTask()}>
                  Add task
                </Button>
                <Button icon={Repeat2} onClick={newHabit}>
                  Add habit
                </Button>
                <Button icon={Target} onClick={() => navigate("focus")}>
                  Start focus
                </Button>
                <Button icon={CalendarCheck2} onClick={() => navigate("calendar")}>
                  Open calendar
                </Button>
              </div>
            </Section>
          </div>
        </div>
      )}
    </div>
  );
}
