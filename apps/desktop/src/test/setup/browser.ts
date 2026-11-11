import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/react";
import { afterEach, beforeEach, vi } from "vitest";
import { resetDiagnosticsStore } from "../../store/diagnosticsStore";
import { resetSystemStore } from "../../store/systemStore";

// The browser-preview test project runs *without* the Tauri IPC channel.
// `@tauri-apps/api/core` is replaced by a controllable double so the tests can
// assert both halves of the contract:
//
//   * outside the desktop runtime, no native command may be dispatched;
//   * inside the desktop runtime, a failing command must surface its name and
//     reason instead of silently returning fallback data.
//
// The native Tauri integration tests live in `src/test/native` and exercise the
// real Rust backend; they never use this double.

export const invokeMock = vi.fn<
  (command: string, args?: Record<string, unknown>) => Promise<unknown>
>();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (command: string, args?: Record<string, unknown>) => invokeMock(command, args),
  isTauri: () => (globalThis as Record<string, unknown>).__TAURI_TEST_RUNTIME__ === true
}));

beforeEach(() => {
  invokeMock.mockReset();
  (globalThis as Record<string, unknown>).__TAURI_TEST_RUNTIME__ = false;
  resetSystemStore();
  resetDiagnosticsStore();
});

afterEach(() => {
  cleanup();
});
