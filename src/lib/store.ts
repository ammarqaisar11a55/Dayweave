import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from "react";

import { ApiError, rpc } from "./api";

/** A minimal external store for state shared across the application. */
export function createStore<T>(initial: T) {
  let state = initial;
  const listeners = new Set<() => void>();
  const subscribe = (listener: () => void) => {
    listeners.add(listener);
    return () => void listeners.delete(listener);
  };
  return {
    get: () => state,
    set(next: T | ((previous: T) => T)) {
      state = typeof next === "function" ? (next as (previous: T) => T)(state) : next;
      listeners.forEach((listener) => listener());
    },
    use: () => useSyncExternalStore(subscribe, () => state),
  };
}

// Every write bumps this, and every mounted query refetches. Against a local database that is cheap,
// and it means no view can ever show stale data. See ARCHITECTURE.md.
const dataVersion = createStore(0);

export function refreshData() {
  dataVersion.set((version) => version + 1);
}

export interface Query<T> {
  data: T | undefined;
  error: ApiError | undefined;
  loading: boolean;
}

export function useRpc<T>(method: string, params: object = {}, enabled = true): Query<T> {
  const version = dataVersion.use();
  const key = JSON.stringify(params);
  const [state, setState] = useState<Query<T>>({ data: undefined, error: undefined, loading: enabled });
  const latest = useRef(0);

  useEffect(() => {
    if (!enabled) return;
    const request = ++latest.current;
    rpc<T>(method, JSON.parse(key) as object).then(
      (data) => {
        if (request === latest.current) setState({ data, error: undefined, loading: false });
      },
      (error: ApiError) => {
        if (request === latest.current) setState((previous) => ({ ...previous, error, loading: false }));
      },
    );
  }, [method, key, version, enabled]);

  return state;
}

export type Toast = {
  id: number;
  message: string;
  tone: "neutral" | "error";
  action?: { label: string; run: () => void };
};

export const toasts = createStore<Toast[]>([]);
let nextToast = 1;

export function dismissToast(id: number) {
  toasts.set((list) => list.filter((toast) => toast.id !== id));
}

export function showToast(message: string, options: Partial<Pick<Toast, "tone" | "action">> = {}) {
  const id = nextToast++;
  toasts.set((list) => [...list.slice(-3), { id, message, tone: options.tone ?? "neutral", ...options }]);
  window.setTimeout(() => dismissToast(id), options.tone === "error" ? 7000 : options.action ? 6000 : 3500);
}

/** Runs a write, then refreshes every view. Failures become an error toast and are rethrown. */
export async function mutate<T>(method: string, params: object = {}): Promise<T> {
  try {
    const result = await rpc<T>(method, params);
    refreshData();
    return result;
  } catch (error) {
    showToast((error as Error).message, { tone: "error" });
    throw error;
  }
}

/** The same as `mutate` for fire-and-forget handlers, where the toast is all the handling needed. */
export function act(method: string, params: object = {}, success?: string) {
  return mutate(method, params).then(
    () => {
      if (success) showToast(success);
    },
    () => undefined,
  );
}

export function useStableCallback<A extends unknown[], R>(callback: (...args: A) => R) {
  const reference = useRef(callback);
  useEffect(() => {
    reference.current = callback;
  });
  return useCallback((...args: A) => reference.current(...args), []);
}
