import type { Config } from 'tailwindcss'

export default {
  content: [
    './index.html',
    './src/**/*.{vue,js,ts,jsx,tsx}',
  ],
  theme: {
    extend: {
      colors: {
        text: 'var(--color-text)',
        background: 'var(--color-background)',
        'background-ui': 'var(--color-background-ui)',
        'mid-gray': 'var(--color-mid-gray)',
      },
    },
  },
  plugins: [],
} satisfies Config
