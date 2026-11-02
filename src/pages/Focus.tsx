import { Check, Pause, Play, Square, Timer, Trash2 } from "lucide-react";
import { useEffect, useState } from "react";

import { Section } from "../components/shell/PageHeader";
import { Button, IconButton } from "../components/ui/Button";
import { EmptyState } from "../components/ui/EmptyState";
import { Input, Segmented } from "../components/ui/Form";
import { type Option, Select } from "../components/ui/Select";
import { formatClock, formatDuration, formatInstant } from "../lib/date";
import { useCountdown } from "../lib/focus";
import { useRoute } from "../lib/router";
import { useSettings } from "../lib/settings";
import { act, useRpc } from "../lib/store";
import type { FocusSession, FocusStats, Habit, Task } from "../lib/types";

const KIND_LABEL = { focus: "Focus", short_break: "Short break", long_break: "Long break" };
type Kind = keyof typeof KIND_LABEL;

const RADIUS = 136;
const CIRCUMFERENCE = 2 * Math.PI * RADIUS;

function Dial({ remaining, planned, paused, children }: { remaining: number; planned: number; paused: boolean; children: React.ReactNode }) {
  const left = planned ? remaining / planned : 1;
  return (
    <div className="relative size-[300px]">
      <svg viewBox="0 0 300 300" aria-hidden className="size-full -rotate-90">
        <circle cx="150" cy="150" r={RADIUS} fill="none" stroke="var(--thread-ghost)" strokeWidth="6" />
        <circle
          cx="150"
          cy="150"
          r={RADIUS}
          fill="none"
          stroke={paused ? "var(--ink-3)" : "var(--color-accent)"}
          strokeWidth="6"
          strokeLinecap="round"
          strokeDasharray={CIRCUMFERENCE}
          strokeDashoffset={CIRCUMFERENCE * (1 - left)}
          style={{ transition: "stroke-dashoffset 500ms linear, stroke 200ms" }}
        />
      </svg>
      <div className="absolute inset-0 flex flex-col items-center justify-center">{children}</div>
    </div>
  );
}

function Running({ session }: { session: FocusSession }) {
  const remaining = useCountdown(session);
  const paused = session.status === "paused";
  return (
    <div className="animate-settle flex flex-col items-center gap-8 pt-[6vh]">
      <Dial remaining={remaining} planned={session.plannedSeconds} paused={paused}>
        <p className="text-label text-ink-2">{paused ? "Paused" : KIND_LABEL[session.kind]}</p>
        <p role="timer" aria-label={`${formatDuration(remaining)} left`} className="tabular text-[60px] font-[600] leading-[72px] tracking-[-0.03em]">
          {formatClock(remaining)}
        </p>
        {(session.taskTitle || session.habitName) && (
          <p className="max-w-[220px] truncate text-label text-ink-2">{session.taskTitle ?? session.habitName}</p>
        )}
      </Dial>
      <div className="flex items-center gap-2">
        {paused ? (
          <Button variant="primary" icon={Play} onClick={() => void act("focus.resume")}>
            Resume
          </Button>
        ) : (
          <Button variant="primary" icon={Pause} onClick={() => void act("focus.pause")}>
            Pause
          </Button>
        )}
        <Button icon={Check} onClick={() => void act("focus.complete", {}, "Session completed")}>
          Finish
        </Button>
        <Button variant="ghost" icon={Square} onClick={() => void act("focus.stop", {}, "Session stopped. Time so far is saved")}>
          Stop
        </Button>
      </div>
    </div>
  );
}

function Setup({ stats }: { stats: FocusStats | undefined }) {
  const settings = useSettings();
  const { query } = useRoute();
  const longBreakDue = (stats?.sessionsSinceLongBreak ?? 0) >= settings.sessionsBeforeLongBreak;
  const [kind, setKind] = useState<Kind>("focus");
  const [preset, setPreset] = useState("default");
  const [custom, setCustom] = useState("30");
  const [taskId, setTaskId] = useState(query.get("task") ?? "");
  const [habitId, setHabitId] = useState("");
  const { data: tasks } = useRpc<Task[]>("tasks.list", { view: "all", statuses: ["todo", "in_progress"] });
  const { data: habits } = useRpc<Habit[]>("habits.list");

  useEffect(() => {
    const task = query.get("task");
    if (task) setTaskId(task);
  }, [query]);

  const minutes = (() => {
    if (kind === "short_break") return settings.shortBreakMinutes;
    if (kind === "long_break") return settings.longBreakMinutes;
    if (preset === "default") return settings.focusMinutes;
    if (preset === "50") return 50;
    return Number(custom) || 0;
  })();

  const taskOptions: Option<string>[] = [
    { value: "", label: "No task" },
    ...(tasks ?? []).map((task) => ({ value: String(task.id), label: task.title })),
  ];
  const habitOptions: Option<string>[] = [
    { value: "", label: "No habit" },
    ...(habits ?? []).map((habit) => ({ value: String(habit.id), label: habit.name })),
  ];

  const start = () =>
    act("focus.start", {
      kind,
      minutes,
      taskId: kind === "focus" && taskId ? Number(taskId) : null,
      habitId: kind === "focus" && habitId ? Number(habitId) : null,
    });

  return (
    <div className="flex flex-col items-center gap-6 pt-[3vh]">
      <Segmented<Kind>
        label="Session type"
        value={kind}
        onChange={setKind}
        options={(Object.keys(KIND_LABEL) as Kind[]).map((value) => ({ value, label: KIND_LABEL[value] }))}
      />
      <Dial remaining={1} planned={1} paused={false}>
        <p className="text-label text-ink-2">{KIND_LABEL[kind]}</p>
        <p className="tabular text-[60px] font-[600] leading-[72px] tracking-[-0.03em]">{formatClock(minutes * 60)}</p>
        {kind !== "focus" && longBreakDue && kind === "short_break" && <p className="text-meta text-ink-2">A long break is due</p>}
      </Dial>
      {kind === "focus" && (
        <div className="flex w-full max-w-md flex-col gap-3">
          <div className="flex items-center justify-center gap-2">
            <Segmented
              label="Duration"
              value={preset}
              onChange={setPreset}
              options={[
                { value: "default", label: `${settings.focusMinutes} min` },
                { value: "50", label: "50 min" },
                { value: "custom", label: "Custom" },
              ]}
            />
            {preset === "custom" && (
              <Input
                type="number"
                min={1}
                max={480}
                aria-label="Minutes"
                value={custom}
                onChange={(event) => setCustom(event.target.value)}
                className="w-20"
              />
            )}
          </div>
          <div className="grid grid-cols-2 gap-2">
            <Select label="Task" value={taskId} options={taskOptions} onChange={setTaskId} className="w-full" />
            <Select label="Habit" value={habitId} options={habitOptions} onChange={setHabitId} className="w-full" />
          </div>
        </div>
      )}
      <Button variant="primary" icon={Play} disabled={minutes < 1 || minutes > 480} onClick={() => void start()} className="!h-11 !px-6 !text-heading">
        {kind === "focus" ? "Start focusing" : "Start break"}
      </Button>
    </div>
  );
}

function History({ stats }: { stats: FocusStats | undefined }) {
  const { data: sessions } = useRpc<FocusSession[]>("focus.history", { limit: 12 });
  return (
    <div className="mt-10 grid grid-cols-[minmax(0,1fr)_minmax(0,2fr)] gap-6 max-[900px]:grid-cols-1">
      <Section title="Your focus">
        <dl className="surface grid grid-cols-2 gap-4 p-4">
          {[
            ["Today", formatDuration(stats?.todaySeconds ?? 0)],
            ["Last 7 days", formatDuration(stats?.weekSeconds ?? 0)],
            ["All time", formatDuration(stats?.totalSeconds ?? 0)],
            ["Sessions completed", String(stats?.totalSessions ?? 0)],
          ].map(([label, value]) => (
            <div key={label}>
              <dd className="tabular text-heading font-semibold">{value}</dd>
              <dt className="text-meta text-ink-2">{label}</dt>
            </div>
          ))}
        </dl>
      </Section>
      <Section title="Session history">
        {sessions?.length === 0 ? (
          <div className="surface">
            <EmptyState compact icon={Timer} title="No sessions yet" body="Finished sessions are listed here." />
          </div>
        ) : (
          <ul className="surface row-list overflow-hidden">
            {sessions?.map((session) => (
              <li key={session.id} className="group flex items-center gap-3 px-4 py-2">
                <span className="min-w-0 flex-1">
                  <span className="block truncate text-body">{session.taskTitle ?? session.habitName ?? KIND_LABEL[session.kind]}</span>
                  <span className="block text-meta text-ink-2">
                    {formatInstant(session.startedAt)}
                    {session.status === "stopped" ? ". Stopped early" : ""}
                  </span>
                </span>
                <span className="tabular text-label text-ink-2">{formatDuration(session.elapsedSeconds)}</span>
                <span className="opacity-0 transition-opacity duration-100 focus-within:opacity-100 group-hover:opacity-100">
                  <IconButton size="sm" icon={Trash2} label="Delete this session" onClick={() => void act("focus.delete", { id: session.id })} />
                </span>
              </li>
            ))}
          </ul>
        )}
      </Section>
    </div>
  );
}

export function Focus() {
  const { data: session, loading } = useRpc<FocusSession | null>("focus.current");
  const { data: stats } = useRpc<FocusStats>("focus.stats");
  if (loading) return null;
  return (
    <div className="animate-page mx-auto max-w-[860px]">
      <h1 className="sr-only">Focus</h1>
      {session ? (
        <Running session={session} />
      ) : (
        <>
          <Setup stats={stats} />
          <History stats={stats} />
        </>
      )}
    </div>
  );
}
