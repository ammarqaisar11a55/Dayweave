import { X } from "lucide-react";

import { dismissToast, toasts } from "../../lib/store";

export function Toasts() {
  const list = toasts.use();
  return (
    <div
      role="status"
      aria-live="polite"
      className="pointer-events-none fixed bottom-5 left-1/2 z-[80] flex -translate-x-1/2 flex-col items-center gap-2"
    >
      {list.map((toast) => (
        <div
          key={toast.id}
          className="float animate-rise pointer-events-auto flex max-w-md items-center gap-3 rounded-md py-2 pl-4 pr-2 text-body"
        >
          <span className={toast.tone === "error" ? "text-danger" : "text-ink"}>{toast.message}</span>
          {toast.action && (
            <button
              type="button"
              onClick={() => {
                toast.action?.run();
                dismissToast(toast.id);
              }}
              className="interactive rounded-sm px-2 py-1 text-label font-semibold text-accent-text"
            >
              {toast.action.label}
            </button>
          )}
          <button
            type="button"
            aria-label="Dismiss"
            onClick={() => dismissToast(toast.id)}
            className="interactive inline-flex size-7 items-center justify-center rounded-sm text-ink-2"
          >
            <X size={14} aria-hidden />
          </button>
        </div>
      ))}
    </div>
  );
}
