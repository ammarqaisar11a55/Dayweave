import { Check, ChevronDown, type LucideIcon } from "lucide-react";
import { type KeyboardEvent, type ReactNode, useEffect, useId, useRef, useState } from "react";

import { Popover } from "./Popover";

export interface Option<T extends string> {
  value: T;
  label: string;
  icon?: LucideIcon;
  decoration?: ReactNode;
}

interface SelectProps<T extends string> {
  value: T;
  options: Option<T>[];
  onChange: (value: T) => void;
  label: string;
  className?: string;
  size?: "md" | "sm";
}

export function Select<T extends string>({
  value,
  options,
  onChange,
  label,
  className = "",
  size = "md",
}: SelectProps<T>) {
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(0);
  const anchor = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLDivElement>(null);
  const id = useId();
  const current = options.find((option) => option.value === value);

  useEffect(() => {
    if (!open) return;
    setActive(Math.max(0, options.findIndex((option) => option.value === value)));
    list.current?.focus();
  }, [open, options, value]);

  const choose = (option: Option<T> | undefined) => {
    if (option) onChange(option.value);
    setOpen(false);
    anchor.current?.focus();
  };

  const onKeyDown = (event: KeyboardEvent) => {
    const last = options.length - 1;
    if (event.key === "ArrowDown") setActive((index) => Math.min(last, index + 1));
    else if (event.key === "ArrowUp") setActive((index) => Math.max(0, index - 1));
    else if (event.key === "Home") setActive(0);
    else if (event.key === "End") setActive(last);
    else if (event.key === "Enter" || event.key === " ") choose(options[active]);
    else if (event.key === "Tab") setOpen(false);
    else if (event.key.length === 1) {
      const typed = event.key.toLowerCase();
      const match = options.findIndex((option) => option.label.toLowerCase().startsWith(typed));
      if (match >= 0) setActive(match);
      return;
    } else return;
    event.preventDefault();
  };

  const height = size === "md" ? "h-9 px-3" : "h-8 px-2.5 text-label";
  return (
    <>
      <button
        ref={anchor}
        type="button"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={label}
        onClick={() => setOpen((state) => !state)}
        onKeyDown={(event) => {
          if (event.key === "ArrowDown" || event.key === "ArrowUp") {
            event.preventDefault();
            setOpen(true);
          }
        }}
        className={`inline-flex min-w-0 items-center justify-between gap-2 control rounded-md text-left transition-colors duration-150 ${height} ${className}`}
      >
        <span className="flex min-w-0 items-center gap-2">
          {current?.decoration}
          {current?.icon && <current.icon size={15} aria-hidden className="text-ink-2" />}
          <span className="truncate">{current?.label ?? "Choose"}</span>
        </span>
        <ChevronDown size={15} aria-hidden className="shrink-0 text-ink-2" />
      </button>
      {open && (
        <Popover anchor={anchor} onClose={() => setOpen(false)} matchWidth role="presentation" className="max-h-72">
          <div
            ref={list}
            role="listbox"
            aria-label={label}
            tabIndex={-1}
            aria-activedescendant={`${id}-${active}`}
            onKeyDown={onKeyDown}
            className="min-w-40 p-1 focus:outline-none"
          >
            {options.map((option, index) => (
              <div
                key={option.value}
                id={`${id}-${index}`}
                role="option"
                aria-selected={option.value === value}
                onMouseEnter={() => setActive(index)}
                onClick={() => choose(option)}
                style={index === active ? { background: "var(--hover)" } : undefined}
                className="flex h-8 items-center gap-2 rounded-sm px-2.5 text-body text-ink"
              >
                {option.decoration}
                {option.icon && <option.icon size={15} aria-hidden className="text-ink-2" />}
                <span className="flex-1 truncate">{option.label}</span>
                {option.value === value && <Check size={15} aria-hidden className="text-accent-text" />}
              </div>
            ))}
          </div>
        </Popover>
      )}
    </>
  );
}
