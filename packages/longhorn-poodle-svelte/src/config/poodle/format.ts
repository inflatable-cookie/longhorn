/**
 * Archive timestamps arrive as ISO 8601 text. They are shown in the host's
 * locale, or in the one passed in, so a host can localise them. A value that
 * does not parse is shown as received rather than as "Invalid Date".
 */
export function formatArchiveTimestamp(value: string, locale?: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return value;
  return new Intl.DateTimeFormat(locale, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(date);
}
