import { enUSMessages } from "./catalogs/en-US";

export type MessageKey = keyof typeof enUSMessages;
export type MessageCatalog = Record<MessageKey, string>;

export function isMessageKey(value: string): value is MessageKey {
  return Object.prototype.hasOwnProperty.call(enUSMessages, value);
}

export function formatMessage(
  catalog: Partial<MessageCatalog>,
  key: MessageKey,
): string {
  return catalog[key] ?? enUSMessages[key];
}
