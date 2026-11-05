// SPDX-License-Identifier: GPL-3.0-or-later
import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  define: { "import.meta.env.VITE_MOCK": JSON.stringify("1") },
  test: {
    environment: "jsdom",
    setupFiles: ["src/test/setup.ts"],
    css: { modules: { classNameStrategy: "non-scoped" } },
  },
});
