/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./components/**/*.{js,vue,ts}",
    "./layouts/**/*.vue",
    "./pages/**/*.vue",
    "./plugins/**/*.{js,ts}",
    "./app.vue",
    "./error.vue",
    "../libraries/base/components/**/*.{js,vue,ts}",
    "../libraries/base/composables/**/*.{js,ts}",
    "../libraries/base/*.vue",
  ],
  theme: {
    extend: {
      fontFamily: {
        sans: ["Inter"],
        display: ["Motiva Sans"],
      },
      colors: {
        zinc: {
          925: "#111112",
        },
      },
    },
  },
};
