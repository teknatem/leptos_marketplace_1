/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./crates/frontend/index.html",
    "./crates/frontend/src/**/*.rs",
  ],
  corePlugins: {
    // IMPORTANT: do not reset existing UI; we already have base/reset/theme CSS
    preflight: false,
  },
  theme: {
    extend: {},
  },
  plugins: [],
};
