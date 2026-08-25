export class SerializedTaskQueue {
  private tail: Promise<void> = Promise.resolve();

  enqueue(task: () => Promise<void>, onError: (error: unknown) => void): void {
    this.tail = this.tail.then(task).catch((error) => {
      onError(error);
    });
  }

  async flush(): Promise<void> {
    await this.tail;
  }
}
