import { useEffect, useState } from "react";

import {
  getAppBootstrap,
  type AppBootstrap,
} from "../bridge/appBootstrap";
import "./app.css";

type LoadBootstrap = () => Promise<AppBootstrap>;

type AppProps = {
  loadBootstrap?: LoadBootstrap;
};

type BootstrapState =
  | { status: "loading" }
  | { status: "ready"; data: AppBootstrap }
  | { status: "error"; message: string };

export function App({ loadBootstrap = getAppBootstrap }: AppProps) {
  const [bootstrap, setBootstrap] = useState<BootstrapState>({
    status: "loading",
  });

  useEffect(() => {
    let isActive = true;

    loadBootstrap()
      .then((data) => {
        if (isActive) {
          setBootstrap({ status: "ready", data });
        }
      })
      .catch((error: unknown) => {
        if (isActive) {
          const message =
            error instanceof Error ? error.message : "Unknown startup error";
          setBootstrap({ status: "error", message });
        }
      });

    return () => {
      isActive = false;
    };
  }, [loadBootstrap]);

  return (
    <main className="app-shell">
      <section className="startup-panel" aria-labelledby="app-title">
        <p className="eyebrow">Desktop workspace</p>
        <h1 id="app-title">Evolish</h1>
        <p className="purpose">
          Translation, context capture, and learning in one focused workspace.
        </p>

        <div className="runtime-status" aria-live="polite">
          {bootstrap.status === "loading" && <p>Starting application core…</p>}
          {bootstrap.status === "error" && (
            <p className="error-message">
              Application core unavailable: {bootstrap.message}
            </p>
          )}
          {bootstrap.status === "ready" && (
            <dl>
              <div>
                <dt>Core</dt>
                <dd>Connected</dd>
              </div>
              <div>
                <dt>Version</dt>
                <dd>{bootstrap.data.version}</dd>
              </div>
              <div>
                <dt>Platform</dt>
                <dd>
                  {bootstrap.data.platform} · {bootstrap.data.architecture}
                </dd>
              </div>
            </dl>
          )}
        </div>
      </section>
    </main>
  );
}
