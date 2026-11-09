/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        charcoal: {
          950: "#070B12", 900: "#090E17", 850: "#0B121E", 800: "#0C1421",
          750: "#101B2D", 700: "#121C2D", 650: "#132035", 600: "#19273C"
        },
        accent: { blue: "#2563EB", indigo: "#A855F7", cyan: "#00E5FF", neon: "#00FF87" },
        surface: "#0F1929",
        elevated: "#0F1929",
        status: {
          ok: "#10B981",
          warn: "#F59E0B",
          danger: "#EF4444",
          idle: "#64748B"
        },
        field: "#0A1322",
        ink: {
          primary: "#F0F4F8",
          secondary: "#94A3B8",
          muted: "#64748B"
        }
      },
      borderColor: (theme) => ({
        ...theme("colors"),
        DEFAULT: "#19273C"
      }),
      boxShadow: {
        panel: "0 4px 20px rgba(0, 0, 0, 0.45)",
        pop: "0 2px 8px rgba(0, 0, 0, 0.4), 0 8px 30px rgba(0, 0, 0, 0.55)",
        ringAccent: "0 0 0 1px rgba(0, 229, 255, 0.35)",
        glow: "0 0 25px rgba(0, 229, 255, 0.15)",
        glowGreen: "0 0 25px rgba(0, 255, 135, 0.15)"
      },
      borderRadius: {
        xl2: "0.75rem",
        xl3: "0.875rem"
      },
      fontFamily: {
        ui: ["Inter", "Segoe UI", "system-ui", "sans-serif"],
        mono: ["JetBrains Mono", "Cascadia Code", "Consolas", "ui-monospace", "monospace"]
      }
    }
  },
  plugins: []
};
