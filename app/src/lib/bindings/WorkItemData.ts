// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { Issue } from "./Issue";
import type { PullRequest } from "./PullRequest";

export type WorkItemData =
  | { "type": "draftIssue" }
  | { "type": "issue" } & Issue
  | { "type": "pullRequest" } & PullRequest;
