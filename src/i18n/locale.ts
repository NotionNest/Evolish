export const supportedLocales = ["zh-CN", "en-US"] as const;

export type SupportedLocale = (typeof supportedLocales)[number];
export type TextDirection = "ltr" | "rtl";

export const defaultLocale: SupportedLocale = "en-US";

export const localeMetadata: Record<
  SupportedLocale,
  { direction: TextDirection }
> = {
  "zh-CN": { direction: "ltr" },
  "en-US": { direction: "ltr" },
};

const supportedByCanonicalName = new Map<string, SupportedLocale>(
  supportedLocales.map((locale) => [locale.toLowerCase(), locale]),
);

export function resolveLocale(
  requestedLocales: readonly string[],
): SupportedLocale {
  for (const requestedLocale of requestedLocales) {
    let canonicalLocale: string | undefined;

    try {
      [canonicalLocale] = Intl.getCanonicalLocales(requestedLocale);
    } catch {
      continue;
    }

    if (!canonicalLocale) {
      continue;
    }

    const exactMatch = supportedByCanonicalName.get(
      canonicalLocale.toLowerCase(),
    );
    if (exactMatch) {
      return exactMatch;
    }

    const language = new Intl.Locale(canonicalLocale).language;
    if (language === "zh") {
      return "zh-CN";
    }
    if (language === "en") {
      return "en-US";
    }
  }

  return defaultLocale;
}
