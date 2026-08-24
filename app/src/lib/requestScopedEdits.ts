export class RequestScopedEdits<K, V> {
  private activeRequest: number | null = null;
  private edits = new Map<K, V>();

  begin(request: number): void {
    this.activeRequest = request;
    this.edits.clear();
  }

  set(request: number, key: K, value: V): void {
    if (request === this.activeRequest) {
      this.edits.set(key, value);
    }
  }

  take(request: number): ReadonlyMap<K, V> {
    if (request !== this.activeRequest) return new Map();
    const edits = new Map(this.edits);
    this.activeRequest = null;
    this.edits.clear();
    return edits;
  }

  cancel(request: number): void {
    if (request === this.activeRequest) {
      this.activeRequest = null;
      this.edits.clear();
    }
  }
}
