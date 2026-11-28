`ForkHistoryController` errors now carry a name and message, a failed
navigation sets `status` to `failed`, and continuation loads reject a result
overtaken by a restart or a newer revision.
