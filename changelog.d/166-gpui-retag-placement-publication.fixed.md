A GPUI retag publishes the old identity's staged placements first, and is
refused if that publication does not succeed. It then moves the window's
pending capture and flush work and its apply evidence to the new identity, so
the window's own resizes stay attributed to the apply after the retag.
