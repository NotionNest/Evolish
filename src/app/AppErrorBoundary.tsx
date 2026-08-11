import { Component, type ReactNode } from "react";

import { useI18n } from "../i18n/useI18n";

type RenderErrorBoundaryProps = {
  children: ReactNode;
  fallback: ReactNode;
};

type RenderErrorBoundaryState = {
  failed: boolean;
};

class RenderErrorBoundary extends Component<
  RenderErrorBoundaryProps,
  RenderErrorBoundaryState
> {
  public state: RenderErrorBoundaryState = { failed: false };

  public static getDerivedStateFromError(): RenderErrorBoundaryState {
    return { failed: true };
  }

  public render() {
    return this.state.failed ? this.props.fallback : this.props.children;
  }
}

type AppErrorBoundaryProps = {
  children: ReactNode;
  onReload?: () => void;
};

export function AppErrorBoundary({
  children,
  onReload,
}: AppErrorBoundaryProps) {
  const { t } = useI18n();
  const reload = onReload ?? (() => window.location.reload());

  const fallback = (
    <main className="app-shell">
      <section className="startup-panel" aria-labelledby="fatal-error-title">
        <h1 id="fatal-error-title">{t("errorBoundary.title")}</h1>
        <p>{t("errorBoundary.description")}</p>
        <button type="button" onClick={reload}>
          {t("errorBoundary.reload")}
        </button>
      </section>
    </main>
  );

  return (
    <RenderErrorBoundary fallback={fallback}>
      {children}
    </RenderErrorBoundary>
  );
}
