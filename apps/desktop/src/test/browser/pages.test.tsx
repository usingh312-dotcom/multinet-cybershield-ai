import { describe, expect, it } from "vitest";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import NetworkManagerPage from "../../pages/NetworkManager";
import TrafficAnalyticsPage from "../../pages/TrafficAnalytics";
import SpeedTestPage from "../../pages/SpeedTest";
import { invokeMock } from "../setup/browser";

function enterNativeRuntime() {
  (globalThis as Record<string, unknown>).__TAURI_TEST_RUNTIME__ = true;
}

/**
 * Builds a command handler that answers only the listed commands.
 *
 * A value shaped like the backend's structured `IpcError` is rejected, which
 * is exactly how Tauri surfaces a failed command; anything else resolves.
 */
function respondWith(responses: Record<string, unknown>) {
  invokeMock.mockImplementation(async (command: string) => {
    if (!(command in responses)) {
      throw new Error(`unexpected command: ${command}`);
    }
    const value = responses[command];
    if (value instanceof Error) throw value;
    if (
      typeof value === "object" &&
      value !== null &&
      typeof (value as { code?: unknown }).code === "string" &&
      typeof (value as { message?: unknown }).message === "string"
    ) {
      throw value;
    }
    return value;
  });
}

/** Returns the status card that carries the given title. */
function card(title: string): HTMLElement {
  const heading = screen.getByText(title);
  const card = heading.closest("div.rounded-xl3");
  if (!card) throw new Error(`no status card found for "${title}"`);
  return card as HTMLElement;
}

const EMPTY_INTERFACES: unknown[] = [];

describe("Network Manager — empty and error states", () => {
  it("shows the browser-preview notice and never calls a native command", async () => {
    render(<NetworkManagerPage />);

    await waitFor(() => {
      expect(screen.getByText(/Browser preview/)).toBeInTheDocument();
    });

    expect(invokeMock).not.toHaveBeenCalled();
    expect(screen.getByText(/No network interfaces available/)).toBeInTheDocument();
  });

  it("shows the empty state when the operating system reports no adapter", async () => {
    enterNativeRuntime();
    respondWith({
      get_foundation_status: {
        product: "MultiNet CyberShield AI",
        version: "0.1.0",
        phase: "Phase 1",
        note: "note"
      },
      get_database_status: {
        db_file: "C:/tmp/app.db",
        migrations_applied: 3,
        latest_version: 3,
        foreign_keys_enabled: true
      },
      list_network_interfaces: EMPTY_INTERFACES,
      get_traffic_snapshot: {
        sampledAt: "2026-11-11T00:00:00Z",
        source: "GetIfTable2",
        monitorActive: true,
        interfaces: [],
        physicalRxBps: null,
        physicalTxBps: null,
        sessionRxBytes: 0,
        sessionTxBytes: 0,
        totalRxBytes: 0,
        totalTxBytes: 0,
        peakCombinedBps: 0
      }
    });

    render(<NetworkManagerPage />);

    await waitFor(() => {
      expect(screen.getByText(/No network interfaces available/)).toBeInTheDocument();
    });

    // The count must reflect the empty backend result, not a placeholder.
    expect(card("Available Interfaces").textContent).toContain("0");
    expect(card("Active Connections").textContent).toContain("0");
  });

  it("reports the failing native command and invents no interface record", async () => {
    enterNativeRuntime();
    respondWith({
      get_foundation_status: {
        product: "MultiNet CyberShield AI",
        version: "0.1.0",
        phase: "Phase 1",
        note: "note"
      },
      get_database_status: {
        db_file: "C:/tmp/app.db",
        migrations_applied: 3,
        latest_version: 3,
        foreign_keys_enabled: true
      },
      list_network_interfaces: new Error("interface discovery failed"),
      get_traffic_snapshot: {
        sampledAt: "2026-11-11T00:00:00Z",
        source: "GetIfTable2",
        monitorActive: true,
        interfaces: [],
        physicalRxBps: null,
        physicalTxBps: null,
        sessionRxBytes: 0,
        sessionTxBytes: 0,
        totalRxBytes: 0,
        totalTxBytes: 0,
        peakCombinedBps: 0
      }
    });

    render(<NetworkManagerPage />);

    await waitFor(() => {
      expect(screen.getByText(/list_network_interfaces/)).toBeInTheDocument();
    });
    expect(screen.getByText(/interface discovery failed/)).toBeInTheDocument();
    expect(screen.getByText(/No network interfaces available/)).toBeInTheDocument();
  });

  it("lists the real adapters the backend reports", async () => {    enterNativeRuntime();
    respondWith({
      get_foundation_status: {
        product: "MultiNet CyberShield AI",
        version: "0.1.0",
        phase: "Phase 1",
        note: "note"
      },
      get_database_status: {
        db_file: "C:/tmp/app.db",
        migrations_applied: 3,
        latest_version: 3,
        foreign_keys_enabled: true
      },
      list_network_interfaces: [
        {
          id: "{IF-1}",
          name: "Ethernet",
          description: "Realtek Gaming GbE Family Controller",
          kind: "ethernet",
          interfaceIndex: 3,
          isUp: true,
          isPhysical: true,
          ipv4Addresses: ["192.168.1.20"],
          ipv6Addresses: ["fe80::1"],
          macAddress: "48:9e:bd:28:67:d3",
          mtu: 1500,
          linkSpeedBps: 1000000000
        }
      ],
      get_traffic_snapshot: {
        sampledAt: "2026-11-11T00:00:00Z",
        source: "GetIfTable2",
        monitorActive: true,
        interfaces: [
          {
            interfaceId: "{IF-1}",
            interfaceIndex: 3,
            name: "Ethernet",
            description: "Realtek Gaming GbE Family Controller",
            kind: "ethernet",
            isPhysical: true,
            isUp: true,
            rxBps: 8000000,
            txBps: 2000000,
            sessionRxBytes: 1024,
            sessionTxBytes: 512,
            totalRxBytes: 2048,
            totalTxBytes: 1024,
            counterReset: false
          }
        ],
        physicalRxBps: 8000000,
        physicalTxBps: 2000000,
        sessionRxBytes: 1024,
        sessionTxBytes: 512,
        totalRxBytes: 2048,
        totalTxBytes: 1024,
        peakCombinedBps: 10000000
      }
    });

    render(<NetworkManagerPage />);

    await waitFor(() => {
      expect(screen.getAllByText("Ethernet").length).toBeGreaterThan(0);
    });

    expect(screen.getByText("Realtek Gaming GbE Family Controller")).toBeInTheDocument();
    expect(screen.getByText("192.168.1.20")).toBeInTheDocument();
    expect(screen.getByText("physical")).toBeInTheDocument();
  });
});

describe("Traffic Analytics — empty state and no fabricated measurements", () => {
  it("shows the empty chart state when the sampler has no history", async () => {
    enterNativeRuntime();
    respondWith({
      get_foundation_status: {
        product: "MultiNet CyberShield AI",
        version: "0.1.0",
        phase: "Phase 1",
        note: "note"
      },
      get_database_status: {
        db_file: "C:/tmp/app.db",
        migrations_applied: 3,
        latest_version: 3,
        foreign_keys_enabled: true
      },
      get_traffic_snapshot: {
        sampledAt: "2026-11-11T00:00:00Z",
        source: "GetIfTable2",
        monitorActive: true,
        interfaces: [],
        physicalRxBps: null,
        physicalTxBps: null,
        sessionRxBytes: 0,
        sessionTxBytes: 0,
        totalRxBytes: 0,
        totalTxBytes: 0,
        peakCombinedBps: 0
      },
      get_traffic_history: [],
      get_traffic_monitor_status: { running: true, lastSampleAgeMs: 12, source: "GetIfTable2" }
    });

    render(<TrafficAnalyticsPage />);

    await waitFor(() => {
      expect(screen.getByText(/No traffic data to chart/)).toBeInTheDocument();
    });

    // Regression guard: no rate may be displayed when the backend reports none.
    const overview = card("Current Download Rate");
    expect(overview.textContent).toContain("—");
    expect(overview.textContent).not.toMatch(/\d+\.\d+ Mbps/);
  });

  it("renders the rates the backend actually measured", async () => {
    enterNativeRuntime();
    respondWith({
      get_foundation_status: {
        product: "MultiNet CyberShield AI",
        version: "0.1.0",
        phase: "Phase 1",
        note: "note"
      },
      get_database_status: {
        db_file: "C:/tmp/app.db",
        migrations_applied: 3,
        latest_version: 3,
        foreign_keys_enabled: true
      },
      get_traffic_snapshot: {
        sampledAt: "2026-11-11T00:00:00Z",
        source: "GetIfTable2",
        monitorActive: true,
        interfaces: [],
        physicalRxBps: 125_000,
        physicalTxBps: 62_500,
        sessionRxBytes: 1024,
        sessionTxBytes: 512,
        totalRxBytes: 2048,
        totalTxBytes: 1024,
        peakCombinedBps: 187_500
      },
      get_traffic_history: [],
      get_traffic_monitor_status: { running: true, lastSampleAgeMs: 12, source: "GetIfTable2" }
    });

    render(<TrafficAnalyticsPage />);

    await waitFor(() => {
      expect(card("Current Download Rate").textContent).toContain("1.00 Mbps");
    });
    expect(card("Current Upload Rate").textContent).toContain("500 Kbps");
  });
});

describe("Speed Test — lifecycle and honest failure reporting", () => {
  it("does not start a measurement outside the desktop runtime", async () => {
    render(<SpeedTestPage />);

    await waitFor(() => {
      expect(screen.getByText(/Browser preview/)).toBeInTheDocument();
    });

    expect(invokeMock).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: /Start Test/ })).toHaveAttribute(
      "aria-disabled",
      "true"
    );
  });

  it("shows the measured result once the backend reports one", async () => {
    enterNativeRuntime();
    respondWith({
      get_foundation_status: {
        product: "MultiNet CyberShield AI",
        version: "0.1.0",
        phase: "Phase 1",
        note: "note"
      },
      get_database_status: {
        db_file: "C:/tmp/app.db",
        migrations_applied: 3,
        latest_version: 3,
        foreign_keys_enabled: true
      },
      list_network_interfaces: [],
      get_speed_test_history: [
        {
          id: "run-1",
          serverUrl: "https://127.0.0.1:9443",
          serverName: "Local",
          interfaceId: null,
          interfaceLocalIp: "192.168.1.20",
          downloadMbps: 412.5,
          uploadMbps: 88.25,
          pingMs: 4.5,
          jitterMs: 0.75,
          lossPct: 0,
          durationMs: 900,
          startedAt: "2026-11-11T00:00:00Z",
          completedAt: "2026-11-11T00:00:01Z"
        }
      ]
    });

    render(<SpeedTestPage />);

    await waitFor(() => {
      expect(screen.getAllByText("412.50 Mbps").length).toBeGreaterThan(0);
    });

    // The history row must come from the backend record, not from a stub.
    expect(screen.getAllByText("88.25 Mbps").length).toBeGreaterThan(0);
    expect(screen.getAllByText("4.50 ms").length).toBeGreaterThan(0);
  });

  it("reports a failed measurement honestly instead of showing a number", async () => {
    enterNativeRuntime();
    respondWith({
      get_foundation_status: {
        product: "MultiNet CyberShield AI",
        version: "0.1.0",
        phase: "Phase 1",
        note: "note"
      },
      get_database_status: {
        db_file: "C:/tmp/app.db",
        migrations_applied: 3,
        latest_version: 3,
        foreign_keys_enabled: true
      },
      list_network_interfaces: [],
      get_speed_test_history: [],
      start_speed_test: {
        code: "invalid_input",
        message: "speed test server must use HTTPS",
        retryable: false
      }
    });

    const user = userEvent.setup();
    render(<SpeedTestPage />);

    await waitFor(() => {
      expect(screen.getByRole("button", { name: /Start Test/ })).toHaveAttribute(
        "aria-disabled",
        "false"
      );
    });

    await user.click(screen.getByRole("button", { name: /Start Test/ }));

    await waitFor(() => {
      expect(screen.getByText(/start_speed_test/)).toBeInTheDocument();
    });
    expect(screen.getByText(/must use HTTPS/)).toBeInTheDocument();

    // No result card may show a measured value after a failed start.
    expect(screen.queryByText(/412\.50/)).toBeNull();
  });
});
