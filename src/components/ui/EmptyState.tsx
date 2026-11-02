import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

export function EmptyState({
  icon: Icon,
  title,
  body,
  action,
  compact = false,
}: {
  icon: LucideIcon;
  title: string;
  body?: string;
  action?: ReactNode;
  compact?: boolean;
}) {
  return (
    <div className={`flex flex-col items-center px-6 text-center ${compact ? "py-8" : "py-16"}`}>
      <span className="mb-3 inline-flex size-11 items-center justify-center rounded-md bg-sunken text-ink-2">
        <Icon size={20} aria-hidden />
      </span>
      <p className="text-heading font-semibold">{title}</p>
      {body && <p className="mt-1 max-w-sm text-label text-ink-2">{body}</p>}
      {action && <div className="mt-4">{action}</div>}
    </div>
  );
}
