export type Data = {
  workItems: WorkItems;
  rootNodes: Node[];
};

export type Node = { data: WorkItemNode | GroupNode; children: Node[] };

export type WorkItemNode = { type: "workItem"; id: string };
export type GroupNode = { type: "group"; name: string; id: string };

export type WorkItemId = string;
export type WorkItems = { [id: WorkItemId]: WorkItem };

export type WorkItem = {
  id: WorkItemId;
  title: string;
  resourcePath: string;
  data: WorkItemData;
};

export type WorkItemData = IssueData | PullRequestData | DraftIssue;

export type IssueData = {
  type: "issue";
  state: IssueState;
  subIssues: WorkItemId[];
  trackedIssues: WorkItemId[];
};
export type IssueState = "OPEN" | "CLOSED" | { Other: string };

export type PullRequestData = { type: "pullRequest"; state: PullRequestState };
export type PullRequestState = "CLOSED" | "MERGED" | "OPEN" | { Other: string };

export type DraftIssue = { type: "draftIssue" };
