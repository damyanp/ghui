use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum PivotField {
    Status,
    Blocked,
    Epic,
    Iteration,
    Kind,
    Workstream,
    Estimate,
    Priority,
    Assignee,
    Repository,
    IssueType,
    Type,
    State,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "kind", content = "field")]
#[ts(export)]
pub enum Axis {
    Pivot(PivotField),
    Group(PivotField),
    Hierarchy,
    Sort(PivotField),
}

#[derive(Default, Serialize, Deserialize, TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum MultiValueStrategy {
    #[default]
    Combined,
    Explode,
}

#[derive(Serialize, Deserialize, TS, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct PivotConfig {
    pub recipe: Vec<Axis>,
    pub multi_value_strategy: MultiValueStrategy,
    pub show_ghost_ancestors: bool,
}

impl Default for PivotConfig {
    fn default() -> Self {
        Self {
            recipe: vec![Axis::Pivot(PivotField::Epic), Axis::Hierarchy],
            multi_value_strategy: MultiValueStrategy::Combined,
            show_ghost_ancestors: true,
        }
    }
}

pub fn parse_recipe(text: &str) -> Result<Vec<Axis>> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }

    let normalised = normalize_recipe_separators(text);

    let tokens = normalised
        .split('|')
        .map(str::trim)
        .filter(|token| !token.is_empty());

    let mut axes = Vec::new();
    for token in tokens {
        let (kind, arg) = parse_axis_token(token)?;
        let kind = kind.to_ascii_lowercase();

        if kind == "hierarchy" {
            if arg.is_some() {
                bail!("Hierarchy takes no argument");
            }
            axes.push(Axis::Hierarchy);
            continue;
        }

        if !matches!(kind.as_str(), "pivot" | "group" | "sort") {
            bail!("Unknown axis: {token} (use Pivot, Group, Sort, or Hierarchy)");
        }

        let Some(arg) = arg else {
            bail!("{kind} requires a field argument, e.g. {kind}(Epic)");
        };

        let Some(field) = resolve_field(arg) else {
            bail!("Unknown field: {arg}");
        };

        axes.push(match kind.as_str() {
            "pivot" => Axis::Pivot(field),
            "group" => Axis::Group(field),
            "sort" => Axis::Sort(field),
            _ => unreachable!(),
        });
    }

    Ok(axes)
}

pub fn recipe_to_string(recipe: &[Axis]) -> String {
    recipe
        .iter()
        .map(|axis| match axis {
            Axis::Hierarchy => "Hierarchy".to_string(),
            Axis::Pivot(field) => format!("Pivot({})", field_label(field)),
            Axis::Group(field) => format!("Group({})", field_label(field)),
            Axis::Sort(field) => format!("Sort({})", field_label(field)),
        })
        .collect::<Vec<_>>()
        .join(" → ")
}

fn parse_axis_token(token: &str) -> Result<(&str, Option<&str>)> {
    if !token
        .chars()
        .all(|c| c.is_ascii_alphabetic() || matches!(c, '_' | ' ' | '(' | ')'))
    {
        bail!("Could not parse axis: {token:?}");
    }

    if let Some(open_paren) = token.find('(') {
        if !token.ends_with(')') {
            bail!("Could not parse axis: {token:?}");
        }
        let close_paren = token.len() - 1;
        if open_paren >= close_paren {
            bail!("Could not parse axis: {token:?}");
        }
        let kind = token[..open_paren].trim();
        let arg = token[(open_paren + 1)..close_paren].trim();
        if kind.is_empty() || arg.is_empty() || arg.contains('(') || arg.contains(')') {
            bail!("Could not parse axis: {token:?}");
        }
        if !kind.chars().all(|c| c.is_ascii_alphabetic()) {
            bail!("Could not parse axis: {token:?}");
        }
        return Ok((kind, Some(arg)));
    }

    if !token
        .chars()
        .all(|c| c.is_ascii_alphabetic() || c.is_whitespace())
    {
        bail!("Could not parse axis: {token:?}");
    }

    let kind = token.trim();
    if kind.is_empty() {
        bail!("Could not parse axis: {token:?}");
    }
    Ok((kind, None))
}

fn resolve_field(name: &str) -> Option<PivotField> {
    let key = name
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_lowercase())
        .collect::<String>();

    match key.as_str() {
        "epic" => Some(PivotField::Epic),
        "workstream" | "ws" => Some(PivotField::Workstream),
        "status" => Some(PivotField::Status),
        "iteration" | "sprint" => Some(PivotField::Iteration),
        "kind" => Some(PivotField::Kind),
        "priority" => Some(PivotField::Priority),
        "blocked" => Some(PivotField::Blocked),
        "estimate" => Some(PivotField::Estimate),
        "assignee" | "assignees" | "assigned" | "owner" => Some(PivotField::Assignee),
        "issuetype" | "issue_type" => Some(PivotField::IssueType),
        "state" => Some(PivotField::State),
        "type" => Some(PivotField::Type),
        "repository" | "repo" => Some(PivotField::Repository),
        _ => None,
    }
}

fn normalize_recipe_separators(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '-' if chars.peek() == Some(&'>') => {
                let _ = chars.next();
                out.push('|');
            }
            '→' | '>' | ',' => out.push('|'),
            _ => out.push(ch),
        }
    }

    out
}

fn field_label(field: &PivotField) -> &'static str {
    match field {
        PivotField::Status => "Status",
        PivotField::Blocked => "Blocked",
        PivotField::Epic => "Epic",
        PivotField::Iteration => "Iteration",
        PivotField::Kind => "Kind",
        PivotField::Workstream => "Workstream",
        PivotField::Estimate => "Estimate",
        PivotField::Priority => "Priority",
        PivotField::Assignee => "Assignee",
        PivotField::Repository => "Repository",
        PivotField::IssueType => "IssueType",
        PivotField::Type => "Type",
        PivotField::State => "State",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    const PRESETS: [&str; 14] = [
        "Pivot(Epic) → Hierarchy",
        "Hierarchy → Group(Workstream)",
        "Hierarchy → Group(Status)",
        "Pivot(Status)",
        "Pivot(Status) → Group(Workstream)",
        "Pivot(Assignee) → Group(Epic)",
        "Pivot(Iteration) → Hierarchy",
        "Pivot(Repository) → Group(Epic) → Hierarchy",
        "Pivot(IssueType) → Group(Status)",
        "Pivot(State) → Group(Epic)",
        "Hierarchy",
        "Sort(Priority) → Hierarchy",
        "Sort(Epic)",
        "Pivot(Type) → Group(Status)",
    ];

    #[test]
    fn test_parse_recipe_presets_round_trip() {
        for preset in PRESETS {
            let parsed = parse_recipe(preset).unwrap();
            assert_eq!(recipe_to_string(&parsed), preset);
        }
    }

    #[test]
    fn test_parse_recipe_unknown_field() {
        let error = parse_recipe("Pivot(NotAField)").unwrap_err();
        assert_eq!(error.to_string(), "Unknown field: NotAField");
    }

    #[test]
    fn test_parse_recipe_unknown_axis() {
        let error = parse_recipe("Bucket(Epic)").unwrap_err();
        assert_eq!(
            error.to_string(),
            "Unknown axis: Bucket(Epic) (use Pivot, Group, Sort, or Hierarchy)"
        );
    }

    #[test]
    fn test_parse_recipe_missing_parens() {
        let error = parse_recipe("Pivot(Epic").unwrap_err();
        assert_eq!(error.to_string(), "Could not parse axis: \"Pivot(Epic\"");
    }

    #[test]
    fn test_parse_recipe_dangling_separator_is_ignored() {
        let parsed = parse_recipe("Pivot(Epic) ->").unwrap();
        assert_eq!(parsed, vec![Axis::Pivot(PivotField::Epic)]);
    }

    #[test]
    fn test_parse_recipe_field_aliases() {
        let parsed = parse_recipe("Pivot(Repo) -> Group(Owner) -> Sort(Assignees)").unwrap();
        assert_eq!(
            parsed,
            vec![
                Axis::Pivot(PivotField::Repository),
                Axis::Group(PivotField::Assignee),
                Axis::Sort(PivotField::Assignee),
            ]
        );
    }

    #[test]
    fn test_recipes_fixture_round_trip() {
        let recipes: BTreeMap<String, Vec<Axis>> =
            serde_json::from_str(include_str!("../tests/fixtures/recipes.json")).unwrap();

        for (recipe, expected_axes) in recipes {
            let parsed = parse_recipe(&recipe).unwrap();
            assert_eq!(parsed, expected_axes);
            assert_eq!(recipe_to_string(&parsed), recipe);
        }
    }

    #[test]
    fn test_recipes_fixture_snapshot() {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../tests/fixtures/recipes.json")).unwrap();
        insta::assert_json_snapshot!(
            fixture,
            @r#"
        {
          "Hierarchy": [
            {
              "kind": "hierarchy"
            }
          ],
          "Hierarchy → Group(Status)": [
            {
              "kind": "hierarchy"
            },
            {
              "field": "status",
              "kind": "group"
            }
          ],
          "Hierarchy → Group(Workstream)": [
            {
              "kind": "hierarchy"
            },
            {
              "field": "workstream",
              "kind": "group"
            }
          ],
          "Pivot(Assignee) → Group(Epic)": [
            {
              "field": "assignee",
              "kind": "pivot"
            },
            {
              "field": "epic",
              "kind": "group"
            }
          ],
          "Pivot(Epic) → Hierarchy": [
            {
              "field": "epic",
              "kind": "pivot"
            },
            {
              "kind": "hierarchy"
            }
          ],
          "Pivot(IssueType) → Group(Status)": [
            {
              "field": "issueType",
              "kind": "pivot"
            },
            {
              "field": "status",
              "kind": "group"
            }
          ],
          "Pivot(Iteration) → Hierarchy": [
            {
              "field": "iteration",
              "kind": "pivot"
            },
            {
              "kind": "hierarchy"
            }
          ],
          "Pivot(Repository) → Group(Epic) → Hierarchy": [
            {
              "field": "repository",
              "kind": "pivot"
            },
            {
              "field": "epic",
              "kind": "group"
            },
            {
              "kind": "hierarchy"
            }
          ],
          "Pivot(State) → Group(Epic)": [
            {
              "field": "state",
              "kind": "pivot"
            },
            {
              "field": "epic",
              "kind": "group"
            }
          ],
          "Pivot(Status)": [
            {
              "field": "status",
              "kind": "pivot"
            }
          ],
          "Pivot(Status) → Group(Workstream)": [
            {
              "field": "status",
              "kind": "pivot"
            },
            {
              "field": "workstream",
              "kind": "group"
            }
          ],
          "Pivot(Type) → Group(Status)": [
            {
              "field": "type",
              "kind": "pivot"
            },
            {
              "field": "status",
              "kind": "group"
            }
          ],
          "Sort(Epic)": [
            {
              "field": "epic",
              "kind": "sort"
            }
          ],
          "Sort(Priority) → Hierarchy": [
            {
              "field": "priority",
              "kind": "sort"
            },
            {
              "kind": "hierarchy"
            }
          ]
        }
        "#
        );
    }
}
