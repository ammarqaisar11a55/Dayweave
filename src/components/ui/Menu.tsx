import type { LucideIcon } from "lucide-react";
import { type KeyboardEvent, type ReactElement, type Ref, cloneElement, useEffect, useRef, useState } from "react";

import { Popover } from "./Popover";

export type MenuItem =
  | { separator: true }
  | {
      separator?: false;
      label: string;
      icon?: LucideIcon;
      danger?: boolean;
      hint?: string;
      onSelect: () => void;
    };

interface TriggerProps {
  ref?: Ref<HTMLElement>;
  onClick?: () => void;
  "aria-haspopup"?: "menu";
  "aria-expanded"?: boolean;
}

export function Menu({
  trigger,
  items,
  label,
  align = "end",
}: {
  trigger: ReactElement<TriggerProps>;
  items: MenuItem[];
  label: string;
  align?: "start" | "end";
}) {
  const [open, setOpen] = useState(false);
  const anchor = useRef<HTMLElement | null>(null);
  const list = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (open) list.current?.querySelector<HTMLElement>("[role=menuitem]")?.focus();
  }, [open]);

  const close = () => {
    setOpen(false);
    anchor.current?.focus();
  };

  const onKeyDown = (event: KeyboardEvent) => {
    const entries = Array.from(list.current?.querySelectorAll<HTMLElement>("[role=menuitem]") ?? []);
    const index = entries.indexOf(document.activeElement as HTMLElement);
    const move = (next: number) => entries[(next + entries.length) % entries.length]?.focus();
    if (event.key === "ArrowDown") move(index + 1);
    else if (event.key === "ArrowUp") move(index - 1);
    else if (event.key === "Home") move(0);
    else if (event.key === "End") move(entries.length - 1);
    else if (event.key === "Tab") setOpen(false);
    else return;
    event.preventDefault();
  };

  return (
    <>
      {cloneElement(trigger, {
        ref: (node: HTMLElement | null) => {
          anchor.current = node;
          const original = trigger.props.ref;
          if (typeof original === "function") original(node);
          else if (original) (original as { current: HTMLElement | null }).current = node;
        },
        onClick: () => setOpen((value) => !value),
        "aria-haspopup": "menu",
        "aria-expanded": open,
      })}
      {open && (
        <Popover anchor={anchor} onClose={() => setOpen(false)} align={align} label={label} role="presentation">
          <div ref={list} role="menu" aria-label={label} onKeyDown={onKeyDown} className="min-w-48 p-1">
            {items.map((item, index) =>
              item.separator ? (
                <div key={index} role="separator" className="mx-1 my-1 h-px bg-line" />
              ) : (
                <button
                  key={item.label}
                  type="button"
                  role="menuitem"
                  onClick={() => {
                    close();
                    item.onSelect();
                  }}
                  className={`interactive flex h-8 w-full items-center gap-2.5 rounded-sm px-2.5 text-left text-body focus-visible:bg-[var(--hover)] focus-visible:outline-none ${item.danger ? "text-danger" : "text-ink"}`}
                >
                  {item.icon && <item.icon size={15} aria-hidden className={item.danger ? "" : "text-ink-2"} />}
                  <span className="flex-1">{item.label}</span>
                  {item.hint && <span className="text-meta text-ink-2">{item.hint}</span>}
                </button>
              ),
            )}
          </div>
        </Popover>
      )}
    </>
  );
}
