import { describe, expect, it } from "vitest";
import { shouldUseWrappedSignature } from "../../src/model/signature";

describe("signature popover planning", () => {
  it("shouldUseWrappedSignature uses strict > clientWidth + 1 by default", () => {
    expect(shouldUseWrappedSignature({ scrollWidth: 101, clientWidth: 100 })).toBe(false);
    expect(shouldUseWrappedSignature({ scrollWidth: 102, clientWidth: 100 })).toBe(true);
  });
});
