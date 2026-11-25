`ConfigWindowPlacementSink`'s `WindowPlacementSink` implementation requires
the consumer domain to be `'static`. Migration: an owned domain type already
satisfies this; a domain that borrows must own its data instead.
