// SPDX-License-Identifier: GPL-3.0-or-later
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// `vite --mode mock` lädt .env.mock (VITE_MOCK=1): die Oberfläche läuft im Browser gegen
// ipc.mock.ts, ohne Tauri und Dienst.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: "es2022", outDir: "dist", emptyOutDir: true },
});
