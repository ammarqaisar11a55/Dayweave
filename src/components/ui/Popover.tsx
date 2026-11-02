import { type ReactNode, type RefObject, useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";

const MARGIN = 8;

interface PopoverProps {
  anchor: RefObject<HTMLElement | null>;
  onClose: () => void;
  children: ReactNode;
  align?: "start" | "end";
  matchWidth?: boolean;
  className?: string;
  label?: string;
  role?: string;
}

/** A floating layer tied to an anchor. It flips above the anchor when there is no room below. */
export function Popover({
  anchor,
  onClose,
  children,
  align = "start",
  matchWidth = false,
  className = "",
  label,
  role = "dialog",
}: PopoverProps) {
  const layer = useRef<HTMLDivElement>(null);
  const [style, setStyle] = useState<React.CSSProperties>({ visibility: "hidden", top: 0, left: 0 });

  useLayoutEffect(() => {
    const place = () => {
      const target = anchor.current?.getBoundingClientRect();
      const own = layer.current?.getBoundingClientRect();
      if (!target || !own) return;
      const below = window.innerHeight - target.bottom - MARGIN;
      const above = target.top - MARGIN;
      const top = own.height > below && above > below ? target.top - own.height - 4 : target.bottom + 4;
      const wanted = align === "end" ? target.right - own.width : target.left;
      const left = Math.max(MARGIN, Math.min(wanted, window.innerWidth - own.width - MARGIN));
      setStyle({
        top: Math.max(MARGIN, top),
        left,
        minWidth: matchWidth ? target.width : undefined,
        maxHeight: window.innerHeight - 2 * MARGIN,
      });
    };
    place();
    window.addEventListener("resize", place);
    return () => window.removeEventListener("resize", place);
  }, [anchor, align, matchWidth]);

  useEffect(() => {
    const onPointer = (event: MouseEvent) => {
      const target = event.target as Node;
      if (!layer.current?.contains(target) && !anchor.current?.contains(target)) onClose();
    };
    document.addEventListener("mousedown", onPointer);
    return () => document.removeEventListener("mousedown", onPointer);
  }, [anchor, onClose]);

  return createPortal(
    <div
      ref={layer}
      role={role}
      aria-label={label}
      style={style}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.stopPropagation();
          onClose();
          anchor.current?.focus();
        }
      }}
      className={`float animate-rise fixed z-[60] overflow-y-auto rounded-md ${className}`}
    >
      {children}
    </div>,
    document.body,
  );
}
