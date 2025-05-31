import type { Attachment } from "svelte/attachments";

export function onFirstVisible(
  rowId: string,
  callback: undefined | ((rowId: string) => void)
): Attachment<HTMLElement> {
  return (node: HTMLElement) => {
    if (!callback) return;

    let hasBeenVisible = false;

    const observer = new IntersectionObserver(
      (entries) => {
        if (!hasBeenVisible && entries[0].isIntersecting) {
          hasBeenVisible = true;
          callback(rowId);
          observer.disconnect();
        }
      },
      { threshold: 0.01 }
    );

    observer.observe(node);

    return () => observer.disconnect();
  };
}
