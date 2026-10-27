/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        ink: {
          950: "var(--ink-950)",
          900: "var(--ink-900)",
          800: "var(--ink-800)",
          700: "var(--ink-700)",
          500: "var(--ink-500)",
          300: "var(--ink-300)",
          100: "var(--ink-100)",
        },
        foam: "var(--foam)",
        ember: {
          DEFAULT: "var(--ember)",
          soft: "var(--ember-soft)",
        },
        tide: {
          DEFAULT: "var(--tide)",
          soft: "var(--tide-soft)",
        },
      },
      fontFamily: {
        display: ["Fraunces", "Georgia", "serif"],
        sans: ["Sora", "ui-sans-serif", "system-ui", "sans-serif"],
        mono: ["IBM Plex Mono", "ui-monospace", "monospace"],
      },
      boxShadow: {
        panel: "0 24px 60px rgba(8, 18, 28, 0.35)",
      },
      keyframes: {
        rise: {
          "0%": { opacity: "0", transform: "translateY(10px)" },
          "100%": { opacity: "1", transform: "translateY(0)" },
        },
        pulseDot: {
          "0%, 100%": { opacity: "1", transform: "scale(1)" },
          "50%": { opacity: "0.55", transform: "scale(0.85)" },
        },
        sweep: {
          "0%": { transform: "translateX(-30%)" },
          "100%": { transform: "translateX(130%)" },
        },
      },
      animation: {
        rise: "rise 0.55s ease-out both",
        "pulse-dot": "pulseDot 1.8s ease-in-out infinite",
        sweep: "sweep 2.4s ease-in-out infinite",
      },
    },
  },
  plugins: [],
};
