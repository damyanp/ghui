import type { Axis } from "./bindings/Axis";
import type { PivotConfig } from "./bindings/PivotConfig";
import type { PivotField } from "./bindings/PivotField";

const FIELD_LABELS: Record<PivotField, string> = {
  status: "Status",
  blocked: "Blocked",
  epic: "Epic",
  iteration: "Iteration",
  kind: "Kind",
  workstream: "Workstream",
  estimate: "Estimate",
  priority: "Priority",
  assignee: "Assignee",
  repository: "Repository",
  issueType: "IssueType",
  type: "Type",
  state: "State",
};

const FIELD_ALIASES: Readonly<Record<string, PivotField>> = {
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

export type RecipeBarToggle = "explodeMulti" | "showGhostAncestors";

export function parse(text: string): Array<Axis> {
  if (!text.trim()) {
    return [];
  }

  const tokens = text
    .replace(/→|->|>|,/g, "|")
    .split("|")
    .map((token) => token.trim())
    .filter(Boolean);

  const axes: Array<Axis> = [];
  for (const token of tokens) {
    const match = /^([A-Za-z]+)\s*(?:\(\s*([A-Za-z_ ]+)\s*\))?$/.exec(token);
    if (!match) {
      throw new Error(`Could not parse axis: ${JSON.stringify(token)}`);
    }

    const kind = match[1].toLowerCase();
    const arg = match[2]?.trim() ?? null;

    if (kind === "hierarchy") {
      if (arg) {
        throw new Error("Hierarchy takes no argument");
      }
      axes.push({ kind: "hierarchy" });
      continue;
    }

    if (!["pivot", "group", "sort"].includes(kind)) {
      throw new Error(
        `Unknown axis: ${token} (use Pivot, Group, Sort, or Hierarchy)`
      );
    }

    if (!arg) {
      throw new Error(
        `${kind} requires a field argument, e.g. ${kind}(Epic)`
      );
    }

    const field = resolveField(arg);
    if (!field) {
      throw new Error(`Unknown field: ${arg}`);
    }

    axes.push(
      kind === "pivot"
        ? { kind: "pivot", field }
        : kind === "group"
          ? { kind: "group", field }
          : { kind: "sort", field }
    );
  }

  return axes;
}

export function format(recipe: Array<Axis>): string {
  return recipe
    .map((axis) => {
      if (axis.kind === "hierarchy") {
        return "Hierarchy";
      }

      return `${capitalize(axis.kind)}(${FIELD_LABELS[axis.field]})`;
    })
    .join(" → ");
}

export function applyRecipeText(config: PivotConfig, text: string): PivotConfig {
  return { ...config, recipe: parse(text) };
}

export function applyPreset(config: PivotConfig, recipe: string): PivotConfig {
  return applyRecipeText(config, recipe);
}

export function setToggle(
  config: PivotConfig,
  toggle: RecipeBarToggle,
  checked: boolean
): PivotConfig {
  switch (toggle) {
    case "explodeMulti":
      return {
        ...config,
        multiValueStrategy: checked ? "explode" : "combined",
      };
    case "showGhostAncestors":
      return {
        ...config,
        showGhostAncestors: checked,
      };
  }
}

function resolveField(name: string): PivotField | null {
  const key = name
    .replaceAll(/\s+/g, "")
    .toLowerCase();

  return FIELD_ALIASES[key] ?? null;
}

function capitalize(value: "pivot" | "group" | "sort"): string {
  return value[0].toUpperCase() + value.slice(1);
}
