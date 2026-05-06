/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{ts,tsx}'],
  darkMode: 'class',
  theme: {
    extend: {
      fontFamily: {
        sans: ['"Inter Tight"', '"Hiragino Sans"', 'system-ui', 'sans-serif'],
        mono: ['"JetBrains Mono"', 'ui-monospace', 'monospace'],
      },
      colors: {
        // Dark (default)
        bg: 'oklch(var(--c-bg) / <alpha-value>)',
        panel: 'oklch(var(--c-panel) / <alpha-value>)',
        panel2: 'oklch(var(--c-panel2) / <alpha-value>)',
        ink: 'oklch(var(--c-ink) / <alpha-value>)',
        mute: 'oklch(var(--c-mute) / <alpha-value>)',
        faint: 'oklch(var(--c-faint) / <alpha-value>)',
        border: 'oklch(var(--c-border) / <alpha-value>)',
        accent: 'oklch(var(--c-accent) / <alpha-value>)',
        high: 'oklch(var(--c-high) / <alpha-value>)',
        med: 'oklch(var(--c-med) / <alpha-value>)',
      },
      fontSize: {
        '2xs': '0.6875rem', // 11px
      },
    },
  },
  plugins: [],
};
