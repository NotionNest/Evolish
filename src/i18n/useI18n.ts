import { createContext, useContext } from "react";

import type {
  SupportedLocale,
  TextDirection,
} from "./locale";
import type { MessageKey } from "./messages";

export type I18nContextValue = {
  locale: SupportedLocale;
  direction: TextDirection;
  t: (key: MessageKey) => string;
  formatNumber: (
    value: number,
    options?: Intl.NumberFormatOptions,
  ) => string;
  formatDate: (
    value: Date | number,
    options?: Intl.DateTimeFormatOptions,
  ) => string;
};

export const I18nContext = createContext<I18nContextValue | null>(null);

export function useI18n(): I18nContextValue {
  const context = useContext(I18nContext);

  if (!context) {
    throw new Error("useI18n must be used inside I18nProvider");
  }

  return context;
}
