import { describe, expect, it } from "vitest";

import { resolveLocale } from "./locale";

describe("resolveLocale", () => {
  it("preserves exact supported locale matches", () => {
    expect(resolveLocale(["zh-CN"])).toBe("zh-CN");
    expect(resolveLocale(["en-US"])).toBe("en-US");
  });

  it("maps Chinese language preferences to the supported Chinese catalog", () => {
    expect(resolveLocale(["zh-TW"])).toBe("zh-CN");
    expect(resolveLocale(["zh"])).toBe("zh-CN");
  });

  it("falls back to English for unsupported or invalid locale input", () => {
    expect(resolveLocale(["fr-FR"])).toBe("en-US");
    expect(resolveLocale(["not_a_locale"])).toBe("en-US");
    expect(resolveLocale([])).toBe("en-US");
  });
});
