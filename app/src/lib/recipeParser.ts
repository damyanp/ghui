import { invoke } from "@tauri-apps/api/core";
import type { Axis } from "$lib/bindings/Axis";

export async function parseRecipe(
  text: string
): Promise<{ ok: true; recipe: Axis[] } | { ok: false; error: string }> {
  try {
    const recipe = await invoke<Axis[]>("parse_recipe", { text });
    return { ok: true, recipe };
  } catch (error) {
    return {
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

export async function recipeToString(recipe: Axis[]): Promise<string> {
  return invoke<string>("recipe_to_string", { recipe });
}
