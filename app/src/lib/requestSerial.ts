export class RequestSerial {
  private latest = 0;

  start(): number {
    return ++this.latest;
  }

  isCurrent(request: number): boolean {
    return request === this.latest;
  }

  invalidate(): void {
    this.latest++;
  }
}
