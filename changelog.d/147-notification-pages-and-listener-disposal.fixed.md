`@inflatable-cookie/longhorn` notification pagination no longer applies a page
that was answered against older state. A page that resolves after a newer
update, a stop or a restart is discarded, and a page from a moved ledger
resyncs the first page instead of replacing it with a slice. The controller
now exposes `pagination` (`idle`, `loading` or `failed`) so a renderer can show
pending and failed page loads.
