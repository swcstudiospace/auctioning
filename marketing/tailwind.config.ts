import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./app/**/*.{ts,tsx}", "./components/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        mint: "#F0F7F2",
        forest: "#3E8E62",
        forestDark: "#2F6B4A",
        ink: "#111111",
      },
      fontFamily: {
        mono: ["var(--font-ibm)", "ui-monospace", "monospace"],
        sans: ["var(--font-ibm)", "ui-sans-serif", "system-ui"],
      },
      keyframes: {
        marquee: {
          from: { transform: "translateX(0)" },
          to: { transform: "translateX(-50%)" },
        },
        shine: {
          from: { backgroundPosition: "0% 0%" },
          to: { backgroundPosition: "-200% 0%" },
        },
        beam: {
          from: { opacity: "0.15" },
          to: { opacity: "0.55" },
        },
      },
      animation: {
        marquee: "marquee 28s linear infinite",
        shine: "shine 3s linear infinite",
        beam: "beam 3s ease-in-out infinite alternate",
      },
    },
  },
  plugins: [],
};
export default config;
