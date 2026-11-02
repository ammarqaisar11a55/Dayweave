import { Plus, X } from "lucide-react";
import { type FormEvent, useState } from "react";

import { today } from "../../lib/date";
import { COLORS, colorVar, ICONS, softStyle } from "../../lib/palette";
import { mutate, showToast } from "../../lib/store";
import type { Habit, HabitFrequency, HabitInput, HabitKind } from "../../lib/types";
import { closeHabitDialog } from "../../lib/ui";
import { WeekdayPicker } from "../task/fields";
import { Button, IconButton } from "../ui/Button";
import { DatePicker } from "../ui/DatePicker";
import { Dialog } from "../ui/Dialog";
import { Field, Input, Segmented, Textarea } from "../ui/Form";
import { TimePicker } from "../ui/TimePicker";

const KINDS: { value: HabitKind; label: string; hint: string }[] = [
  { value: "build", label: "Build", hint: "Something to do, like exercising" },
  { value: "quit", label: "Quit", hint: "Something to avoid. Every day counts until you log a slip" },
  { value: "quantity", label: "Quantity", hint: "An amount to reach, like 20 pages" },
  { value: "checklist", label: "Checklist", hint: "Several steps that all need doing" },
];

export function ColorPicker({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  return (
    <div role="radiogroup" aria-label="Color" className="flex flex-wrap gap-1.5">
      {COLORS.map((color) => (
        <button
          key={color}
          type="button"
          role="radio"
          aria-checked={value === color}
          aria-label={color}
          onClick={() => onChange(color)}
          className={`size-7 rounded-full transition-transform duration-100 ${value === color ? "ring-2 ring-ink ring-offset-2 ring-offset-[var(--elevated)]" : "hover:scale-110"}`}
          style={{ background: colorVar(color) }}
        />
      ))}
    </div>
  );
}

export function IconPicker({ value, color, onChange }: { value: string; color: string; onChange: (value: string) => void }) {
  return (
    <div role="radiogroup" aria-label="Icon" className="grid grid-cols-11 gap-1">
      {Object.entries(ICONS).map(([name, Icon]) => (
        <button
          key={name}
          type="button"
          role="radio"
          aria-checked={value === name}
          aria-label={name.replace("-", " ")}
          onClick={() => onChange(name)}
          style={value === name ? softStyle(color) : undefined}
          className={`inline-flex size-8 items-center justify-center rounded-sm ${value === name ? "" : "interactive text-ink-2"}`}
        >
          <Icon size={16} aria-hidden />
        </button>
      ))}
    </div>
  );
}

export function HabitDialog({ habit }: { habit?: Habit }) {
  const [input, setInput] = useState<HabitInput>({
    name: habit?.name ?? "",
    description: habit?.description ?? "",
    icon: habit?.icon ?? "sparkles",
    color: habit?.color ?? "orange",
    kind: habit?.kind ?? "build",
    frequency: habit?.frequency ?? "daily",
    weekdays: habit?.weekdays ?? [0, 1, 2, 3, 4],
    weeklyTarget: habit?.weeklyTarget ?? 3,
    startDate: habit?.startDate ?? today(),
    reminderTime: habit?.reminderTime ?? null,
    quantityTarget: habit?.quantityTarget ?? null,
    unit: habit?.unit ?? "",
    checklist: habit?.checklist ?? [{ title: "" }],
  });
  const [saving, setSaving] = useState(false);
  const update = (patch: Partial<HabitInput>) => setInput((current) => ({ ...current, ...patch }));
  const kind = KINDS.find((item) => item.value === input.kind);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setSaving(true);
    try {
      await mutate(habit ? "habits.update" : "habits.create", { ...input, id: habit?.id });
      showToast(habit ? "Habit saved" : "Habit added");
      closeHabitDialog();
    } catch {
      setSaving(false);
    }
  };

  const setItem = (index: number, title: string) =>
    update({ checklist: input.checklist.map((item, position) => (position === index ? { ...item, title } : item)) });

  return (
    <Dialog title={habit ? "Edit habit" : "New habit"} onClose={closeHabitDialog} width="max-w-xl">
      <form onSubmit={(event) => void submit(event)} className="flex flex-col gap-4 pb-5">
        <Field label="Name">
          {(id) => (
            <Input
              id={id}
              data-autofocus
              value={input.name}
              placeholder="Read 20 pages"
              onChange={(event) => update({ name: event.target.value })}
            />
          )}
        </Field>
        <Field label="Type" hint={kind?.hint}>
          {() => (
            <Segmented
              label="Habit type"
              value={input.kind}
              options={KINDS}
              onChange={(value) =>
                update({ kind: value, frequency: value === "quit" && input.frequency === "weekly_target" ? "daily" : input.frequency })
              }
            />
          )}
        </Field>
        {input.kind === "quantity" && (
          <div className="grid grid-cols-2 gap-3">
            <Field label="Daily target">
              {(id) => (
                <Input
                  id={id}
                  type="number"
                  min={0}
                  step="any"
                  value={input.quantityTarget ?? ""}
                  placeholder="20"
                  onChange={(event) => update({ quantityTarget: event.target.value ? Number(event.target.value) : null })}
                />
              )}
            </Field>
            <Field label="Unit">
              {(id) => <Input id={id} value={input.unit} placeholder="pages" onChange={(event) => update({ unit: event.target.value })} />}
            </Field>
          </div>
        )}
        {input.kind === "checklist" && (
          <Field label="Steps">
            {() => (
              <div className="flex flex-col gap-1.5">
                {input.checklist.map((item, index) => (
                  <div key={item.id ?? `new-${index}`} className="flex items-center gap-1">
                    <Input value={item.title} aria-label={`Step ${index + 1}`} placeholder="Stretch" onChange={(event) => setItem(index, event.target.value)} />
                    <IconButton
                      icon={X}
                      label={`Remove step ${index + 1}`}
                      onClick={() => update({ checklist: input.checklist.filter((_, position) => position !== index) })}
                    />
                  </div>
                ))}
                <Button size="sm" variant="ghost" icon={Plus} className="w-fit" onClick={() => update({ checklist: [...input.checklist, { title: "" }] })}>
                  Add step
                </Button>
              </div>
            )}
          </Field>
        )}
        <Field label="Schedule">
          {() => (
            <div className="flex flex-col gap-2">
              <Segmented<HabitFrequency>
                label="Schedule"
                value={input.frequency}
                onChange={(frequency) => update({ frequency })}
                options={[
                  { value: "daily", label: "Every day" },
                  { value: "weekdays", label: "Set days" },
                  ...(input.kind === "quit" ? [] : [{ value: "weekly_target" as const, label: "Times a week" }]),
                ]}
              />
              {input.frequency === "weekdays" && <WeekdayPicker value={input.weekdays} onChange={(weekdays) => update({ weekdays })} />}
              {input.frequency === "weekly_target" && (
                <div className="flex items-center gap-2">
                  <Input
                    type="number"
                    min={1}
                    max={7}
                    aria-label="Times a week"
                    value={input.weeklyTarget ?? ""}
                    onChange={(event) => update({ weeklyTarget: Number(event.target.value) || null })}
                    className="w-20"
                  />
                  <span className="text-label text-ink-2">times a week, on any days</span>
                </div>
              )}
            </div>
          )}
        </Field>
        <div className="grid grid-cols-2 gap-3">
          <Field label="Start date">
            {() => <DatePicker value={input.startDate} onChange={(startDate) => update({ startDate })} label="Start date" clearable={false} className="w-full" />}
          </Field>
          <Field label="Reminder">
            {() => <TimePicker value={input.reminderTime} onChange={(reminderTime) => update({ reminderTime })} label="Reminder time" className="w-full" />}
          </Field>
        </div>
        <Field label="Color">{() => <ColorPicker value={input.color} onChange={(color) => update({ color })} />}</Field>
        <Field label="Icon">{() => <IconPicker value={input.icon} color={input.color} onChange={(icon) => update({ icon })} />}</Field>
        <Field label="Description">
          {(id) => <Textarea id={id} value={input.description} minRows={2} placeholder="Why this matters to you" onChange={(event) => update({ description: event.target.value })} />}
        </Field>
        <div className="flex justify-end gap-2 pt-1">
          <Button onClick={closeHabitDialog}>Cancel</Button>
          <Button type="submit" variant="primary" disabled={!input.name.trim() || saving}>
            {habit ? "Save habit" : "Add habit"}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
