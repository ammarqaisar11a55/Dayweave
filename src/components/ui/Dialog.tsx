import { X } from "lucide-react";
import { type KeyboardEvent, type ReactNode, useEffect, useId, useRef } from "react";
import { createPortal } from "react-dom";

import { createStore } from "../../lib/store";
import { Button, IconButton } from "./Button";

const FOCUSABLE =
  'a[href], button:not([disabled]), input:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

interface DialogProps {
  title: string;
  onClose: () => void;
  children: ReactNode;
  footer?: ReactNode;
  width?: string;
  /** Hides the title visually while keeping it for screen readers. */
  bare?: boolean;
  align?: "center" | "top";
}

export function Dialog({ title, onClose, children, footer, width = "max-w-lg", bare = false, align = "center" }: DialogProps) {
  const panel = useRef<HTMLDivElement>(null);
  const titleId = useId();

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const node = panel.current;
    if (node && !node.contains(document.activeElement)) {
      (node.querySelector<HTMLElement>("[data-autofocus]") ?? node.querySelector<HTMLElement>(FOCUSABLE))?.focus();
    }
    return () => previous?.focus?.();
  }, []);

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === "Escape") {
      event.stopPropagation();
      onClose();
      return;
    }
    if (event.key !== "Tab") return;
    const items = Array.from(panel.current?.querySelectorAll<HTMLElement>(FOCUSABLE) ?? []);
    const first = items[0];
    const last = items[items.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last?.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first?.focus();
    }
  };

  return createPortal(
    <div
      className={`animate-fade fixed inset-0 z-50 flex justify-center bg-[var(--overlay)] p-6 backdrop-blur-[2px] ${align === "top" ? "items-start pt-[12vh]" : "items-center"}`}
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        ref={panel}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onKeyDown={onKeyDown}
        className={`float animate-rise flex max-h-full w-full flex-col overflow-hidden rounded-xl ${width}`}
      >
        {bare ? (
          <h2 id={titleId} className="sr-only">
            {title}
          </h2>
        ) : (
          <div className="flex items-center justify-between gap-3 px-6 pb-2 pt-5">
            <h2 id={titleId} className="text-section font-semibold">
              {title}
            </h2>
            <IconButton size="sm" icon={X} label="Close" onClick={onClose} />
          </div>
        )}
        <div className={`min-h-0 flex-1 overflow-y-auto ${bare ? "" : "px-6 py-3"}`}>{children}</div>
        {footer && <div className="flex items-center justify-end gap-2 px-6 pb-5 pt-3">{footer}</div>}
      </div>
    </div>,
    document.body,
  );
}

interface ConfirmRequest {
  title: string;
  body: string;
  confirmLabel: string;
  danger: boolean;
  resolve: (confirmed: boolean) => void;
}

const request = createStore<ConfirmRequest | null>(null);

/** Asks a question and resolves with the answer. */
export function confirm(options: Omit<ConfirmRequest, "resolve" | "danger"> & { danger?: boolean }) {
  return new Promise<boolean>((resolve) => request.set({ danger: true, ...options, resolve }));
}

export function ConfirmDialog() {
  const current = request.use();
  if (!current) return null;
  const answer = (confirmed: boolean) => {
    current.resolve(confirmed);
    request.set(null);
  };
  return (
    <Dialog
      title={current.title}
      onClose={() => answer(false)}
      width="max-w-sm"
      footer={
        <>
          <Button onClick={() => answer(false)}>Cancel</Button>
          <Button variant={current.danger ? "danger" : "primary"} data-autofocus onClick={() => answer(true)}>
            {current.confirmLabel}
          </Button>
        </>
      }
    >
      <p className="text-body text-ink-2">{current.body}</p>
    </Dialog>
  );
}
