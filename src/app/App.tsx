import { useEffect, useState } from "react";

import {
  getAppBootstrap,
  type AppBootstrap,
} from "../bridge/appBootstrap";
import { normalizeIpcError } from "../bridge/errors";
import type { AppErrorDto } from "../bridge/generated/AppErrorDto";
import { isMessageKey } from "../i18n/messages";
import { useI18n } from "../i18n/useI18n";
import "./app.css";

type LoadBootstrap = () => Promise<AppBootstrap>;

type AppProps = {
  loadBootstrap?: LoadBootstrap;
};

type BootstrapState =
  | { status: "loading" }
  | { status: "ready"; data: AppBootstrap }
  | { status: "error"; error: AppErrorDto };

export function App({ loadBootstrap = getAppBootstrap }: AppProps) {
  const { t } = useI18n();
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
          setBootstrap({ status: "error", error: normalizeIpcError(error) });
        }
      });

    return () => {
      isActive = false;
    };
  }, [loadBootstrap]);

  return (
    <main className="app-shell">
      <section className="startup-panel" aria-labelledby="app-title">
        <p className="eyebrow">{t("startup.eyebrow")}</p>
        <h1 id="app-title">Evolish</h1>
        <p className="purpose">{t("startup.purpose")}</p>

        <div className="runtime-status" aria-live="polite">
          {bootstrap.status === "loading" && <p>{t("startup.loading")}</p>}
          {bootstrap.status === "error" && (
            <p className="error-message">
              {t(
                isMessageKey(bootstrap.error.messageKey)
                  ? bootstrap.error.messageKey
                  : "error.unexpected",
              )}
            </p>
          )}
          {bootstrap.status === "ready" && (
            <dl>
              <div>
                <dt>{t("runtime.core")}</dt>
                <dd>{t("runtime.connected")}</dd>
              </div>
              <div>
                <dt>{t("runtime.version")}</dt>
                <dd>{bootstrap.data.version}</dd>
              </div>
              <div>
                <dt>{t("runtime.platform")}</dt>
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
