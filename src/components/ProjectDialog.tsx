import { type FormEvent, useState } from "react";

import { mutate, showToast } from "../lib/store";
import type { Project } from "../lib/types";
import { closeProjectDialog } from "../lib/ui";
import { ColorPicker, IconPicker } from "./habit/HabitDialog";
import { Button } from "./ui/Button";
import { DatePicker } from "./ui/DatePicker";
import { Dialog } from "./ui/Dialog";
import { Field, Input, Textarea } from "./ui/Form";

export function ProjectDialog({ project }: { project?: Project }) {
  const [name, setName] = useState(project?.name ?? "");
  const [description, setDescription] = useState(project?.description ?? "");
  const [color, setColor] = useState(project?.color ?? "orange");
  const [icon, setIcon] = useState(project?.icon ?? "folder");
  const [startDate, setStartDate] = useState(project?.startDate ?? null);
  const [deadline, setDeadline] = useState(project?.deadline ?? null);
  const [saving, setSaving] = useState(false);

  const submit = async (event: FormEvent) => {
    event.preventDefault();
    setSaving(true);
    try {
      const input = { id: project?.id, name, description, color, icon, startDate, deadline };
      await mutate(project ? "projects.update" : "projects.create", input);
      showToast(project ? "Project saved" : "Project added");
      closeProjectDialog();
    } catch {
      setSaving(false);
    }
  };

  return (
    <Dialog title={project ? "Edit project" : "New project"} onClose={closeProjectDialog} width="max-w-lg">
      <form onSubmit={(event) => void submit(event)} className="flex flex-col gap-4 pb-5">
        <Field label="Name">
          {(id) => <Input id={id} data-autofocus value={name} placeholder="Final year thesis" onChange={(event) => setName(event.target.value)} />}
        </Field>
        <Field label="Description">
          {(id) => <Textarea id={id} value={description} minRows={2} onChange={(event) => setDescription(event.target.value)} />}
        </Field>
        <div className="grid grid-cols-2 gap-3">
          <Field label="Start date">{() => <DatePicker value={startDate} onChange={setStartDate} label="Start date" className="w-full" />}</Field>
          <Field label="Deadline">{() => <DatePicker value={deadline} onChange={setDeadline} label="Deadline" className="w-full" />}</Field>
        </div>
        <Field label="Color">{() => <ColorPicker value={color} onChange={setColor} />}</Field>
        <Field label="Icon">{() => <IconPicker value={icon} color={color} onChange={setIcon} />}</Field>
        <div className="flex justify-end gap-2 pt-1">
          <Button onClick={closeProjectDialog}>Cancel</Button>
          <Button type="submit" variant="primary" disabled={!name.trim() || saving}>
            {project ? "Save project" : "Add project"}
          </Button>
        </div>
      </form>
    </Dialog>
  );
}
