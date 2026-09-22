import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Keep the CRA dev-server port so packages/devtools/niva.json
// (`debug.entry: http://localhost:3000`) keeps working.
export default defineConfig({
  plugins: [react()],
  server: {
    port: 3000,
    strictPort: true,
  },
  build: {
    outDir: "build",
    sourcemap: false,
  },
});
