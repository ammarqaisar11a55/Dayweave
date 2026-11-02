import { Plus, Repeat2 } from "lucide-react";
import { useState } from "react";

import { HabitCard } from "../components/habit/HabitCard";
import { Heatmap } from "../components/habit/Heatmap";
import { PageHeader, Section } from "../components/shell/PageHeader";
import { Button } from "../components/ui/Button";
import { EmptyState } from "../components/ui/EmptyState";
import { Switch } from "../components/ui/Form";
import { Select } from "../components/ui/Select";
import { plural } from "../lib/format";
import { useRpc } from "../lib/store";
import type { Habit } from "../lib/types";
import { newHabit } from "../lib/ui";

export function Habits() {
  const [archived, setArchived] = useState(false);
  const [selected, setSelected] = useState("");
  const { data: habits } = useRpc<Habit[]>("habits.list", { includeArchived: archived });
  const due = habits?.filter((habit) => habit.today.due && !habit.archivedAt) ?? [];
  const others = habits?.filter((habit) => !habit.today.due || habit.archivedAt) ?? [];
  const options = [
    { value: "", label: "All habits" },
    ...(habits ?? []).map((habit) => ({ value: String(habit.id), label: habit.name })),
  ];

  return (
    <div className="animate-page mx-auto max-w-[1000px]">
      <PageHeader
        title="Habits"
        subtitle={habits?.length ? `${plural(due.length, "habit")} due today` : undefined}
        actions={
          <>
            <label className="flex items-center gap-2 text-label text-ink-2">
              Show archived
              <Switch checked={archived} onChange={setArchived} label="Show archived habits" />
            </label>
            <Button variant="primary" icon={Plus} onClick={newHabit}>
              New habit
            </Button>
          </>
        }
      />
      {habits?.length === 0 ? (
        <div className="surface">
          <EmptyState
            icon={Repeat2}
            title="No habits yet"
            body="Start small. A habit can be something to do, to avoid, to measure, or a short routine."
            action={
              <Button variant="primary" icon={Plus} onClick={newHabit}>
                Add a habit
              </Button>
            }
          />
        </div>
      ) : (
        <div className="flex flex-col gap-6">
          {due.length > 0 && (
            <Section title="Due today">
              <div className="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-3">
                {due.map((habit) => (
                  <HabitCard key={habit.id} habit={habit} detailed />
                ))}
              </div>
            </Section>
          )}
          {others.length > 0 && (
            <Section title={archived ? "Not due today and archived" : "Not due today"}>
              <div className="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-3">
                {others.map((habit) => (
                  <HabitCard key={habit.id} habit={habit} detailed />
                ))}
              </div>
            </Section>
          )}
          {habits && (
            <Section
              title="Activity"
              action={<Select size="sm" label="Show activity for" value={selected} options={options} onChange={setSelected} className="w-48" />}
            >
              <div className="surface p-4">
                <Heatmap habitId={selected ? Number(selected) : null} />
              </div>
            </Section>
          )}
        </div>
      )}
    </div>
  );
}
