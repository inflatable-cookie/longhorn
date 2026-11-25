Isolated-window host destruction retires the generation before bounded owner
teardown and keeps an unresolved owner available for retry. Failed isolated
owners stop admitting runtime events. Isolated-window and child-view callbacks
report `ForeignIsland` for a mismatched island.
