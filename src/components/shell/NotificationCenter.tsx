import { Bell, BellOff, CheckCheck, Repeat2, Timer, Trash2 } from "lucide-react";
import { useRef, useState } from "react";

import { formatInstant } from "../../lib/date";
import { navigate } from "../../lib/router";
import { act, useRpc } from "../../lib/store";
import type { Badges, NoticeEntry } from "../../lib/types";
import { openHabit, openTask } from "../../lib/ui";
import { Button, IconButton } from "../ui/Button";
import { EmptyState } from "../ui/EmptyState";
import { Popover } from "../ui/Popover";

const KIND_ICON = { habit: Repeat2, focus: Timer };

function open(entry: NoticeEntry) {
  if (entry.taskId !== null && entry.kind === "task") openTask(entry.taskId);
  else if (entry.habitId !== null && entry.kind === "habit") openHabit(entry.habitId);
  else if (entry.kind === "focus") navigate("focus");
}

export function NotificationCenter() {
  const [opened, setOpened] = useState(false);
  const anchor = useRef<HTMLButtonElement>(null);
  const { data: badges } = useRpc<Badges>("overview.badges");
  const { data: notices } = useRpc<NoticeEntry[]>("notifications.list", {}, opened);
  const unread = badges?.unread ?? 0;

  const toggle = () => {
    if (!opened && unread) void act("notifications.markRead");
    setOpened((state) => !state);
  };

  return (
    <div className="relative">
      <IconButton
        ref={anchor}
        icon={Bell}
        label={unread ? `Notifications, ${unread} unread` : "Notifications"}
        aria-expanded={opened}
        onClick={toggle}
      />
      {unread > 0 && <span aria-hidden className="pointer-events-none absolute right-2 top-2 size-2 rounded-full bg-accent" />}
      {opened && (
        <Popover anchor={anchor} onClose={() => setOpened(false)} align="end" label="Notifications" className="w-[360px]">
          <div className="flex items-center justify-between px-4 pb-1 pt-3">
            <h2 className="text-heading font-semibold">Notifications</h2>
            {notices && notices.length > 0 && (
              <Button size="sm" variant="ghost" icon={Trash2} onClick={() => void act("notifications.clear")}>
                Clear
              </Button>
            )}
          </div>
          {notices?.length === 0 && (
            <EmptyState compact icon={BellOff} title="Nothing new" body="Reminders and finished focus sessions appear here." />
          )}
          <ul className="max-h-[420px] overflow-y-auto p-1">
            {notices?.map((entry) => {
              const Icon = KIND_ICON[entry.kind as keyof typeof KIND_ICON] ?? CheckCheck;
              return (
                <li key={entry.key}>
                  <button
                    type="button"
                    onClick={() => {
                      setOpened(false);
                      open(entry);
                    }}
                    className="interactive flex w-full gap-3 rounded-sm px-3 py-2 text-left"
                  >
                    <Icon size={16} aria-hidden className="mt-0.5 shrink-0 text-ink-2" />
                    <span className="min-w-0 flex-1">
                      <span className="block truncate text-body">{entry.title}</span>
                      <span className="block truncate text-meta text-ink-2">
                        {entry.body ? `${entry.body}. ` : ""}
                        {formatInstant(entry.createdAt)}
                      </span>
                    </span>
                  </button>
                </li>
              );
            })}
          </ul>
        </Popover>
      )}
    </div>
  );
}
