// Tauri doesn't have a Node.js server to do proper SSR
// so we will use adapter-static to prerender the app (SSG)
// See: https://v2.tauri.app/start/frontend/sveltekit/ for more info
import adapter from "@sveltejs/adapter-static";
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess({ script: true }),
  kit: {
    adapter: adapter({ fallback: "index.html" }),
  },
  // Treat any Svelte compiler warning from our own source as a build error.
  // Warnings from third-party components in node_modules are out of our
  // control and only logged, not promoted to errors. Intentional warnings
  // in our source must be acknowledged with a `<!-- svelte-ignore -->`
  // comment.
  onwarn: (warning, handler) => {
    const filename = warning.filename ?? "";
    if (filename.includes("node_modules")) {
      handler(warning);
      return;
    }
    throw new Error(
      `[svelte:${warning.code ?? "warning"}] ${warning.message}` +
        (filename ? ` (${filename})` : "")
    );
  },
};

export default config;
