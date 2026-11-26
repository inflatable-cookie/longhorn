PTY sessions no longer withhold the child's exit result while a background
child keeps the terminal open and writes. The reader now works in bounded passes
that check the child's state between them, and its post-exit drain reads at most
1 MiB. A read failure is reported as soon as the reader sees it instead of
after the child exits.
