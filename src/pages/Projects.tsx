import { Archive, ArchiveRestore, FolderKanban, MoreHorizontal, Pencil, Plus, Trash2 } from "lucide-react";
import { useState } from "react";

import { PageHeader } from "../components/shell/PageHeader";
import { Button, IconButton } from "../components/ui/Button";
import { confirm } from "../components/ui/Dialog";
import { EmptyState } from "../components/ui/EmptyState";
import { Switch } from "../components/ui/Form";
import { Menu } from "../components/ui/Menu";
import { formatShortDate, today } from "../lib/date";
import { percent, plural } from "../lib/format";
import { colorVar, iconFor, softStyle } from "../lib/palette";
import { navigate } from "../lib/router";
import { useSettings } from "../lib/settings";
import { act, useRpc } from "../lib/store";
import type { Project } from "../lib/types";
import { editProject, newProject, newTask } from "../lib/ui";

function ProjectCard({ project }: { project: Project }) {
  const { confirmDelete } = useSettings();
  const Icon = iconFor(project.icon);
  const late = project.deadline && project.deadline < today() && project.taskOpen > 0;

  const remove = async () => {
    if (confirmDelete) {
      const confirmed = await confirm({
        title: "Delete this project?",
        body: `"${project.name}" will be deleted. Its tasks stay in your task list without a project.`,
        confirmLabel: "Delete project",
      });
      if (!confirmed) return;
    }
    await act("projects.delete", { id: project.id }, "Project deleted");
  };

  return (
    <article className="surface group flex flex-col gap-3 p-4">
      <div className="flex items-start gap-3">
        <span style={softStyle(project.color)} className="inline-flex size-10 shrink-0 items-center justify-center rounded-md">
          <Icon size={20} aria-hidden />
        </span>
        <div className="min-w-0 flex-1">
          <button
            type="button"
            onClick={() => navigate("tasks", { project: String(project.id) })}
            className="block max-w-full truncate text-left text-heading font-semibold"
          >
            {project.name}
          </button>
          <p className="truncate text-label text-ink-2">{project.description || plural(project.taskTotal, "task")}</p>
        </div>
        <Menu
          label={`Actions for ${project.name}`}
          items={[
            { label: "Add task", icon: Plus, onSelect: () => newTask({ projectId: project.id }) },
            { label: "Edit", icon: Pencil, onSelect: () => editProject(project) },
            project.archivedAt
              ? { label: "Restore", icon: ArchiveRestore, onSelect: () => void act("projects.restore", { id: project.id }, "Project restored") }
              : { label: "Archive", icon: Archive, onSelect: () => void act("projects.archive", { id: project.id }, "Project archived") },
            { separator: true },
            { label: "Delete", icon: Trash2, danger: true, onSelect: () => void remove() },
          ]}
          trigger={<IconButton size="sm" icon={MoreHorizontal} label="More actions" />}
        />
      </div>
      <div>
        <div className="mb-1 flex items-baseline justify-between text-meta text-ink-2">
          <span className="tabular">
            {project.taskDone} of {plural(project.taskTotal, "task")} done
          </span>
          <span className="tabular font-medium text-ink">{percent(project.completion)}</span>
        </div>
        <div className="h-1.5 overflow-hidden rounded-full bg-sunken">
          <div
            className="h-full rounded-full transition-[width] duration-200 ease-weave"
            style={{ width: `${project.completion * 100}%`, background: colorVar(project.color) }}
          />
        </div>
      </div>
      <div className="flex min-h-4 items-center gap-3 text-meta text-ink-2">
        {project.deadline && <span className={late ? "font-medium text-danger" : ""}>Deadline {formatShortDate(project.deadline)}</span>}
        {project.taskOverdue > 0 && <span className="font-medium text-danger">{project.taskOverdue} overdue</span>}
      </div>
    </article>
  );
}

export function Projects() {
  const [archived, setArchived] = useState(false);
  const { data: projects } = useRpc<Project[]>("projects.list", { includeArchived: archived });
  return (
    <div className="animate-page mx-auto max-w-[1000px]">
      <PageHeader
        title="Projects"
        subtitle="Group related tasks and follow their progress."
        actions={
          <>
            <label className="flex items-center gap-2 text-label text-ink-2">
              Show archived
              <Switch checked={archived} onChange={setArchived} label="Show archived projects" />
            </label>
            <Button variant="primary" icon={Plus} onClick={newProject}>
              New project
            </Button>
          </>
        }
      />
      {projects?.length === 0 ? (
        <div className="surface">
          <EmptyState
            icon={FolderKanban}
            title="No projects yet"
            body="A project gathers the tasks behind something bigger, with its own deadline."
            action={
              <Button variant="primary" icon={Plus} onClick={newProject}>
                Add a project
              </Button>
            }
          />
        </div>
      ) : (
        <div className="grid grid-cols-[repeat(auto-fill,minmax(300px,1fr))] gap-3">
          {projects?.map((project) => (
            <ProjectCard key={project.id} project={project} />
          ))}
        </div>
      )}
    </div>
  );
}
