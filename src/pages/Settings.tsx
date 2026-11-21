import { CloudOff, Database, Download, FileJson, FileSpreadsheet, FolderOpen, Keyboard, LogOut, RefreshCw, Upload } from "lucide-react";
import { type ReactNode, useState } from "react";

import { open, save } from "@tauri-apps/plugin-dialog";

import { PageHeader, Section } from "../components/shell/PageHeader";
import { Button } from "../components/ui/Button";
import { Dialog } from "../components/ui/Dialog";
import { Input, Segmented, Switch } from "../components/ui/Form";
import { Select } from "../components/ui/Select";
import { TimePicker } from "../components/ui/TimePicker";
import { PRIORITY_OPTIONS } from "../components/task/TaskDialog";
import { inTauri, rpc } from "../lib/api";
import { formatInstant, today } from "../lib/date";
import { setSetting, useSettings } from "../lib/settings";
import { act, mutate, showToast, useRpc } from "../lib/store";
import type { AppInfo, BackupCounts, BackupPreview, Settings as SettingsData, SyncStatus } from "../lib/types";
import { showShortcuts } from "../lib/ui";

function Row({ title, detail, children, htmlFor }: { title: string; detail?: string; children: ReactNode; htmlFor?: string }) {
  return (
    <div className="flex items-center justify-between gap-6 px-4 py-3">
      <div className="min-w-0">
        <label htmlFor={htmlFor} className="block text-body">
          {title}
        </label>
        {detail && <p className="mt-0.5 text-meta text-ink-2">{detail}</p>}
      </div>
      {children}
    </div>
  );
}

function Toggle({ name, title, detail, disabled }: { name: keyof SettingsData; title: string; detail?: string; disabled?: boolean }) {
  const settings = useSettings();
  return (
    <div className={disabled ? "pointer-events-none opacity-50" : ""}>
      <Row title={title} detail={detail}>
        <Switch checked={Boolean(settings[name])} onChange={(value) => void setSetting(name, value as never)} label={title} />
      </Row>
    </div>
  );
}

function MinutesField({ name, title, max }: { name: "focusMinutes" | "shortBreakMinutes" | "longBreakMinutes" | "sessionsBeforeLongBreak"; title: string; max: number }) {
  const settings = useSettings();
  const [value, setValue] = useState(String(settings[name]));
  return (
    <Row title={title}>
      <Input
        type="number"
        min={1}
        max={max}
        aria-label={title}
        value={value}
        onChange={(event) => setValue(event.target.value)}
        onBlur={() => {
          const number = Number(value);
          if (number >= 1 && number <= max) void setSetting(name, number);
          else setValue(String(settings[name]));
        }}
        className="w-20"
      />
    </Row>
  );
}

async function choosePath(mode: "save" | "open", name: string, extensions: string[]): Promise<string | null> {
  if (!inTauri) return window.prompt("Path on this computer", `/tmp/${name}`);
  const filters = [{ name: "Dayweave backup", extensions }];
  if (mode === "save") return save({ defaultPath: name, filters });
  const chosen = await open({ multiple: false, filters });
  return typeof chosen === "string" ? chosen : null;
}

async function exportTo(method: string, name: string, extension: string, label: string) {
  const path = await choosePath("save", name, [extension]);
  if (!path) return;
  await act(method, { path }, `${label} saved`);
}

const COUNT_LABELS: [keyof BackupCounts, string][] = [
  ["tasks", "Tasks"],
  ["habits", "Habits"],
  ["habitEntries", "Habit entries"],
  ["projects", "Projects"],
  ["focusSessions", "Focus sessions"],
];

function RestoreDialog({ path, preview, onClose }: { path: string; preview: BackupPreview; onClose: () => void }) {
  const [busy, setBusy] = useState(false);
  const restore = async () => {
    setBusy(true);
    try {
      await mutate("backup.restore", { path });
      showToast("Backup restored");
      onClose();
    } catch {
      setBusy(false);
    }
  };
  return (
    <Dialog
      title="Restore this backup?"
      onClose={onClose}
      width="max-w-md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="danger" disabled={busy} onClick={() => void restore()}>
            Replace my data
          </Button>
        </>
      }
    >
      <p className="text-body text-ink-2">
        Everything in Dayweave will be replaced with this backup from {formatInstant(preview.exportedAt)}. A safety copy of your current data is saved first.
      </p>
      <table className="mt-3 w-full text-body">
        <thead>
          <tr className="text-left text-meta text-ink-2">
            <th className="py-1 font-medium">Contains</th>
            <th className="py-1 text-right font-medium">Now</th>
            <th className="py-1 text-right font-medium">In backup</th>
          </tr>
        </thead>
        <tbody className="tabular">
          {COUNT_LABELS.map(([key, label]) => (
            <tr key={key} className="border-t border-line">
              <td className="py-1">{label}</td>
              <td className="py-1 text-right text-ink-2">{preview.current[key]}</td>
              <td className="py-1 text-right font-medium">{preview.incoming[key]}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </Dialog>
  );
}

function DataSection() {
  const [pending, setPending] = useState<{ path: string; preview: BackupPreview } | null>(null);
  const stamp = today();

  const chooseBackup = async () => {
    const path = await choosePath("open", "backup.json", ["json", "db"]);
    if (!path) return;
    try {
      setPending({ path, preview: await rpc<BackupPreview>("backup.preview", { path }) });
    } catch (error) {
      showToast((error as Error).message, { tone: "error" });
    }
  };

  return (
    <Section title="Data and backups">
      <div className="surface row-list">
        <Row title="Full backup" detail="A complete copy of your database. The safest way to back up">
          <Button icon={Database} onClick={() => void exportTo("backup.exportDatabase", `dayweave-${stamp}.db`, "db", "Backup")}>
            Save backup
          </Button>
        </Row>
        <Row title="Export everything as JSON" detail="All tasks, habits, projects and history, in a readable format">
          <Button icon={FileJson} onClick={() => void exportTo("backup.exportJson", `dayweave-${stamp}.json`, "json", "Export")}>
            Export JSON
          </Button>
        </Row>
        <Row title="Export tasks as CSV" detail="For spreadsheets">
          <Button icon={FileSpreadsheet} onClick={() => void exportTo("backup.exportTasksCsv", `dayweave-tasks-${stamp}.csv`, "csv", "Tasks")}>
            Export tasks
          </Button>
        </Row>
        <Row title="Export habit history as CSV" detail="Every logged day, for spreadsheets">
          <Button icon={Download} onClick={() => void exportTo("backup.exportHabitsCsv", `dayweave-habits-${stamp}.csv`, "csv", "Habit history")}>
            Export history
          </Button>
        </Row>
        <Row title="Restore from a backup" detail="Choose a .json or .db backup. You'll see what's inside before anything changes">
          <Button icon={Upload} onClick={() => void chooseBackup()}>
            Choose backup
          </Button>
        </Row>
      </div>
      {pending && <RestoreDialog {...pending} onClose={() => setPending(null)} />}
    </Section>
  );
}

function syncSummary(status: SyncStatus): string {
  if (status.pending > 0) return `${status.pending} ${status.pending === 1 ? "change" : "changes"} waiting to sync`;
  if (status.lastSyncAt) return `Up to date. Last synced ${formatInstant(status.lastSyncAt)}`;
  return "Not synced yet";
}

function SignInForm({ status }: { status: SyncStatus }) {
  const [server, setServer] = useState(status.server ?? "");
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const ready = server.trim() !== "" && email.trim() !== "" && password !== "";
  const submit = async (create: boolean) => {
    setBusy(true);
    try {
      await mutate(create ? "sync.signUp" : "sync.signIn", { server, email, password, device: "Dayweave for Ubuntu" });
      showToast(create ? "Account created. Syncing is on" : "Signed in. Syncing is on");
    } catch {
      setBusy(false);
    }
  };
  return (
    <form
      className="flex flex-col gap-3 p-4"
      onSubmit={(event) => {
        event.preventDefault();
        if (ready) void submit(false);
      }}
    >
      <p className="text-body text-ink-2">
        Dayweave works fully on this computer. Sign in to a Dayweave server to keep your tasks and habits in step with your phone and other
        computers. Changes made offline sync when you&rsquo;re back online.
      </p>
      <label className="flex flex-col gap-1 text-meta text-ink-2">
        Server address
        <Input
          aria-label="Server address"
          value={server}
          onChange={(event) => setServer(event.target.value)}
          placeholder="https://your-server.vercel.app"
          autoComplete="url"
        />
      </label>
      <div className="grid grid-cols-2 gap-3">
        <label className="flex flex-col gap-1 text-meta text-ink-2">
          Email
          <Input aria-label="Email" type="email" value={email} onChange={(event) => setEmail(event.target.value)} autoComplete="email" />
        </label>
        <label className="flex flex-col gap-1 text-meta text-ink-2">
          Password
          <Input
            aria-label="Password"
            type="password"
            value={password}
            onChange={(event) => setPassword(event.target.value)}
            autoComplete="current-password"
          />
        </label>
      </div>
      <div className="flex justify-end gap-2">
        <Button disabled={!ready || busy} onClick={() => void submit(true)}>
          Create account
        </Button>
        <Button type="submit" variant="primary" disabled={!ready || busy}>
          Sign in
        </Button>
      </div>
    </form>
  );
}

function DeleteAccountDialog({ onClose }: { onClose: () => void }) {
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const remove = async () => {
    setBusy(true);
    try {
      await mutate("sync.deleteAccount", { password });
      showToast("Account deleted from the server. Your data here is unchanged");
      onClose();
    } catch {
      setBusy(false);
    }
  };
  return (
    <Dialog
      title="Delete your sync account?"
      onClose={onClose}
      width="max-w-md"
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="danger" disabled={busy || password === ""} onClick={() => void remove()}>
            Delete account
          </Button>
        </>
      }
    >
      <p className="text-body text-ink-2">
        Everything stored on the server for this account is deleted, and every device stops syncing. The data on this computer stays as
        it is.
      </p>
      <label className="mt-3 flex flex-col gap-1 text-meta text-ink-2">
        Password
        <Input
          aria-label="Password"
          type="password"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
          autoComplete="current-password"
        />
      </label>
    </Dialog>
  );
}

function SyncSection() {
  const { data: status } = useRpc<SyncStatus>("sync.status");
  const [syncing, setSyncing] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const syncNow = async () => {
    setSyncing(true);
    try {
      await mutate("sync.run");
    } catch {
      // The toast says why. Changes stay here until the next try.
    }
    setSyncing(false);
  };
  if (!status) return null;
  return (
    <Section title="Sync">
      <div className="surface row-list">
        {!status.signedIn ? (
          <SignInForm status={status} />
        ) : (
          <>
            <Row title={`Syncing as ${status.email ?? "you"}`} detail={`${status.server ?? ""}. ${syncSummary(status)}`}>
              <Button icon={RefreshCw} disabled={syncing} onClick={() => void syncNow()}>
                {syncing ? "Syncing…" : "Sync now"}
              </Button>
            </Row>
            {status.lastError && (
              <div className="flex items-start gap-2 px-4 py-3 text-meta text-warning">
                <CloudOff size={14} aria-hidden className="mt-px shrink-0" />
                <span>{status.lastError}</span>
              </div>
            )}
            <Row title="Sign out" detail="Stops syncing on this computer. Your data stays here and on the server">
              <Button icon={LogOut} onClick={() => void act("sync.signOut", {}, "Signed out. Syncing is off")}>
                Sign out
              </Button>
            </Row>
            <Row title="Delete account" detail="Removes everything stored on the server. The data on your devices stays">
              <Button variant="danger" onClick={() => setDeleting(true)}>
                Delete account
              </Button>
            </Row>
          </>
        )}
      </div>
      {deleting && <DeleteAccountDialog onClose={() => setDeleting(false)} />}
    </Section>
  );
}

export function Settings() {
  const settings = useSettings();
  const { data: info } = useRpc<AppInfo>("app.info");
  const notifications = settings.notificationsEnabled;
  return (
    <div className="animate-page mx-auto max-w-[720px]">
      <PageHeader
        title="Settings"
        actions={
          <Button icon={Keyboard} onClick={() => showShortcuts(true)}>
            Keyboard shortcuts
          </Button>
        }
      />
      <div className="flex flex-col gap-8">
        <Section title="Appearance">
          <div className="surface row-list">
            <Row title="Theme">
              <Segmented
                label="Theme"
                value={settings.theme}
                onChange={(value) => void setSetting("theme", value)}
                options={[
                  { value: "system", label: "System" },
                  { value: "light", label: "Light" },
                  { value: "dark", label: "Dark" },
                ]}
              />
            </Row>
            <Row title="Task list density">
              <Segmented
                label="Task list density"
                value={settings.taskDensity}
                onChange={(value) => void setSetting("taskDensity", value)}
                options={[
                  { value: "comfortable", label: "Comfortable" },
                  { value: "compact", label: "Compact" },
                ]}
              />
            </Row>
            <Toggle name="reduceMotion" title="Reduce motion" detail="Also applies automatically when your desktop asks for it" />
          </div>
        </Section>

        <Section title="Notifications">
          <div className="surface row-list">
            <Toggle name="notificationsEnabled" title="Desktop notifications" detail="Everything still appears in the notification center when this is off" />
            <Toggle name="taskNotifications" title="Task reminders and deadlines" disabled={!notifications} />
            <Toggle name="habitNotifications" title="Habit reminders" disabled={!notifications} />
            <Toggle name="focusNotifications" title="Focus timer" disabled={!notifications} />
            <Row title="Default reminder" detail="Added to new tasks with a deadline">
              <Select
                label="Default reminder"
                value={String(settings.defaultReminderMinutes ?? "none")}
                className="w-52"
                onChange={(value) => void setSetting("defaultReminderMinutes", value === "none" ? null : Number(value))}
                options={[
                  { value: "none", label: "None" },
                  { value: "0", label: "At deadline" },
                  { value: "10", label: "10 minutes before" },
                  { value: "30", label: "30 minutes before" },
                  { value: "60", label: "1 hour before" },
                  { value: "1440", label: "1 day before" },
                ]}
              />
            </Row>
            <Row title="Time for date-only deadlines" detail="Deadlines without a time have their reminders counted from this time of day">
              <TimePicker
                value={settings.dateOnlyReminderTime}
                onChange={(value) => value && void setSetting("dateOnlyReminderTime", value)}
                label="Reminder time"
                clearable={false}
                className="w-32"
              />
            </Row>
          </div>
        </Section>

        <Section title="General">
          <div className="surface row-list">
            <Toggle
              name="closeToBackground"
              title="Keep running when the window closes"
              detail="Dayweave stays in the tray and keeps delivering reminders. Reminders stop when you choose Quit"
            />
            <Toggle
              name="startAtLogin"
              title="Start at login"
              detail="Starts in the background when you sign in, so reminders work without opening Dayweave"
            />
            <Row title="First day of the week">
              <Select
                label="First day of the week"
                value={settings.weekStart}
                className="w-36"
                onChange={(value) => void setSetting("weekStart", value)}
                options={[
                  { value: "monday", label: "Monday" },
                  { value: "sunday", label: "Sunday" },
                  { value: "saturday", label: "Saturday" },
                ]}
              />
            </Row>
            <Row title="Time format">
              <Segmented
                label="Time format"
                value={settings.timeFormat}
                onChange={(value) => void setSetting("timeFormat", value)}
                options={[
                  { value: "24h", label: "24-hour" },
                  { value: "12h", label: "12-hour" },
                ]}
              />
            </Row>
            <Row title="Default priority">
              <Select
                label="Default priority"
                value={settings.defaultPriority}
                options={PRIORITY_OPTIONS}
                className="w-36"
                onChange={(value) => void setSetting("defaultPriority", value)}
              />
            </Row>
            <Toggle name="confirmDelete" title="Ask before deleting" />
          </div>
        </Section>

        <Section title="Focus">
          <div className="surface row-list">
            <MinutesField name="focusMinutes" title="Focus length in minutes" max={240} />
            <MinutesField name="shortBreakMinutes" title="Short break in minutes" max={60} />
            <MinutesField name="longBreakMinutes" title="Long break in minutes" max={120} />
            <MinutesField name="sessionsBeforeLongBreak" title="Sessions before a long break" max={12} />
          </div>
        </Section>

        <SyncSection />

        <DataSection />

        <Section title="Privacy">
          <div className="surface p-4 text-body text-ink-2">
            <p>
              Dayweave works entirely on this computer unless you turn on sync. Then it talks only to the server you chose, and only to sync
              your data. It contains no tracking or advertising, and your data stays in one file that you control.
            </p>
            {info?.dataDir && (
              <p className="mt-3 flex items-start gap-2 text-meta">
                <FolderOpen size={14} aria-hidden className="mt-px shrink-0" />
                <span data-selectable className="break-all">
                  {info.dataDir}
                </span>
              </p>
            )}
          </div>
        </Section>

        <Section title="About Dayweave">
          <div className="surface p-4 text-body text-ink-2">
            Version {info?.version}. Database schema {info?.schemaVersion}.{" "}
            <a href="#/about" className="font-medium text-accent-text">
              More about Dayweave
            </a>
          </div>
        </Section>
      </div>
    </div>
  );
}
