/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        'cs-blue': '#3b82f6',
        'cs-green': '#22c55e',
        'cs-red': '#ef4444',
        'cs-yellow': '#eab308',
        'cs-purple': '#a855f7',
      },
    },
  },
  plugins: [],
}
