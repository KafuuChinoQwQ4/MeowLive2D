import { describe, expect, it } from "vitest";
import { shouldUseNativeGlass } from "./nativeAppearance";

describe("shouldUseNativeGlass", () => {
  it("keeps the native glass layer disabled on Linux", () => {
    expect(shouldUseNativeGlass(true, "Linux x86_64")).toBe(false);
  });

  it("keeps native glass enabled on Windows desktop", () => {
    expect(shouldUseNativeGlass(true, "Windows")).toBe(true);
  });

  it("does not enable native glass in a browser", () => {
    expect(shouldUseNativeGlass(false, "Windows")).toBe(false);
  });
});
