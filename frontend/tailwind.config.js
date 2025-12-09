/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {
      colors: {
        // Semantic colors mapping to CSS variables
        bg: {
          main: 'var(--bg-main)',
          card: 'var(--bg-card)',
          sidebar: 'var(--bg-sidebar)',
          input: 'var(--bg-input)',
        },
        text: {
          main: 'var(--text-main)',
          muted: 'var(--text-muted)',
          sidebar: 'var(--text-sidebar)',
          'sidebar-muted': 'var(--text-sidebar-muted)',
        },
        border: {
          main: 'var(--border-main)',
        },
        primary: {
          DEFAULT: 'var(--primary)',
          hover: 'var(--primary-hover)',
          fg: 'var(--primary-fg)',
        }
      }
    },
  },
  plugins: [],
}
