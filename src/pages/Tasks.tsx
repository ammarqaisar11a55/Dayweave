import {
  ArrowDownNarrowWide,
  ArrowUpNarrowWide,
  CheckCircle2,
  FolderKanban,
  ListFilter,
  Plus,
  Search,
  X,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { PRIORITY } from "../components/Badges";
import { PageHeader } from "../components/shell/PageHeader";
import { useProjectOptions } from "../components/task/TaskDialog";
import { TaskRow } from "../components/task/TaskRow";
import { Button, IconButton } from "../components/ui/Button";
import { DatePicker } from "../components/ui/DatePicker";
import { EmptyState } from "../components/ui/EmptyState";
import { Checkbox, Field, Input, Segmented } from "../components/ui/Form";
import { Popover } from "../components/ui/Popover";
import { Select } from "../components/ui/Select";
import { formatShortDate, today } from "../lib/date";
import { navigate, useRoute } from "../lib/router";
import { setSetting, useSettings } from "../lib/settings";
import { reschedule } from "../lib/taskActions";
import { useRpc } from "../lib/store";
import type { Priority, Task, TaskCounts, TaskSort, TaskStatus, TaskView } from "../lib/types";
import { newTask } from "../lib/ui";

const VIEWS: { value: TaskView; label: string }[] = [
  { value: "all", label: "All" },
  { value: "today", label: "Today" },
  { value: "upcoming", label: "Upcoming" },
  { value: "overdue", label: "Overdue" },
  { value: "completed", label: "Completed" },
  { value: "archived", label: "Archived" },
];

const SORTS: { value: TaskSort; label: string }[] = [
  { value: "deadline", label: "Deadline" },
  { value: "priority", label: "Priority" },
  { value: "created", label: "Date created" },
  { value: "status", label: "Status" },
  { value: "title", label: "Name" },
];

const STATUSES: { value: TaskStatus; label: string }[] = [
  { value: "todo", label: "To do" },
  { value: "in_progress", label: "In progress" },
  { value: "completed", label: "Completed" },
];

const EMPTY: Record<TaskView, { title: string; body: string }> = {
  all: { title: "No tasks yet", body: "Add your first task to start planning." },
  today: { title: "Nothing due today", body: "Tasks due or starting today appear here." },
  upcoming: { title: "Nothing coming up", body: "Tasks with a future deadline appear here." },
  overdue: { title: "Nothing overdue", body: "You're on top of your deadlines." },
  completed: { title: "Nothing completed yet", body: "Finished tasks stay here as a record." },
  archived: { title: "Nothing archived", body: "Archive tasks to keep them without seeing them." },
};

interface Filters {
  priorities: Priority[];
  statuses: TaskStatus[];
  category: string;
  tag: string;
  dateFrom: string | null;
  dateTo: string | null;
}

const NO_FILTERS: Filters = { priorities: [], statuses: [], category: "", tag: "", dateFrom: null, dateTo: null };

function toggled<T>(list: T[], value: T) {
  return list.includes(value) ? list.filter((item) => item !== value) : [...list, value];
}

function FilterPanel({ filters, onChange }: { filters: Filters; onChange: (filters: Filters) => void }) {
  const { data: categories } = useRpc<string[]>("tasks.categories");
  const { data: tags } = useRpc<{ name: string }[]>("tasks.tags");
  const update = (patch: Partial<Filters>) => onChange({ ...filters, ...patch });
  return (
    <div className="flex w-[300px] flex-col gap-4 p-4">
      <fieldset>
        <legend className="mb-1.5 text-label font-medium text-ink-2">Priority</legend>
        <div className="grid grid-cols-2 gap-1.5">
          {(["urgent", "high", "medium", "low"] as const).map((priority) => (
            <label key={priority} className="flex items-center gap-2 text-body">
              <Checkbox
                checked={filters.priorities.includes(priority)}
                onChange={() => update({ priorities: toggled(filters.priorities, priority) })}
                label={PRIORITY[priority].label}
              />
              {PRIORITY[priority].label}
            </label>
          ))}
        </div>
      </fieldset>
      <fieldset>
        <legend className="mb-1.5 text-label font-medium text-ink-2">Status</legend>
        <div className="flex flex-col gap-1.5">
          {STATUSES.map(({ value, label }) => (
            <label key={value} className="flex items-center gap-2 text-body">
              <Checkbox
                checked={filters.statuses.includes(value)}
                onChange={() => update({ statuses: toggled(filters.statuses, value) })}
                label={label}
              />
              {label}
            </label>
          ))}
        </div>
      </fieldset>
      <div className="grid grid-cols-2 gap-3">
        <Field label="Category">
          {() => (
            <Select
              label="Category"
              value={filters.category}
              onChange={(category) => update({ category })}
              className="w-full"
              options={[{ value: "", label: "Any" }, ...(categories ?? []).map((value) => ({ value, label: value }))]}
            />
          )}
        </Field>
        <Field label="Tag">
          {() => (
            <Select
              label="Tag"
              value={filters.tag}
              onChange={(tag) => update({ tag })}
              className="w-full"
              options={[{ value: "", label: "Any" }, ...(tags ?? []).map(({ name }) => ({ value: name, label: name }))]}
            />
          )}
        </Field>
        <Field label="Due from">
          {() => <DatePicker value={filters.dateFrom} onChange={(dateFrom) => update({ dateFrom })} label="Due from" placeholder="Any" className="w-full" />}
        </Field>
        <Field label="Due until">
          {() => <DatePicker value={filters.dateTo} onChange={(dateTo) => update({ dateTo })} label="Due until" placeholder="Any" className="w-full" />}
        </Field>
      </div>
      <Button size="sm" variant="ghost" onClick={() => onChange(NO_FILTERS)}>
        Clear filters
      </Button>
    </div>
  );
}

function Chip({ label, onRemove }: { label: string; onRemove: () => void }) {
  return (
    <span className="control inline-flex h-7 items-center gap-1 rounded-sm pl-2.5 pr-1 text-meta">
      {label}
      <button type="button" aria-label={`Remove filter: ${label}`} onClick={onRemove} className="inline-flex size-5 items-center justify-center rounded-[5px] text-ink-2 hover:text-ink">
        <X size={12} aria-hidden />
      </button>
    </span>
  );
}

export function Tasks() {
  const { query } = useRoute();
  const { taskDensity } = useSettings();
  const routeView = (query.get("view") as TaskView | null) ?? "all";
  const routeTag = query.get("tag");
  const projectFilter = query.get("project");
  const [search, setSearch] = useState("");
  const [filters, setFilters] = useState<Filters>({ ...NO_FILTERS, tag: routeTag ?? "" });
  const [sort, setSort] = useState<TaskSort>("deadline");
  const [descending, setDescending] = useState(false);
  const [filtersOpen, setFiltersOpen] = useState(false);
  const filterButton = useRef<HTMLButtonElement>(null);
  const searchBox = useRef<HTMLInputElement>(null);
  const projects = useProjectOptions();

  useEffect(() => {
    if (routeTag) setFilters((current) => ({ ...current, tag: routeTag }));
  }, [routeTag]);

  useEffect(() => {
    const focusSearch = () => searchBox.current?.focus();
    window.addEventListener("dayweave:focus-search", focusSearch);
    return () => window.removeEventListener("dayweave:focus-search", focusSearch);
  }, []);

  const { data: tasks } = useRpc<Task[]>("tasks.list", {
    view: routeView,
    search: search || undefined,
    priorities: filters.priorities,
    statuses: filters.statuses,
    projectId: projectFilter ? Number(projectFilter) : undefined,
    category: filters.category || undefined,
    tag: filters.tag || undefined,
    dateFrom: filters.dateFrom ?? undefined,
    dateTo: filters.dateTo ?? undefined,
    sort,
    descending,
  });
  const { data: counts } = useRpc<TaskCounts>("tasks.counts");

  const setView = (view: TaskView) => navigate("tasks", view === "all" ? {} : { view });
  const project = projects.find((option) => option.value === projectFilter);
  const chips = [
    ...filters.priorities.map((priority) => ({
      key: priority,
      label: PRIORITY[priority].label,
      remove: () => setFilters({ ...filters, priorities: toggled(filters.priorities, priority) }),
    })),
    ...filters.statuses.map((status) => ({
      key: status,
      label: STATUSES.find((item) => item.value === status)?.label ?? status,
      remove: () => setFilters({ ...filters, statuses: toggled(filters.statuses, status) }),
    })),
    ...(filters.category ? [{ key: "category", label: filters.category, remove: () => setFilters({ ...filters, category: "" }) }] : []),
    ...(filters.tag ? [{ key: "tag", label: `#${filters.tag}`, remove: () => setFilters({ ...filters, tag: "" }) }] : []),
    ...(filters.dateFrom ? [{ key: "from", label: `From ${formatShortDate(filters.dateFrom)}`, remove: () => setFilters({ ...filters, dateFrom: null }) }] : []),
    ...(filters.dateTo ? [{ key: "to", label: `Until ${formatShortDate(filters.dateTo)}`, remove: () => setFilters({ ...filters, dateTo: null }) }] : []),
  ];
  const filtered = chips.length > 0 || Boolean(search) || Boolean(projectFilter);
  const empty = EMPTY[routeView];

  return (
    <div className="animate-page mx-auto max-w-[1000px]">
      <PageHeader
        title={project && projectFilter ? project.label : "Tasks"}
        subtitle={project && projectFilter ? "Tasks in this project" : undefined}
        actions={
          <>
            {projectFilter && (
              <Button icon={FolderKanban} onClick={() => navigate("projects")}>
                All projects
              </Button>
            )}
            {routeView === "overdue" && tasks && tasks.length > 0 && (
              <Button onClick={() => tasks.forEach((task) => void reschedule(task.id, today(), task.dueTime))}>Move all to today</Button>
            )}
          </>
        }
      />
      <Segmented
        label="Task view"
        value={routeView}
        onChange={setView}
        className="mb-3 max-w-full overflow-x-auto"
        options={VIEWS.map((view) => ({ ...view, count: view.value === "completed" || view.value === "archived" ? undefined : counts?.[view.value] }))}
      />
      <div className="mb-3 flex flex-wrap items-center gap-2">
        <div className="relative min-w-52 flex-1">
          <Search size={15} aria-hidden className="absolute left-3 top-1/2 -translate-y-1/2 text-ink-2" />
          <Input
            ref={searchBox}
            value={search}
            onChange={(event) => setSearch(event.target.value)}
            placeholder="Search these tasks"
            aria-label="Search tasks"
            className="!bg-card pl-9 shadow-[inset_0_0_0_1px_var(--line)]"
          />
        </div>
        <Button ref={filterButton} icon={ListFilter} aria-expanded={filtersOpen} onClick={() => setFiltersOpen((open) => !open)}>
          Filters{chips.length ? ` (${chips.length})` : ""}
        </Button>
        {filtersOpen && (
          <Popover anchor={filterButton} onClose={() => setFiltersOpen(false)} label="Filters">
            <FilterPanel filters={filters} onChange={setFilters} />
          </Popover>
        )}
        <Select label="Sort by" value={sort} options={SORTS} onChange={setSort} className="w-40" />
        <IconButton
          icon={descending ? ArrowDownNarrowWide : ArrowUpNarrowWide}
          label={descending ? "Sorted descending" : "Sorted ascending"}
          onClick={() => setDescending((value) => !value)}
        />
        <Segmented
          label="Density"
          value={taskDensity}
          onChange={(value) => void setSetting("taskDensity", value)}
          options={[
            { value: "comfortable", label: "List" },
            { value: "compact", label: "Compact" },
          ]}
        />
      </div>
      {(chips.length > 0 || projectFilter) && (
        <div className="mb-3 flex flex-wrap items-center gap-1.5">
          {projectFilter && project && <Chip label={project.label} onRemove={() => navigate("tasks")} />}
          {chips.map((chip) => (
            <Chip key={chip.key} label={chip.label} onRemove={chip.remove} />
          ))}
        </div>
      )}
      {tasks && (
        <div className="surface row-list overflow-hidden">
          {tasks.map((task) => (
            <TaskRow key={task.id} task={task} showProject={!projectFilter} />
          ))}
          {tasks.length === 0 && (
            <EmptyState
              icon={filtered ? Search : CheckCircle2}
              title={filtered ? "No matching tasks" : empty.title}
              body={filtered ? "Try a different search, or clear the filters." : empty.body}
              action={
                !filtered && routeView !== "archived" && routeView !== "completed" ? (
                  <Button variant="primary" icon={Plus} onClick={() => newTask(routeView === "today" ? { dueDate: today() } : {})}>
                    Add a task
                  </Button>
                ) : undefined
              }
            />
          )}
        </div>
      )}
    </div>
  );
}
