import { render, screen } from "@testing-library/react";

import { App } from "./App";

describe("App", () => {
  it("shows the Rust application core metadata after startup", async () => {
    render(
      <App
        loadBootstrap={() =>
          Promise.resolve({
            name: "Evolish",
            version: "0.1.0",
            platform: "macOS",
            architecture: "aarch64",
          })
        }
      />,
    );

    expect(screen.getByText("Starting application core…")).toBeInTheDocument();
    expect(await screen.findByText("Connected")).toBeInTheDocument();
    expect(screen.getByText("macOS · aarch64")).toBeInTheDocument();
  });

  it("surfaces an application core startup failure", async () => {
    render(
      <App
        loadBootstrap={() => Promise.reject(new Error("IPC unavailable"))}
      />,
    );

    expect(
      await screen.findByText(
        "Application core unavailable: IPC unavailable",
      ),
    ).toBeInTheDocument();
  });
});
