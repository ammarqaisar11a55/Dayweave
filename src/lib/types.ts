// Mirrors the JSON returned by crates/dayweave-core.

export type TaskStatus = "todo" | "in_progress" | "completed" | "archived";
export type Priority = "low" | "medium" | "high" | "urgent";
export type TaskView = "all" | "today" | "upcoming" | "overdue" | "completed" | "archived";
export type TaskSort = "deadline" | "priority" | "created" | "status" | "title";

export interface Tag {
  id: number;
  name: string;
}

export interface Subtask {
  id: number;
  parentId: number | null;
  title: string;
  completed: boolean;
}

export interface Reminder {
  id?: number;
  kind: "before" | "at";
  offsetMinutes: number | null;
  remindAt: string | null;
}

export interface Recurrence {
  frequency: "daily" | "weekly" | "monthly" | "yearly";
  interval: number;
  weekdays: number[];
  endDate: string | null;
  maxCount: number | null;
}

export interface Task {
  id: number;
  title: string;
  description: string;
  status: TaskStatus;
  priority: Priority;
  category: string | null;
  projectId: number | null;
  projectName: string | null;
  projectColor: string | null;
  habitId: number | null;
  habitName: string | null;
  recurrenceId: number | null;
  startDate: string | null;
  dueDate: string | null;
  dueTime: string | null;
  estimatedMinutes: number | null;
  createdAt: string;
  updatedAt: string;
  completedAt: string | null;
  completedOn: string | null;
  archivedAt: string | null;
  overdue: boolean;
  subtaskTotal: number;
  subtaskDone: number;
  reminderCount: number;
  tags: Tag[];
  subtasks: Subtask[];
  reminders: Reminder[];
  recurrence: Recurrence | null;
}

export interface TaskInput {
  title: string;
  description: string;
  status?: TaskStatus;
  priority: Priority;
  category: string | null;
  projectId: number | null;
  habitId: number | null;
  startDate: string | null;
  dueDate: string | null;
  dueTime: string | null;
  estimatedMinutes: number | null;
  tags: string[];
  subtasks?: string[];
  reminders: Reminder[];
  recurrence: Recurrence | null;
}

export interface TaskCounts {
  all: number;
  today: number;
  upcoming: number;
  overdue: number;
  completed: number;
  archived: number;
}

export interface Project {
  id: number;
  name: string;
  description: string;
  color: string;
  icon: string;
  startDate: string | null;
  deadline: string | null;
  archivedAt: string | null;
  createdAt: string;
  taskTotal: number;
  taskDone: number;
  taskOpen: number;
  taskOverdue: number;
  completion: number;
}

export type HabitKind = "build" | "quit" | "quantity" | "checklist";
export type HabitFrequency = "daily" | "weekdays" | "weekly_target";
export type DayState = "inactive" | "not_scheduled" | "done" | "partial" | "missed" | "pending";

export interface HabitStats {
  currentStreak: number;
  bestStreak: number;
  streakUnit: "days" | "weeks";
  completionRate: number;
  weeklyConsistency: number;
  monthlyConsistency: number;
  totalDone: number;
}

export interface ChecklistItem {
  id: number;
  title: string;
}

export interface Habit {
  id: number;
  name: string;
  description: string;
  icon: string;
  color: string;
  kind: HabitKind;
  frequency: HabitFrequency;
  weekdays: number[];
  weeklyTarget: number | null;
  startDate: string;
  reminderTime: string | null;
  quantityTarget: number | null;
  unit: string;
  position: number;
  archivedAt: string | null;
  checklist: ChecklistItem[];
  today: {
    state: DayState;
    due: boolean;
    value: number | null;
    checkedItems: number[];
    note: string;
    slipped: boolean;
  };
  stats: HabitStats;
  recent: { date: string; state: DayState }[];
}

export interface HabitInput {
  name: string;
  description: string;
  icon: string;
  color: string;
  kind: HabitKind;
  frequency: HabitFrequency;
  weekdays: number[];
  weeklyTarget: number | null;
  startDate: string | null;
  reminderTime: string | null;
  quantityTarget: number | null;
  unit: string;
  checklist: { id?: number; title: string }[];
}

export interface HistoryDay {
  date: string;
  state: DayState;
  value: number | null;
  note: string;
}

export interface HeatmapDay {
  date: string;
  level: number;
  done: number;
  due: number;
  pending: number;
  missed: boolean;
}

export interface FocusSession {
  id: number;
  kind: "focus" | "short_break" | "long_break";
  status: "running" | "paused" | "completed" | "stopped";
  plannedSeconds: number;
  elapsedSeconds: number;
  remainingSeconds: number;
  taskId: number | null;
  taskTitle: string | null;
  habitId: number | null;
  habitName: string | null;
  localDate: string;
  startedAt: string;
  resumedAt: string | null;
  endedAt: string | null;
  endsAt: string | null;
}

export interface FocusStats {
  todaySeconds: number;
  weekSeconds: number;
  totalSeconds: number;
  todaySessions: number;
  totalSessions: number;
  sessionsSinceLongBreak: number;
}

export interface OverviewWeekDay {
  date: string;
  tasksDone: number;
  habitsDone: number;
  habitsDue: number;
  focusSeconds: number;
}

export interface Overview {
  date: string;
  tasks: Task[];
  overdue: Task[];
  upcoming: Task[];
  habits: Habit[];
  tasksDone: number;
  tasksTotal: number;
  habitsDone: number;
  habitsTotal: number;
  progress: number;
  focusSeconds: number;
  focusSessions: number;
  week: OverviewWeekDay[];
}

export interface Badges {
  overdue: number;
  today: number;
  unread: number;
  focus: FocusSession | null;
}

export interface CalendarItem {
  kind: "task" | "deadline" | "habit" | "focus" | "reminder";
  id: number;
  title: string;
  time: string | null;
  priority: Priority | null;
  completed: boolean;
  overdue: boolean;
  seconds: number | null;
  color: string | null;
  dueDate: string | null;
}

export interface CalendarDay {
  date: string;
  items: CalendarItem[];
  habitsDue: number;
  habitsDone: number;
  focusSeconds: number;
}

export interface HabitSummary {
  id: number;
  name: string;
  color: string;
  icon: string;
  currentStreak: number;
  bestStreak: number;
  streakUnit: "days" | "weeks";
  completionRate: number;
  weeklyConsistency: number;
  monthlyConsistency: number;
}

export interface DayPoint {
  date: string;
  tasksCompleted: number;
  tasksCreated: number;
  habitRate: number | null;
  habitsDone: number;
  focusSeconds: number;
  focusSessions: number;
}

export interface WeekPoint {
  week: string;
  tasksCompleted: number;
  habitRate: number | null;
  focusSeconds: number;
}

export interface Analytics {
  days: number;
  tasksCompleted: number;
  tasksOverdue: number;
  tasksPending: number;
  taskCompletionRate: number;
  habitCompletionRate: number;
  focusSeconds: number;
  focusSessions: number;
  daily: DayPoint[];
  weekly: WeekPoint[];
  priorities: { priority: Priority; count: number }[];
  projects: { id: number; name: string; color: string; total: number; done: number; completion: number }[];
  habits: HabitSummary[];
  mostConsistent: HabitSummary[];
  needsImprovement: HabitSummary[];
}

export interface SearchResults {
  tasks: Task[];
  habits: { id: number; name: string; color: string | null; icon: string | null }[];
  projects: { id: number; name: string; color: string | null; icon: string | null }[];
  tags: { id: number; name: string }[];
}

export interface NoticeEntry {
  key: string;
  kind: string;
  title: string;
  body: string;
  taskId: number | null;
  habitId: number | null;
  createdAt: string;
  read: boolean;
}

export interface Settings {
  theme: "system" | "light" | "dark";
  weekStart: "monday" | "sunday" | "saturday";
  timeFormat: "12h" | "24h";
  taskDensity: "comfortable" | "compact";
  sidebarCollapsed: boolean;
  reduceMotion: boolean;
  defaultPriority: Priority;
  defaultReminderMinutes: number | null;
  confirmDelete: boolean;
  closeToBackground: boolean;
  startAtLogin: boolean;
  notificationsEnabled: boolean;
  taskNotifications: boolean;
  habitNotifications: boolean;
  focusNotifications: boolean;
  dateOnlyReminderTime: string;
  focusMinutes: number;
  shortBreakMinutes: number;
  longBreakMinutes: number;
  sessionsBeforeLongBreak: number;
}

export interface BackupCounts {
  tasks: number;
  habits: number;
  habitEntries: number;
  projects: number;
  focusSessions: number;
}

export interface BackupPreview {
  kind: "json" | "database";
  exportedAt: string;
  appVersion: string;
  schemaVersion: number;
  incoming: BackupCounts;
  current: BackupCounts;
}

export interface AppInfo {
  version: string;
  schemaVersion: number;
  dataDir: string | null;
}
