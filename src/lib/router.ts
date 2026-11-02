import { createStore } from "./store";

export type Page =
  | "overview"
  | "tasks"
  | "projects"
  | "habits"
  | "calendar"
  | "analytics"
  | "focus"
  | "settings"
  | "about";

const PAGES: Page[] = [
  "overview",
  "tasks",
  "projects",
  "habits",
  "calendar",
  "analytics",
  "focus",
  "settings",
  "about",
];

export interface Route {
  page: Page;
  query: URLSearchParams;
}

function parse(): Route {
  const [path = "", search = ""] = window.location.hash.replace(/^#\/?/, "").split("?");
  const page = PAGES.includes(path as Page) ? (path as Page) : "overview";
  return { page, query: new URLSearchParams(search) };
}

const route = createStore<Route>(parse());

window.addEventListener("hashchange", () => route.set(parse()));

export const useRoute = route.use;

export function navigate(page: Page, query: Record<string, string> = {}) {
  const search = new URLSearchParams(query).toString();
  window.location.hash = `/${page}${search ? `?${search}` : ""}`;
}
