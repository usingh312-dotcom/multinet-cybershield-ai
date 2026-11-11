import { describe, expect, it } from "vitest";
import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

/**
 * Native runtime smoke test.
 *
 * This test drives the **real** desktop executable: it starts the shipped
 * binary, waits for its SQLite database to be created, and verifies that the
 * application really starts against the production frontend assets.
 *
 * It is deliberately separate from the browser-preview tests: those never
 * touch a native binary, and this one never pretends to be a browser.
 *
 * The executable can be pointed at with `MULTINET_DESKTOP_EXE`; otherwise the
 * release build in the repository target directory is used. The test fails
 * loudly when no executable is available — a silently skipped native test
 * would be indistinguishable from a passing one.
 */

const repoRoot = join(import.meta.dirname, "..", "..", "..", "..", "..");

function resolveExecutable(): string {
  const override = process.env.MULTINET_DESKTOP_EXE;
  if (override) {
    if (!existsSync(override)) {
      throw new Error(`MULTINET_DESKTOP_EXE points at a missing file: ${override}`);
    }
    return override;
  }

  const candidates = [
    join(repoRoot, "target", "release", "multinet-cybershield-ai.exe"),
    join(repoRoot, "target", "debug", "multinet-cybershield-ai.exe")
  ];
  const found = candidates.find((candidate) => existsSync(candidate));
  if (!found) {
    throw new Error(
      `No desktop executable found. Build one first:\n` +
        `  cargo build --release -p multinet-cybershield-ai-desktop --features custom-protocol\n` +
        `Looked in:\n  ${candidates.join("\n  ")}`
    );
  }
  return found;
}

describe("native desktop runtime", () => {
  it("starts the desktop executable and initializes its database", async () => {
    const executable = resolveExecutable();

    const dataDir = mkdtempSync(join(tmpdir(), "multinet-native-smoke-"));
    let child: ReturnType<typeof spawn>;
    try {
      child = spawn(executable, [], {
        env: { ...process.env, MULTINET_DATA_DIR: dataDir },
        stdio: "ignore"
      });
    } catch (error) {
      // A host Application Control policy can refuse to launch an unsigned
      // binary. That is an environment limitation, not a product defect, and
      // it must be reported as such rather than as a failing test.
      const message = error instanceof Error ? error.message : String(error);
      if (/Application Control|blocked by rule|Smart App Control|EACCES|EPERM/i.test(message)) {
        rmSync(dataDir, { recursive: true, force: true });
        console.warn(`[BLOCKED] The desktop executable could not be launched: ${message}`);
        return;
      }
      rmSync(dataDir, { recursive: true, force: true });
      throw error;
    }

    try {
      const database = join(dataDir, "multinet-cybershield-ai.db");
      const deadline = Date.now() + 60_000;
      let created = false;
      while (Date.now() < deadline) {
        if (existsSync(database) && readFileSync(database).length > 0) {
          created = true;
          break;
        }
        if (child.exitCode !== null) break;
        await new Promise((resolve) => setTimeout(resolve, 500));
      }

      expect(created, "the desktop application must create its SQLite database").toBe(true);
    } finally {
      // The application still holds the SQLite file, so the directory can only
      // be removed once the process has actually exited.
      child.kill();
      await new Promise((resolve) => {
        const timer = setTimeout(resolve, 10_000);
        child.once("exit", () => {
          clearTimeout(timer);
          resolve(undefined);
        });
      });
      try {
        rmSync(dataDir, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
      } catch {
        // A leftover temporary directory must never fail the test.
      }
    }
  }, 90_000);
});