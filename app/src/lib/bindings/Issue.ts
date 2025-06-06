// This file was generated by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Do not edit this file manually.
import type { DelayLoad } from "./DelayLoad";
import type { IssueState } from "./IssueState";
import type { WorkItemId } from "./WorkItemId";

export type Issue = {
  parentId: WorkItemId | null;
  issueType: DelayLoad<string | null>;
  state: DelayLoad<IssueState>;
  subIssues: Array<WorkItemId>;
  trackedIssues: DelayLoad<Array<WorkItemId>>;
  assignees: Array<string>;
};
