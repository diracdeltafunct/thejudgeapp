import { defineConfig } from "vitest/config";
import { fileURLToPath } from "url";
import path from "path";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  test: {
    environment: "jsdom",
    globals: true,
    alias: {
      // Stub out Tauri runtime APIs — tests run in Node, not Tauri WebView
      "@tauri-apps/api/core": path.resolve(__dirname, "src/__mocks__/tauri-core.ts"),
      "@tauri-apps/api/event": path.resolve(__dirname, "src/__mocks__/tauri-event.ts"),
      "@tauri-apps/plugin-os": path.resolve(__dirname, "src/__mocks__/tauri-os.ts"),
      "@tauri-apps/plugin-shell": path.resolve(__dirname, "src/__mocks__/tauri-shell.ts"),
    },
  },
});
