export type Data = {
  workItems: WorkItems;
  rootNodes: Node[];
};

export type Node = {
  level: number;
  id: string;
  data: WorkItemNode | GroupNode;
  hasChildren: boolean
};

export type WorkItemNode = { type: "workItem" };
export type GroupNode = { type: "group"; name: string };

export type WorkItemId = string;
export type WorkItems = { [id: WorkItemId]: WorkItem };

export type WorkItem = {
  id: WorkItemId;
  title: string;
  resourcePath: string;
  data: WorkItemData;
  project_item: ProjectItem;
};

export type ProjectItem = {
  epic?: SingleSelectValue
};

export type SingleSelectValue = {
  optionId: string,
  name: string
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
