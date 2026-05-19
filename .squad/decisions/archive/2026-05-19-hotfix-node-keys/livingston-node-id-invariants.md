### 2025-11-26: Structural Node.id uniqueness invariant for recipe builder
**By:** Livingston (Tester), at Damyan's request
**What:** Added a structural-invariant test harness to `ghui-app/src/nodes/recipe_builder.rs` that catches the `each_key_duplicate` runtime Svelte error class at the Rust layer (where the node tree is built), instead of letting it surface in `TreeTable.svelte`. Two pieces:
1. **Augmented** `test_recipe_builder_preset_snapshots`: per-preset assertion that `nodes.iter().map(|n| &n.id).collect::<HashSet<_>>().len() == nodes.len()`, runs BEFORE the snapshot string compare, with a failure message naming the preset and listing colliding ids. Also added `total=N unique_ids=M` header line to each preset block in the snapshot so duplicates are visually obvious in diffs.
2. **New** `test_recipe_builder_node_id_uniqueness_invariant`: matrix test covering every preset × `show_ghost_ancestors∈{true,false}` × `multi_value_strategy∈{Combined,Explode}`. Asserts three invariants per combination: Node.id uniqueness, level monotonicity (consecutive nodes never jump down by more than 1 level), and WorkItem id presence in `work_items`. Failure messages include preset, ghost flag, strategy.

Also refactored `render_recipe_nodes` into `build_recipe_nodes` (returns `Vec<Node>`) + `format_nodes_string` (`&[Node] -> String`), keeping the existing combined helper as a thin wrapper so the other 3 recipe-builder tests remain unchanged.

**Why:** The existing snapshot test had locked the bug in as "expected" output — the literal at lines 752-762 of `recipe_builder.rs` recorded `item 4` appearing twice in `Pivot(Epic) → Hierarchy` with the same `Node.id` ("4"), once as ghost in `EpicB` and once as real in `epic=(none)`. The pretty-printer showed the duplicate, but `assert_eq!(actual, "<giant literal>")` is too opaque for humans to spot it on review. Same pattern existed in `Pivot(Iteration) → Hierarchy` and `Pivot(Repository) → Group(Epic) → Hierarchy`. The matrix test ALSO surfaced a previously-invisible case: `Pivot(Assignee) → Group(Epic)` with `ghost=false strategy=Explode` produces duplicate id "4" when an item is assigned to multiple users (item 4 is rendered once in each assignee bucket).

**Convention going forward:** Snapshot tests over Vec<Node> output MUST be paired with structural assertions (uniqueness, monotonicity, id presence). Opaque string compares are insufficient — they reward "render exactly what you currently render" but cannot catch invariant violations the eye misses.

**Merge dependency:** This PR's tests are FAILING on current main by design. They go green only after `fix/duplicate-node-keys` (Linus) merges first. PR https://github.com/damyanp/ghui/pull/79 must merge AFTER that fix.

**Open question for Linus:** the matrix test surfaced one bug case (`Pivot(Assignee) → Group(Epic)` ghost=false Explode) that may or may not be covered by his ghost-dedup fix. If his fix doesn't address Explode-strategy duplicates, the matrix test will still fail after his rebase and either (a) Linus expands his fix or (b) we open a follow-up issue. Either way, the invariant test correctly documents that the bug class includes the Explode case.

**Files touched in this PR:** `ghui-app/src/nodes/recipe_builder.rs` only (Rust test-side change, no frontend, no production code).
