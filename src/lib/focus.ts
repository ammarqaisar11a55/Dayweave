import { useEffect, useState } from "react";

import { refreshData } from "./store";
import type { FocusSession } from "./types";

/**
 * Seconds left in a session. The time comes from the core's end time rather than from counting
 * ticks, so it stays right after the webview has been throttled in the background.
 */
export function useCountdown(session: FocusSession | null | undefined): number {
  const [, tick] = useState(0);
  const running = session?.status === "running" && session.endsAt;

  useEffect(() => {
    if (!running) return;
    const timer = window.setInterval(() => tick((value) => value + 1), 500);
    return () => window.clearInterval(timer);
  }, [running]);

  const remaining = !session
    ? 0
    : session.status === "running" && session.endsAt
      ? Math.max(0, Math.ceil((new Date(session.endsAt).getTime() - Date.now()) / 1000))
      : session.remainingSeconds;

  useEffect(() => {
    if (!running || remaining > 0) return;
    // The core ends the session. This only asks for the result a moment after it has.
    const timer = window.setTimeout(refreshData, 1200);
    return () => window.clearTimeout(timer);
  }, [running, remaining]);

  return remaining;
}
