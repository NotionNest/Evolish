import { fireEvent, render, screen } from "@testing-library/react";
import { afterEach, vi } from "vitest";

import { I18nProvider } from "../i18n/I18nProvider";
import { AppErrorBoundary } from "./AppErrorBoundary";

function BrokenView(): never {
  throw new Error("render failed");
}

afterEach(() => {
  vi.restoreAllMocks();
});

describe("AppErrorBoundary", () => {
  it("contains render failures and offers a localized recovery action", () => {
    const reload = vi.fn();
    vi.spyOn(console, "error").mockImplementation(() => undefined);

    render(
      <I18nProvider locale="zh-CN">
        <AppErrorBoundary onReload={reload}>
          <BrokenView />
        </AppErrorBoundary>
      </I18nProvider>,
    );

    expect(
      screen.getByRole("heading", { name: "Evolish 遇到了意外错误" }),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "重新加载应用" }));
    expect(reload).toHaveBeenCalledOnce();
  });
});
