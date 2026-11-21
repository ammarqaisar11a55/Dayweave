import { formatInstant } from "./date";
import type { SyncStatus } from "./types";

/** One line on where sync stands, for Settings and the sidebar. */
export function syncSummary(status: SyncStatus): string {
  if (status.pending > 0) return `${status.pending} ${status.pending === 1 ? "change" : "changes"} waiting to sync`;
  if (status.lastSyncAt) return `Up to date. Last synced ${formatInstant(status.lastSyncAt)}`;
  return "Not synced yet";
}
