import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Two test projects, deliberately separated:
//
//   * `browser` — runs in jsdom with the Tauri IPC layer absent. These tests
//     prove the frontend never calls a native command outside the desktop
//     runtime and that it degrades to explicit browser-preview messaging.
//
//   * `native`  — runs against the real Rust backend through the Tauri IPC
//     bridge. These are the true integration tests: they exercise the actual
//     `#[tauri::command]` implementations and the actual SQLite history.
//
// The separation matters: a test that mocks `invoke` proves the guard works,
// while only a native test proves the backend answers truthfully.
export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    projects: [
      {
        plugins: [react()],
        test: {
          name: "browser",
          environment: "jsdom",
          globals: true,
          setupFiles: ["./src/test/setup/browser.ts"],
          include: ["src/test/browser/**/*.test.ts", "src/test/browser/**/*.test.tsx"],
          // The native project owns the Rust bridge; browser tests must never
          // reach a real WebView.
          exclude: ["src/test/native/**"]
        }
      },
      {
        plugins: [react()],
        test: {
          name: "native",
          environment: "node",
          globals: true,
          include: ["src/test/native/**/*.test.ts"],
          testTimeout: 120_000,
          hookTimeout: 120_000
        }
      }
    ]
  }
});
