import { CornerDownRight, Plus, X } from "lucide-react";
import { useState } from "react";

import { act } from "../../lib/store";
import type { Subtask, Task } from "../../lib/types";
import { IconButton } from "../ui/Button";
import { Checkbox, Input } from "../ui/Form";

const MAX_DEPTH = 3;

function AddSubtask({ taskId, parentId, onDone }: { taskId: number; parentId: number | null; onDone?: () => void }) {
  const [title, setTitle] = useState("");
  const submit = () => {
    const clean = title.trim();
    if (clean) void act("subtasks.add", { taskId, parentId, title: clean });
    setTitle("");
    if (parentId !== null) onDone?.();
  };
  return (
    <div className="flex items-center gap-2">
      <Plus size={15} aria-hidden className="ml-px shrink-0 text-ink-2" />
      <Input
        autoFocus={parentId !== null}
        value={title}
        placeholder={parentId === null ? "Add a subtask" : "Add a nested subtask"}
        aria-label={parentId === null ? "Add a subtask" : "Add a nested subtask"}
        onChange={(event) => setTitle(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            submit();
          } else if (event.key === "Escape" && parentId !== null) {
            event.stopPropagation();
            onDone?.();
          }
        }}
        onBlur={() => parentId !== null && !title.trim() && onDone?.()}
        className="h-8"
      />
    </div>
  );
}

function SubtaskItem({ subtask, all, taskId, depth }: { subtask: Subtask; all: Subtask[]; taskId: number; depth: number }) {
  const [title, setTitle] = useState(subtask.title);
  const [nesting, setNesting] = useState(false);
  const children = all.filter((item) => item.parentId === subtask.id);

  const rename = () => {
    const clean = title.trim();
    if (clean && clean !== subtask.title) void act("subtasks.rename", { id: subtask.id, title: clean });
    else setTitle(subtask.title);
  };

  return (
    <li>
      <div className="group flex items-center gap-2 rounded-sm py-0.5">
        <Checkbox
          checked={subtask.completed}
          onChange={() => void act("subtasks.toggle", { id: subtask.id })}
          label={`${subtask.completed ? "Reopen" : "Complete"} ${subtask.title}`}
        />
        <input
          value={title}
          aria-label="Subtask title"
          onChange={(event) => setTitle(event.target.value)}
          onBlur={rename}
          onKeyDown={(event) => {
            if (event.key === "Enter") event.currentTarget.blur();
          }}
          className={`h-7 min-w-0 flex-1 rounded-[6px] bg-transparent px-1 text-body outline-none focus:bg-sunken ${subtask.completed ? "text-ink-2 line-through" : "text-ink"}`}
        />
        <div className="flex opacity-0 transition-opacity duration-100 focus-within:opacity-100 group-hover:opacity-100">
          {depth < MAX_DEPTH && (
            <IconButton size="sm" icon={CornerDownRight} label="Add a nested subtask" onClick={() => setNesting(true)} />
          )}
          <IconButton size="sm" icon={X} label={`Delete ${subtask.title}`} onClick={() => void act("subtasks.delete", { id: subtask.id })} />
        </div>
      </div>
      {(children.length > 0 || nesting) && (
        <ul className="ml-[26px]">
          {children.map((child) => (
            <SubtaskItem key={child.id} subtask={child} all={all} taskId={taskId} depth={depth + 1} />
          ))}
          {nesting && <AddSubtask taskId={taskId} parentId={subtask.id} onDone={() => setNesting(false)} />}
        </ul>
      )}
    </li>
  );
}

export function SubtaskList({ task }: { task: Task }) {
  const roots = task.subtasks.filter((subtask) => subtask.parentId === null);
  const done = task.subtaskDone;
  const total = task.subtaskTotal;
  return (
    <div className="flex flex-col gap-1.5">
      {total > 0 && (
        <div className="flex items-center gap-2" aria-label={`${done} of ${total} subtasks complete`}>
          <div className="h-1 flex-1 overflow-hidden rounded-full bg-sunken">
            <div
              className="h-full rounded-full bg-accent transition-[width] duration-200 ease-weave"
              style={{ width: `${(done / total) * 100}%` }}
            />
          </div>
          <span className="tabular text-meta text-ink-2">{Math.round((done / total) * 100)}%</span>
        </div>
      )}
      <ul>
        {roots.map((subtask) => (
          <SubtaskItem key={subtask.id} subtask={subtask} all={task.subtasks} taskId={task.id} depth={1} />
        ))}
      </ul>
      <AddSubtask taskId={task.id} parentId={null} />
    </div>
  );
}
