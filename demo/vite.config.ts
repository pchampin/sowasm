import {defineConfig} from "vite";

// https://vitejs.dev/config/
export default defineConfig({
  // base: "/pierre-antoine.champin/2023/sowasm/",
  server: {
    port: 8000,
  },
  resolve: {
    preserveSymlinks: true
  },
  build: {
    outDir: "./dist",
  },
});
