/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        charcoal: { 950: "#0b0f14", 900: "#121821", 800: "#1a2230", 700: "#243044" },
        accent: { blue: "#3b82f6", indigo: "#6366f1", cyan: "#22d3ee" }
      }
    }
  },
  plugins: []
};
