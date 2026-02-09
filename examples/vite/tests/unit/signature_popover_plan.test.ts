import { describe, expect, it } from "vitest";
import {
  computePopoverWidthPx,
  pickPopoverSide,
  shouldUseWrappedSignature,
} from "../../src/model/signature";

describe("signature popover planning", () => {
  it("shouldUseWrappedSignature uses strict > clientWidth + 1 by default", () => {
    expect(shouldUseWrappedSignature({ scrollWidth: 101, clientWidth: 100 })).toBe(false);
    expect(shouldUseWrappedSignature({ scrollWidth: 102, clientWidth: 100 })).toBe(true);
  });

  it("pickPopoverSide chooses left when it can fit", () => {
    const side = pickPopoverSide({
      viewportWidth: 1000,
      wrapLeft: 500,
      wrapRight: 700,
      popoverWidthPx: 240,
    });
    expect(side).toBe("left");
  });

  it("pickPopoverSide chooses right when only right can fit", () => {
    const side = pickPopoverSide({
      viewportWidth: 1000,
      wrapLeft: 100,
      wrapRight: 200,
      popoverWidthPx: 240,
    });
    expect(side).toBe("right");
  });

  it("pickPopoverSide falls back to the larger available side", () => {
    const side = pickPopoverSide({
      viewportWidth: 1000,
      wrapLeft: 100,
      wrapRight: 400,
      popoverWidthPx: 900,
    });
    expect(side).toBe("right");
  });

  it("computePopoverWidthPx clamps to min/max and returns an in-range value otherwise", () => {
    expect(computePopoverWidthPx(500)).toBe(240);
    expect(computePopoverWidthPx(2000)).toBe(360);
    expect(computePopoverWidthPx(1000)).toBe(280);
  });
});
