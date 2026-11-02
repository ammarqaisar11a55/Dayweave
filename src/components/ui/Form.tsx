import { Check, Minus } from "lucide-react";
import {
  forwardRef,
  type InputHTMLAttributes,
  type KeyboardEvent,
  type ReactNode,
  type TextareaHTMLAttributes,
  useId,
  useLayoutEffect,
  useRef,
} from "react";

const FIELD =
  "w-full rounded-md bg-sunken px-3 text-body text-ink placeholder:text-ink-3 outline-none transition-shadow duration-150 focus:ring-2 focus:ring-accent/70";

export const Input = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement>>(function Input(
  { className = "", ...rest },
  reference,
) {
  return <input ref={reference} className={`h-9 ${FIELD} ${className}`} {...rest} />;
});

/** A text area that grows with its content. */
export function Textarea({
  className = "",
  minRows = 3,
  value,
  ...rest
}: TextareaHTMLAttributes<HTMLTextAreaElement> & { minRows?: number }) {
  const element = useRef<HTMLTextAreaElement>(null);
  useLayoutEffect(() => {
    const node = element.current;
    if (!node) return;
    node.style.height = "auto";
    node.style.height = `${node.scrollHeight}px`;
  }, [value]);
  return (
    <textarea
      ref={element}
      rows={minRows}
      value={value}
      className={`resize-none overflow-hidden py-2 ${FIELD} ${className}`}
      {...rest}
    />
  );
}

export function Field({
  label,
  hint,
  children,
  className = "",
}: {
  label: string;
  hint?: string;
  children: (id: string) => ReactNode;
  className?: string;
}) {
  const id = useId();
  return (
    <div className={`flex min-w-0 flex-col gap-1.5 ${className}`}>
      <label htmlFor={id} className="text-label font-medium text-ink-2">
        {label}
      </label>
      {children(id)}
      {hint && <p className="text-meta text-ink-2">{hint}</p>}
    </div>
  );
}

export function Switch({
  checked,
  onChange,
  label,
  id,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  id?: string;
}) {
  return (
    <button
      id={id}
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      onClick={() => onChange(!checked)}
      className={`relative h-6 w-10 shrink-0 rounded-full transition-colors duration-150 ${checked ? "bg-accent" : "bg-[var(--thread-idle)]"}`}
    >
      <span
        aria-hidden
        className={`absolute left-0.5 top-0.5 size-5 rounded-full bg-white shadow-sm transition-transform duration-150 ease-weave ${checked ? "translate-x-4" : ""}`}
      />
    </button>
  );
}

export interface SegmentOption<T extends string> {
  value: T;
  label: string;
  count?: number;
}

export function Segmented<T extends string>({
  value,
  options,
  onChange,
  label,
  className = "",
}: {
  value: T;
  options: SegmentOption<T>[];
  onChange: (value: T) => void;
  label: string;
  className?: string;
}) {
  const onKeyDown = (event: KeyboardEvent) => {
    const step = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
    if (!step) return;
    event.preventDefault();
    const index = options.findIndex((option) => option.value === value);
    const next = options[(index + step + options.length) % options.length];
    if (next) onChange(next.value);
  };
  return (
    <div
      role="radiogroup"
      aria-label={label}
      onKeyDown={onKeyDown}
      className={`inline-flex rounded-md bg-sunken p-0.5 ${className}`}
    >
      {options.map((option) => {
        const selected = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={selected}
            tabIndex={selected ? 0 : -1}
            onClick={() => onChange(option.value)}
            className={`inline-flex h-8 items-center gap-1.5 whitespace-nowrap rounded-[10px] px-3 text-label font-medium transition-colors duration-150 ${selected ? "bg-card text-ink shadow-[0_0_0_1px_var(--line)]" : "text-ink-2 hover:text-ink"}`}
          >
            {option.label}
            {option.count !== undefined && option.count > 0 && (
              <span className="tabular text-meta font-normal text-ink-2">{option.count}</span>
            )}
          </button>
        );
      })}
    </div>
  );
}

/** A round completion control, used for anything that can be finished. */
export function DoneCheck({
  checked,
  onChange,
  label,
  color,
  size = 22,
}: {
  checked: boolean;
  onChange: () => void;
  label: string;
  color?: string;
  size?: number;
}) {
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={checked}
      aria-label={label}
      onClick={(event) => {
        event.stopPropagation();
        onChange();
      }}
      style={{ width: size, height: size, color: color ?? "var(--ink-3)" }}
      className={`group inline-flex shrink-0 items-center justify-center rounded-full transition-colors duration-150 ${
        checked ? "animate-check bg-accent" : "border-[1.5px] border-current hover:bg-[var(--hover)]"
      }`}
    >
      <Check
        size={size - 8}
        strokeWidth={3}
        aria-hidden
        className={checked ? "text-accent-ink" : "text-ink-3 opacity-0 group-hover:opacity-100"}
      />
    </button>
  );
}

export function Checkbox({
  checked,
  onChange,
  label,
  indeterminate = false,
}: {
  checked: boolean;
  onChange: () => void;
  label: string;
  indeterminate?: boolean;
}) {
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={indeterminate ? "mixed" : checked}
      aria-label={label}
      onClick={onChange}
      className={`inline-flex size-[18px] shrink-0 items-center justify-center rounded-[6px] transition-colors duration-150 ${
        checked || indeterminate ? "bg-accent text-accent-ink" : "border-[1.5px] border-ink-3 hover:bg-[var(--hover)]"
      }`}
    >
      {indeterminate ? (
        <Minus size={12} strokeWidth={3} aria-hidden />
      ) : (
        checked && <Check size={12} strokeWidth={3} aria-hidden />
      )}
    </button>
  );
}
