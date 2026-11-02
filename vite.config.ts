import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  build: { target: "es2022", chunkSizeWarningLimit: 900 },
  server: {
    port: 1420,
    strictPort: true,
    // In a plain browser the interface talks to dayweave-bridge. See src/lib/api.ts.
    proxy: { "/rpc": "http://127.0.0.1:1421" },
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["src/test/setup.ts"],
    css: false,
  },
});
