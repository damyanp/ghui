export function isDirectAuthSwitch(
  event: Pick<MouseEvent, "ctrlKey">,
): boolean {
  return event.ctrlKey;
}
