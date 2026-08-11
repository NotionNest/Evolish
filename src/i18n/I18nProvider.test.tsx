import { render, screen } from "@testing-library/react";

import { I18nProvider } from "./I18nProvider";
import { formatMessage } from "./messages";
import { useI18n } from "./useI18n";

function MessageProbe() {
  const { locale, direction, t } = useI18n();

  return (
    <p>
      {locale}|{direction}|{t("startup.loading")}
    </p>
  );
}

describe("I18nProvider", () => {
  it("provides translated messages and synchronizes document language metadata", () => {
    render(
      <I18nProvider locale="zh-CN">
        <MessageProbe />
      </I18nProvider>,
    );

    expect(screen.getByText("zh-CN|ltr|正在启动应用核心…")).toBeInTheDocument();
    expect(document.documentElement).toHaveAttribute("lang", "zh-CN");
    expect(document.documentElement).toHaveAttribute("dir", "ltr");
  });

  it("falls back to the English catalog when a localized key is absent", () => {
    expect(formatMessage({}, "startup.loading")).toBe(
      "Starting application core…",
    );
  });
});
