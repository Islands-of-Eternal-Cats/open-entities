import { defineConfig } from "vitest/config";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
  test: {
    environment: "jsdom",
    include: ["src/**/*.{test,spec}.ts"],
    globals: false,
  },
  resolve: {
    alias: {
      "open-entities-wasm": path.resolve(
        __dirname,
        "node_modules/open-entities-wasm/wasm_bindings.js"
      ),
    },
  },
});
