import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from '@tailwindcss/vite';

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [sveltekit(), tailwindcss()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },

  build: {
    // The monaco-editor entry chunk and its language workers are inherently
    // large (the TypeScript worker alone is ~7 MB). Bumping the threshold to
    // 5 MB silences the noise while still flagging any new code that grows
    // unexpectedly large.
    chunkSizeWarningLimit: 5000,
    rollupOptions: {
      onLog(level, log, defaultHandler) {
        if (level !== "warn") {
          defaultHandler(level, log);
          return;
        }
        // Known false positives we deliberately allow:
        //
        // - UNUSED_EXTERNAL_IMPORT: only fires in the SSR build for symbols
        //   (e.g. `writeText` from the Tauri clipboard plugin and the
        //   monaco-editor loader) that are referenced inside `$effect` or
        //   event handlers stripped during prerendering. They are used in
        //   the client bundle.
        if (log.code === "UNUSED_EXTERNAL_IMPORT") return;
        // - SOURCEMAP_ERROR / INVALID_ANNOTATION: rollup occasionally fails
        //   to map the location of `/* @__PURE__ */` annotations the Svelte
        //   compiler emits for TypeScript `as` casts. Harmless: the
        //   annotation is simply dropped by rollup.
        if (log.code === "SOURCEMAP_ERROR") return;
        if (log.code === "INVALID_ANNOTATION") return;
        // - CIRCULAR_DEPENDENCY entirely inside third-party packages
        //   (e.g. svelte's internal runtime) is out of our control.
        if (
          log.code === "CIRCULAR_DEPENDENCY" &&
          log.ids?.every((id) => id.includes("node_modules"))
        )
          return;

        // Everything else is escalated to a hard error so the build fails
        // on any new warning.
        const message = log.code
          ? `[rollup:${log.code}] ${log.message}`
          : `[rollup] ${log.message}`;
        throw new Error(message);
      },
    },
  },
}));
