import { invoke } from "@tauri-apps/api/core";

export class ApiError extends Error {
  constructor(
    readonly code: string,
    message: string,
  ) {
    super(message);
  }
}

export const inTauri = "__TAURI_INTERNALS__" in window;

function asApiError(error: unknown): ApiError {
  if (error && typeof error === "object" && "message" in error) {
    const { code, message } = error as { code?: string; message: string };
    return new ApiError(code ?? "unknown", message);
  }
  return new ApiError("unknown", String(error));
}

/** Calls the core. In a plain browser this reaches dayweave-bridge, which runs the same dispatcher. */
export async function rpc<T>(method: string, params: object = {}): Promise<T> {
  if (inTauri) {
    try {
      return await invoke<T>("rpc", { method, params });
    } catch (error) {
      throw asApiError(error);
    }
  }
  const response = await fetch("/rpc", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ method, params }),
  });
  const body = await response.json();
  if (!response.ok) throw asApiError(body);
  return body as T;
}
