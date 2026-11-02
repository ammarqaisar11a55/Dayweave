import { createStore } from "./store";
import type { Habit, Project, TaskInput } from "./types";

export interface Ui {
  taskPanel: number | null;
  taskDialog: Partial<TaskInput> | null;
  habitDialog: { habit?: Habit } | null;
  habitDetail: number | null;
  projectDialog: { project?: Project } | null;
  palette: "commands" | "search" | null;
  shortcuts: boolean;
}

export const ui = createStore<Ui>({
  taskPanel: null,
  taskDialog: null,
  habitDialog: null,
  habitDetail: null,
  projectDialog: null,
  palette: null,
  shortcuts: false,
});

const patch = (next: Partial<Ui>) => ui.set((state) => ({ ...state, ...next }));

export const openTask = (id: number | null) => patch({ taskPanel: id });
export const newTask = (defaults: Partial<TaskInput> = {}) => patch({ taskDialog: defaults });
export const closeTaskDialog = () => patch({ taskDialog: null });
export const newHabit = () => patch({ habitDialog: {} });
export const editHabit = (habit: Habit) => patch({ habitDialog: { habit } });
export const closeHabitDialog = () => patch({ habitDialog: null });
export const openHabit = (id: number | null) => patch({ habitDetail: id });
export const newProject = () => patch({ projectDialog: {} });
export const editProject = (project: Project) => patch({ projectDialog: { project } });
export const closeProjectDialog = () => patch({ projectDialog: null });
export const openPalette = (mode: "commands" | "search") => patch({ palette: mode });
export const closePalette = () => patch({ palette: null });
export const showShortcuts = (open: boolean) => patch({ shortcuts: open });
