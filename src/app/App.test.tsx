import { render, screen } from "@testing-library/react";

import { I18nProvider } from "../i18n/I18nProvider";
import { App } from "./App";

describe("App", () => {
  it("shows the Rust application core metadata after startup", async () => {
    render(
      <I18nProvider locale="en-US">
        <App
          loadBootstrap={() =>
            Promise.resolve({
              name: "Lingua Forge",
              version: "0.1.0",
              platform: "macOS",
              architecture: "aarch64",
            })
          }
        />
      </I18nProvider>,
    );

    expect(screen.getByText("Starting application core…")).toBeInTheDocument();
    expect(
      await screen.findByRole("heading", { name: "Lingua Forge" }),
    ).toBeInTheDocument();
    expect(await screen.findByText("Connected")).toBeInTheDocument();
    expect(screen.getByText("macOS · aarch64")).toBeInTheDocument();
  });

  it("surfaces an application core startup failure", async () => {
    render(
      <I18nProvider locale="zh-CN">
        <App
          loadBootstrap={() => Promise.reject(new Error("IPC unavailable"))}
        />
      </I18nProvider>,
    );

    expect(
      await screen.findByText("无法连接到应用核心。"),
    ).toBeInTheDocument();
  });
});
