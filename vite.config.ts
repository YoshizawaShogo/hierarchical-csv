import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri は固定ポートを期待する
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
});
