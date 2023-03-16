/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./src/**/*.{js,jsx,ts,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        "table-background-light": "#666666",
        "table-background-dark": "#333333",
      },
    },
  },
  plugins: [
    require('tailwind-scrollbar'),
  ],
}
