import { describe, expect, it } from "vitest";
import { RequestSerial } from "./requestSerial";

describe("RequestSerial", () => {
  it("marks only the latest request as current", () => {
    const serial = new RequestSerial();
    const first = serial.start();
    const second = serial.start();

    expect(serial.isCurrent(first)).toBe(false);
    expect(serial.isCurrent(second)).toBe(true);
  });

  it("invalidates an outstanding request", () => {
    const serial = new RequestSerial();
    const request = serial.start();

    serial.invalidate();

    expect(serial.isCurrent(request)).toBe(false);
  });
});
