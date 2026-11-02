import type { ReactNode } from "react";

export function PageHeader({ title, subtitle, actions }: { title: string; subtitle?: string; actions?: ReactNode }) {
  return (
    <header className="mb-6 flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
      <div className="min-w-0">
        <h1 className="text-page font-[650]">{title}</h1>
        {subtitle && <p className="mt-1 text-body text-ink-2">{subtitle}</p>}
      </div>
      {actions && <div className="flex items-center gap-2">{actions}</div>}
    </header>
  );
}

export function Section({
  title,
  action,
  children,
  className = "",
}: {
  title: string;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <section aria-label={title} className={className}>
      <div className="mb-2 flex min-h-8 items-center justify-between gap-3">
        <h2 className="text-section font-semibold">{title}</h2>
        {action}
      </div>
      {children}
    </section>
  );
}
