import type { LucideIcon } from "lucide-react";
import { type ButtonHTMLAttributes, forwardRef } from "react";

import { Tooltip } from "./Tooltip";

type Variant = "primary" | "secondary" | "ghost" | "danger";

const VARIANTS: Record<Variant, string> = {
  primary: "bg-accent text-accent-ink font-semibold hover:bg-accent-hover active:brightness-95",
  secondary: "control font-medium active:brightness-95",
  ghost: "text-ink-2 font-medium interactive hover:text-ink",
  danger: "bg-danger/12 text-danger font-semibold hover:bg-danger/18 active:bg-danger/25",
};

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: "md" | "sm";
  icon?: LucideIcon;
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { variant = "secondary", size = "md", icon: Icon, className = "", children, type = "button", ...rest },
  reference,
) {
  const sizing = size === "md" ? "h-9 px-3.5 text-body gap-2" : "h-8 px-2.5 text-label gap-1.5";
  return (
    <button
      ref={reference}
      type={type}
      className={`inline-flex shrink-0 items-center justify-center whitespace-nowrap rounded-md transition-[background-color,color,filter] duration-150 disabled:pointer-events-none disabled:opacity-45 ${sizing} ${VARIANTS[variant]} ${className}`}
      {...rest}
    >
      {Icon && <Icon size={size === "md" ? 16 : 15} aria-hidden />}
      {children}
    </button>
  );
});

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon: LucideIcon;
  label: string;
  size?: "md" | "sm";
  hint?: string;
}

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(function IconButton(
  { icon: Icon, label, size = "md", hint, className = "", type = "button", ...rest },
  reference,
) {
  const sizing = size === "md" ? "size-9" : "size-8";
  return (
    <Tooltip text={hint ? `${label} (${hint})` : label}>
      <button
        ref={reference}
        type={type}
        aria-label={label}
        className={`interactive inline-flex shrink-0 items-center justify-center rounded-md text-ink-2 transition-colors duration-150 hover:text-ink disabled:pointer-events-none disabled:opacity-45 ${sizing} ${className}`}
        {...rest}
      >
        <Icon size={size === "md" ? 18 : 16} aria-hidden />
      </button>
    </Tooltip>
  );
});
