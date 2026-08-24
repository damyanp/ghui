export class ItemUpdateQueue<T> {
  private items = new Set<T>();
  private timer: ReturnType<typeof setTimeout> | null = null;

  constructor(
    private submit: (items: T[]) => Promise<void>,
    private reportError: (error: unknown) => void
  ) {}

  add(item: T): void {
    this.items.add(item);
    if (this.timer === null) {
      this.timer = setTimeout(() => this.flush());
    }
  }

  clear(): void {
    if (this.timer !== null) {
      clearTimeout(this.timer);
      this.timer = null;
    }
    this.items.clear();
  }

  private flush(): void {
    this.timer = null;
    const items = Array.from(this.items);
    this.items.clear();
    void this.submit(items).catch(this.reportError);
  }
}
