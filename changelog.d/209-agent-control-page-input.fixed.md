Agent-control `type` no longer throws on email, number, date and other inputs
without a text selection, honours a cancelled `keydown`, answers `unsupported`
for file inputs, and treats `contenteditable="false"` as not editable. `drag`
uses the page's `DataTransfer`, and logging an unprintable value no longer
throws into the page's `console`.
