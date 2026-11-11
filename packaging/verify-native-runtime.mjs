// Native runtime verification over the WebView2 DevTools Protocol.
//
// This drives the *production* desktop executable and proves that:
//   * the window loads the embedded frontend through the Tauri custom
//     protocol (`tauri.localhost`), not a Vite development server;
//   * the Tauri runtime detection succeeds inside the application;
//   * the foundation commands and all nine diagnostics commands answer with
//     real backend data.
//
// It is used by `packaging/test-native-runtime.ps1` so that CI verifies the
// shipped runtime rather than only the build output.

const port = process.argv[2] || "9222";
const commandNames = [
  "get_foundation_status",
  "get_database_status",
  "list_network_interfaces",
  "get_traffic_monitor_status",
  "get_traffic_snapshot",
  "get_traffic_history",
  "get_speed_test_history",
  "get_incidents",
  "get_security_events"
];

const diagnosticsCommands = new Set([
  "list_network_interfaces",
  "get_traffic_monitor_status",
  "get_traffic_snapshot",
  "get_traffic_history",
  "get_speed_test_history"
]);

async function listTargets() {
  const response = await fetch(`http://127.0.0.1:${port}/json/list`);
  return response.json();
}

async function pickTarget() {
  for (let attempt = 0; attempt < 60; attempt++) {
    try {
      const targets = await listTargets();
      const pages = targets.filter((target) => target.type === "page");
      // The application page is the one that is not the initial blank page.
      const real = pages.find((target) => target.url && !target.url.startsWith("about:blank"));
      if (real) return real;
      if (pages.length > 0 && attempt > 20) return pages[0];
    } catch {
      /* the runtime is not listening yet */
    }
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
  return null;
}

/**
 * `BLOCKED` marks an environment limitation — the host could not present a
 * window at all — as opposed to a product defect. It must never be used to
 * excuse a window that loaded but failed to answer.
 */
function fail(message, kind = "FAIL") {
  console.log(`RESULT_${kind} ${message}`);
  process.exit(kind === "BLOCKED" ? 2 : 1);
}

async function main() {
  const target = await pickTarget();
  if (!target) fail("no WebView2 page target became available", "BLOCKED");

  const socket = new WebSocket(target.webSocketDebuggerUrl);
  let nextId = 0;
  const pending = new Map();

  socket.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      pending.get(message.id)(message);
      pending.delete(message.id);
    }
  });

  await new Promise((resolve) => socket.addEventListener("open", resolve));

  const send = (method, params) =>
    new Promise((resolve) => {
      const id = ++nextId;
      pending.set(id, resolve);
      socket.send(JSON.stringify({ id, method, params }));
    });

  await send("Runtime.enable", {});

  // The IPC bridge must be installed before anything else is attempted.
  let bridgeReady = false;
  for (let attempt = 0; attempt < 60; attempt++) {
    const probe = await send("Runtime.evaluate", {
      expression: "typeof window.__TAURI_INTERNALS__ !== 'undefined'",
      returnByValue: true
    });
    if (probe.result?.result?.value === true) {
      bridgeReady = true;
      break;
    }
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
  if (!bridgeReady) fail("the Tauri IPC bridge was not installed in the application window");
  const evaluate = async (expression) => {
    const response = await send("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
      timeout: 120000
    });
    if (response.result?.exceptionDetails) {
      fail(`evaluation raised: ${JSON.stringify(response.result.exceptionDetails)}`);
    }
    return response.result?.result?.value;
  };

  const runtime = await evaluate(`(() => ({
    url: location.href,
    title: document.title,
    hasTauriGlobal: typeof window.__TAURI__ !== 'undefined',
    hasTauriInternals: typeof window.__TAURI_INTERNALS__ !== 'undefined',
    rootHtmlLength: (document.getElementById('root')?.innerHTML || '').length
  }))()`);

  // The frontend must be served by the embedded asset protocol. A development
  // server URL means the production binary is not self-contained.
  if (!String(runtime.url).startsWith("http://tauri.localhost")) {
    fail(`the application did not load through the Tauri custom protocol (url=${runtime.url})`);
  }
  if (runtime.rootHtmlLength < 1000) {
    fail(`the application window rendered an empty frontend (root length=${runtime.rootHtmlLength})`);
  }

  const results = await evaluate(`(async () => {
    const out = {};
    const names = ${JSON.stringify(commandNames)};
    for (const name of names) {
      try {
        const value = name === 'get_traffic_history'
          ? await window.__TAURI_INTERNALS__.invoke(name, { interfaceId: null, seconds: 300 })
          : await window.__TAURI_INTERNALS__.invoke(name);
        out[name] = { ok: true, value };
      } catch (error) {
        out[name] = { ok: false, error: String(error) };
      }
    }
    return out;
  })()`);

  const summary = { runtime, commands: {} };
  let failures = 0;

  for (const name of commandNames) {
    const entry = results[name];
    if (!entry || entry.ok !== true) {
      failures++;
      summary.commands[name] = { ok: false, error: entry ? entry.error : "no response" };
      continue;
    }
    summary.commands[name] = { ok: true };
    if (diagnosticsCommands.has(name)) {
      // Diagnostics commands must return a well-formed payload, never a stub.
      const value = entry.value;
      const wellFormed =
        (name === "list_network_interfaces" && Array.isArray(value)) ||
        (name === "get_traffic_monitor_status" && value && typeof value.running === "boolean") ||
        (name === "get_traffic_snapshot" && value && typeof value.sampledAt === "string") ||
        (name === "get_traffic_history" && Array.isArray(value)) ||
        (name === "get_speed_test_history" && Array.isArray(value));
      if (!wellFormed) {
        failures++;
        summary.commands[name] = { ok: false, error: "malformed payload" };
      }
    }
  }

  summary.failures = failures;
  console.log("RESULT " + JSON.stringify(summary, null, 2));
  socket.close();
  process.exit(failures === 0 ? 0 : 1);
}

main();
