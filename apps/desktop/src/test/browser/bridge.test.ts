import { describe, expect, it } from "vitest";
import {
  BROWSER_PREVIEW_MESSAGE,
  NativeCommandError,
  detectNativeRuntime,
  getDatabaseStatus,
  getFoundationStatus,
  invokeNative,
  isNativeRuntime
} from "../../api";
import { invokeMock } from "../setup/browser";

function enterNativeRuntime() {
  (globalThis as Record<string, unknown>).__TAURI_TEST_RUNTIME__ = true;
}

describe("browser runtime detection", () => {
  it("reports a browser preview when the Tauri runtime is absent", () => {
    const runtime = detectNativeRuntime();

    expect(runtime.isTauri).toBe(false);
    expect(isNativeRuntime()).toBe(false);
    expect(runtime.previewMessage).toBe(BROWSER_PREVIEW_MESSAGE);
    expect(runtime.previewMessage).toMatch(/Browser preview/);
  });

  it("reports the native runtime when the Tauri bridge is present", () => {
    enterNativeRuntime();

    const runtime = detectNativeRuntime();

    expect(runtime.isTauri).toBe(true);
    expect(isNativeRuntime()).toBe(true);
    expect(runtime.previewMessage).toBe("");
  });

  it("never dispatches a native command outside the desktop runtime", async () => {
    await expect(getFoundationStatus()).rejects.toBeInstanceOf(NativeCommandError);
    await expect(getDatabaseStatus()).rejects.toBeInstanceOf(NativeCommandError);

    expect(invokeMock).not.toHaveBeenCalled();
  });

  it("explains that the native bridge is unavailable when refusing a command", async () => {
    const error = await getFoundationStatus().then(
      () => null,
      (e: unknown) => e as NativeCommandError
    );

    expect(error).toBeInstanceOf(NativeCommandError);
    expect(error?.command).toBe("get_foundation_status");
    expect(error?.isRuntimeUnavailable).toBe(true);
    expect(error?.reason).toBe(BROWSER_PREVIEW_MESSAGE);
  });

  it("dispatches the command inside the desktop runtime", async () => {
    enterNativeRuntime();
    invokeMock.mockResolvedValue({
      product: "MultiNet CyberShield AI",
      version: "0.1.0",
      phase: "Phase 1 - Foundation and Architecture",
      note: "No live network/security operations, no fake metrics, no system mutations."
    });

    const status = await getFoundationStatus();

    expect(invokeMock).toHaveBeenCalledWith("get_foundation_status", undefined);
    expect(status.product).toBe("MultiNet CyberShield AI");
  });

  it("captures the exact command name and reason when a native call fails", async () => {
    enterNativeRuntime();
    invokeMock.mockRejectedValue(new Error("the state is not managed"));

    const error = await invokeNative<string>("get_traffic_snapshot").then(
      () => null,
      (e: unknown) => e as NativeCommandError
    );

    expect(error).toBeInstanceOf(NativeCommandError);
    expect(error?.command).toBe("get_traffic_snapshot");
    expect(error?.reason).toBe("the state is not managed");
    expect(error?.isRuntimeUnavailable).toBe(false);
    expect(error?.message).toContain("get_traffic_snapshot");
  });

  it("never substitutes fallback data for a failed native command", async () => {
    enterNativeRuntime();
    invokeMock.mockRejectedValue(new Error("database unavailable"));

    // The rejection must propagate: no partially filled status object may be
    // returned in place of the failed call.
    const outcome = await getDatabaseStatus().then(
      (value) => ({ resolved: true as const, value }),
      (error) => ({ resolved: false as const, error })
    );

    expect(outcome.resolved).toBe(false);
    if (!outcome.resolved) {
      expect(outcome.error).toBeInstanceOf(NativeCommandError);
      expect((outcome.error as NativeCommandError).command).toBe("get_database_status");
    }
  });
});
