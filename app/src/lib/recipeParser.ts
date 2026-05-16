import type { Axis } from "$lib/bindings/Axis";
import type { PivotField } from "$lib/bindings/PivotField";

const FIELD_ALIASES: Record<string, PivotField> = {
  epic: "epic",
  workstream: "workstream",
  ws: "workstream",
  status: "status",
  iteration: "iteration",
  sprint: "iteration",
  kind: "kind",
  priority: "priority",
  blocked: "blocked",
  estimate: "estimate",
  assignee: "assignee",
  assignees: "assignee",
  assigned: "assignee",
  owner: "assignee",
  issuetype: "issueType",
  issue_type: "issueType",
  state: "state",
  type: "type",
  repository: "repository",
  repo: "repository",
};

function isAsciiAlphabetic(char: string): boolean {
  return /^[A-Za-z]$/.test(char);
}

function normalizeRecipeSeparators(text: string): string {
  let out = "";

  for (let i = 0; i < text.length; i++) {
    const char = text[i];
    if (char === "-" && text[i + 1] === ">") {
      out += "|";
      i++;
      continue;
    }
    if (char === "→" || char === ">" || char === ",") {
      out += "|";
      continue;
    }
    out += char;
  }

  return out;
}

function parseAxisToken(token: string): { kind: string; arg: string | null } {
  if (
    ![...token].every(
      (char) =>
        isAsciiAlphabetic(char) || char === "_" || char === " " || char === "(" || char === ")"
    )
  ) {
    throw new Error(`Could not parse axis: ${JSON.stringify(token)}`);
  }

  const openParen = token.indexOf("(");
  if (openParen !== -1) {
    if (!token.endsWith(")")) {
      throw new Error(`Could not parse axis: ${JSON.stringify(token)}`);
    }

    const closeParen = token.length - 1;
    if (openParen >= closeParen) {
      throw new Error(`Could not parse axis: ${JSON.stringify(token)}`);
    }

    const kind = token.slice(0, openParen).trim();
    const arg = token.slice(openParen + 1, closeParen).trim();
    if (
      !kind ||
      !arg ||
      arg.includes("(") ||
      arg.includes(")") ||
      ![...kind].every((char) => isAsciiAlphabetic(char))
    ) {
      throw new Error(`Could not parse axis: ${JSON.stringify(token)}`);
    }

    return { kind, arg };
  }

  if (![...token].every((char) => isAsciiAlphabetic(char) || /\s/.test(char))) {
    throw new Error(`Could not parse axis: ${JSON.stringify(token)}`);
  }

  const kind = token.trim();
  if (!kind) {
    throw new Error(`Could not parse axis: ${JSON.stringify(token)}`);
  }

  return { kind, arg: null };
}

function resolveField(name: string): PivotField | null {
  const key = [...name]
    .filter((char) => !/\s/.test(char))
    .join("")
    .toLowerCase();

  return FIELD_ALIASES[key] ?? null;
}

function fieldLabel(field: PivotField): string {
  switch (field) {
    case "status":
      return "Status";
    case "blocked":
      return "Blocked";
    case "epic":
      return "Epic";
    case "iteration":
      return "Iteration";
    case "kind":
      return "Kind";
    case "workstream":
      return "Workstream";
    case "estimate":
      return "Estimate";
    case "priority":
      return "Priority";
    case "assignee":
      return "Assignee";
    case "repository":
      return "Repository";
    case "issueType":
      return "IssueType";
    case "type":
      return "Type";
    case "state":
      return "State";
  }
}

export function parseRecipe(
  text: string
): { ok: true; recipe: Axis[] } | { ok: false; error: string } {
  try {
    if (!text.trim()) {
      return { ok: true, recipe: [] };
    }

    const normalized = normalizeRecipeSeparators(text);
    const tokens = normalized
      .split("|")
      .map((token) => token.trim())
      .filter((token) => token.length > 0);

    const axes: Axis[] = [];
    for (const token of tokens) {
      const { kind, arg } = parseAxisToken(token);
      const lowerKind = kind.toLowerCase();

      if (lowerKind === "hierarchy") {
        if (arg !== null) {
          throw new Error("Hierarchy takes no argument");
        }
        axes.push({ kind: "hierarchy" });
        continue;
      }

      if (lowerKind !== "pivot" && lowerKind !== "group" && lowerKind !== "sort") {
        throw new Error(`Unknown axis: ${token} (use Pivot, Group, Sort, or Hierarchy)`);
      }

      if (arg === null) {
        throw new Error(`${lowerKind} requires a field argument, e.g. ${lowerKind}(Epic)`);
      }

      const field = resolveField(arg);
      if (field === null) {
        throw new Error(`Unknown field: ${arg}`);
      }

      axes.push({ kind: lowerKind, field } as Axis);
    }

    return { ok: true, recipe: axes };
  } catch (error) {
    return {
      ok: false,
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

export function recipeToString(recipe: Axis[]): string {
  return recipe
    .map((axis) => {
      if (axis.kind === "hierarchy") {
        return "Hierarchy";
      }
      const kind = axis.kind[0].toUpperCase() + axis.kind.slice(1);
      return `${kind}(${fieldLabel(axis.field)})`;
    })
    .join(" → ");
}
