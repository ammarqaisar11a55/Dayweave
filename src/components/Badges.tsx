import { AlertCircle, Check, ChevronDown, ChevronUp, ChevronsUp, Clock, Equal, type LucideIcon } from "lucide-react";

import { describeDeadline, type DeadlineTone } from "../lib/deadline";
import { useSettings } from "../lib/settings";
import type { Priority, Task } from "../lib/types";
import { Tooltip } from "./ui/Tooltip";

const TONES: Record<DeadlineTone, { className: string; icon: LucideIcon | null }> = {
  overdue: { className: "text-danger bg-danger/10", icon: AlertCircle },
  today: { className: "text-accent-text bg-accent-soft", icon: Clock },
  soon: { className: "text-warning", icon: null },
  later: { className: "text-ink-2", icon: null },
  done: { className: "text-success", icon: Check },
  none: { className: "", icon: null },
};

export function DeadlineBadge({ task }: { task: Pick<Task, "dueDate" | "dueTime" | "status"> }) {
  const { timeFormat } = useSettings();
  const { label, detail, tone } = describeDeadline(task, new Date(), timeFormat);
  if (!label) return null;
  const { className, icon: Icon } = TONES[tone];
  return (
    <Tooltip text={detail}>
      <span className={`tabular inline-flex h-6 shrink-0 items-center gap-1 whitespace-nowrap rounded-sm px-1.5 text-meta font-medium ${className}`}>
        {Icon && <Icon size={13} aria-hidden />}
        {label}
        <span className="sr-only">{detail}</span>
      </span>
    </Tooltip>
  );
}

export const PRIORITY: Record<Priority, { label: string; icon: LucideIcon; className: string }> = {
  low: { label: "Low", icon: ChevronDown, className: "text-ink-2" },
  medium: { label: "Medium", icon: Equal, className: "text-info" },
  high: { label: "High", icon: ChevronUp, className: "text-warning" },
  urgent: { label: "Urgent", icon: ChevronsUp, className: "text-danger" },
};

export function PriorityBadge({ priority, showLabel = false }: { priority: Priority; showLabel?: boolean }) {
  const { label, icon: Icon, className } = PRIORITY[priority];
  return (
    <Tooltip text={`${label} priority`}>
      <span className={`inline-flex shrink-0 items-center gap-1 text-meta font-medium ${className}`}>
        <Icon size={15} aria-hidden />
        <span className={showLabel ? "" : "sr-only"}>{showLabel ? label : `${label} priority`}</span>
      </span>
    </Tooltip>
  );
}
