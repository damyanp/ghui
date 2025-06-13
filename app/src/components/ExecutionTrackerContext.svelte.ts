import { getContext, setContext } from "svelte";

const key = Symbol("ExecutionTrackerContext");

export function setExecutionTrackerContext(c: ExecutionTrackerContext) {
  setContext(key, c);
  return c;
}

export function getExecutionTrackerContext() {
  return getContext(key) as ExecutionTrackerContext;
}

export class ExecutionTrackerContext {
  scale = $state(0.0001);
  scrollLeft = $state(0);
}


