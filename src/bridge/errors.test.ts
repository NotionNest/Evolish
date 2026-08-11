import { describe, expect, it } from "vitest";

import type { AppErrorDto } from "./generated/AppErrorDto";
import { normalizeIpcError } from "./errors";

const providerError: AppErrorDto = {
  code: "provider.rate_limited",
  messageKey: "error.unexpected",
  retryable: true,
  suggestedAction: "retry",
  source: "provider:example",
  diagnosticDetail: null,
};

describe("normalizeIpcError", () => {
  it("preserves a valid typed object rejection", () => {
    expect(normalizeIpcError(providerError)).toEqual(providerError);
  });

  it("decodes a serialized typed rejection", () => {
    expect(normalizeIpcError(JSON.stringify(providerError))).toEqual(
      providerError,
    );
  });

  it("classifies raw strings and Error instances as an unexpected IPC error", () => {
    expect(normalizeIpcError("IPC unavailable")).toEqual({
      code: "ipc.unexpected",
      messageKey: "error.ipc.unavailable",
      retryable: true,
      suggestedAction: "retry",
      source: "ipc",
      diagnosticDetail: "IPC unavailable",
    });

    expect(normalizeIpcError(new Error("transport closed"))).toMatchObject({
      code: "ipc.unexpected",
      messageKey: "error.ipc.unavailable",
      diagnosticDetail: "transport closed",
    });
  });

  it("uses a safe generic fallback for malformed and unknown values", () => {
    expect(normalizeIpcError({ code: 42 })).toEqual({
      code: "ipc.unexpected",
      messageKey: "error.ipc.unavailable",
      retryable: true,
      suggestedAction: "retry",
      source: "ipc",
      diagnosticDetail: null,
    });
    expect(normalizeIpcError(Symbol("failure"))).toMatchObject({
      code: "ipc.unexpected",
      diagnosticDetail: null,
    });
  });
});
