import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  root: "webview-ui",
  plugins: [react()],
  build: {
    outDir: "../media/webview",
    emptyOutDir: true,
    sourcemap: false,
    rollupOptions: {
      input: "webview-ui/index.html",
      output: {
        entryFileNames: "assets/index.js",
        chunkFileNames: "assets/[name].js",
        assetFileNames: "assets/[name][extname]",
      },
    },
  },
});
