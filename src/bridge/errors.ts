import type { AppErrorDto } from "./generated/AppErrorDto";

const unexpectedIpcError: AppErrorDto = {
  code: "ipc.unexpected",
  messageKey: "error.ipc.unavailable",
  retryable: true,
  suggestedAction: "retry",
  source: "ipc",
  diagnosticDetail: null,
};

export class IpcError extends Error {
  public readonly dto: AppErrorDto;

  public constructor(dto: AppErrorDto) {
    super(dto.code);
    this.name = "IpcError";
    this.dto = dto;
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isNullableString(value: unknown): value is string | null {
  return value === null || typeof value === "string";
}

function isAppErrorDto(value: unknown): value is AppErrorDto {
  if (!isRecord(value)) {
    return false;
  }

  return (
    typeof value.code === "string" &&
    value.code.length > 0 &&
    typeof value.messageKey === "string" &&
    value.messageKey.length > 0 &&
    typeof value.retryable === "boolean" &&
    isNullableString(value.suggestedAction) &&
    isNullableString(value.source) &&
    isNullableString(value.diagnosticDetail)
  );
}

function redactSensitiveText(value: string): string {
  return value
    .replace(/Bearer\s+\S+/giu, "Bearer [redacted]")
    .replace(/(api[_ -]?key\s*[:=]\s*)\S+/giu, "$1[redacted]")
    .replace(
      /-----BEGIN [^-]*PRIVATE KEY-----[\s\S]*$/giu,
      "[redacted private key]",
    );
}

function safeDiagnosticDetail(value: string): string | null {
  const printable = [...value]
    .filter((character) => {
      const codePoint = character.codePointAt(0) ?? 0;
      return codePoint >= 32 || character === "\n" || character === "\t";
    })
    .join("")
    .trim();

  if (!printable) {
    return null;
  }

  return redactSensitiveText(printable).slice(0, 500);
}

function withDiagnosticDetail(detail: string | null): AppErrorDto {
  return { ...unexpectedIpcError, diagnosticDetail: detail };
}

export function normalizeIpcError(rejection: unknown): AppErrorDto {
  if (rejection instanceof IpcError) {
    return rejection.dto;
  }

  if (isAppErrorDto(rejection)) {
    return rejection;
  }

  if (typeof rejection === "string") {
    try {
      const decoded: unknown = JSON.parse(rejection);
      if (isAppErrorDto(decoded)) {
        return decoded;
      }
    } catch {
      // The rejection is an ordinary diagnostic string, not a serialized DTO.
    }

    return withDiagnosticDetail(safeDiagnosticDetail(rejection));
  }

  if (rejection instanceof Error) {
    return withDiagnosticDetail(safeDiagnosticDetail(rejection.message));
  }

  return unexpectedIpcError;
}
