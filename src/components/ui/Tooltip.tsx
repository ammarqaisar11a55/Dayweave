import { cloneElement, type ReactElement, type Ref, useId, useRef, useState } from "react";
import { createPortal } from "react-dom";

type Placement = "bottom" | "right";

interface TriggerProps {
  ref?: Ref<HTMLElement>;
  onMouseEnter?: () => void;
  onMouseLeave?: () => void;
  onFocus?: () => void;
  onBlur?: () => void;
  "aria-describedby"?: string;
}

/** Shows a label on hover and keyboard focus. Pass `disabled` to keep the wrapper without the label. */
export function Tooltip({
  text,
  children,
  placement = "bottom",
  disabled = false,
}: {
  text: string;
  children: ReactElement<TriggerProps>;
  placement?: Placement;
  disabled?: boolean;
}) {
  const [position, setPosition] = useState<{ x: number; y: number } | null>(null);
  const element = useRef<HTMLElement | null>(null);
  const timer = useRef(0);
  const id = useId();

  const show = () => {
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => {
      const rect = element.current?.getBoundingClientRect();
      if (!rect) return;
      setPosition(
        placement === "right"
          ? { x: rect.right + 8, y: rect.top + rect.height / 2 }
          : { x: rect.left + rect.width / 2, y: rect.bottom + 6 },
      );
    }, 350);
  };
  const hide = () => {
    window.clearTimeout(timer.current);
    setPosition(null);
  };

  const original = children.props.ref;
  const trigger = cloneElement(children, {
    ref: (node: HTMLElement | null) => {
      element.current = node;
      if (typeof original === "function") original(node);
      else if (original) (original as { current: HTMLElement | null }).current = node;
    },
    onMouseEnter: show,
    onMouseLeave: hide,
    onFocus: show,
    onBlur: hide,
    "aria-describedby": position ? id : undefined,
  });

  const transform = placement === "right" ? "translateY(-50%)" : "translateX(-50%)";
  return (
    <>
      {trigger}
      {position &&
        !disabled &&
        createPortal(
          <div
            id={id}
            role="tooltip"
            style={{ left: position.x, top: position.y, transform }}
            className="float animate-fade pointer-events-none fixed z-[70] max-w-64 rounded-sm px-2 py-1 text-meta text-ink"
          >
            {text}
          </div>,
          document.body,
        )}
    </>
  );
}
