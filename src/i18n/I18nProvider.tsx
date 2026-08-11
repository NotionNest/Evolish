import { type ReactNode, useEffect, useMemo } from "react";

import { enUSMessages } from "./catalogs/en-US";
import { zhCNMessages } from "./catalogs/zh-CN";
import {
  defaultLocale,
  localeMetadata,
  resolveLocale,
  type SupportedLocale,
} from "./locale";
import { formatMessage, type MessageCatalog } from "./messages";
import { I18nContext, type I18nContextValue } from "./useI18n";

const catalogs: Record<SupportedLocale, MessageCatalog> = {
  "en-US": enUSMessages,
  "zh-CN": zhCNMessages,
};

type I18nProviderProps = {
  children: ReactNode;
  locale?: SupportedLocale;
};

function detectLocale(): SupportedLocale {
  if (typeof navigator === "undefined") {
    return defaultLocale;
  }

  return resolveLocale(navigator.languages);
}

export function I18nProvider({ children, locale }: I18nProviderProps) {
  const activeLocale = locale ?? detectLocale();
  const direction = localeMetadata[activeLocale].direction;

  useEffect(() => {
    const root = document.documentElement;
    const previousLanguage = root.lang;
    const previousDirection = root.dir;

    root.lang = activeLocale;
    root.dir = direction;

    return () => {
      root.lang = previousLanguage;
      root.dir = previousDirection;
    };
  }, [activeLocale, direction]);

  const value = useMemo<I18nContextValue>(
    () => ({
      locale: activeLocale,
      direction,
      t: (key) => formatMessage(catalogs[activeLocale], key),
      formatNumber: (number, options) =>
        new Intl.NumberFormat(activeLocale, options).format(number),
      formatDate: (date, options) =>
        new Intl.DateTimeFormat(activeLocale, options).format(date),
    }),
    [activeLocale, direction],
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}
