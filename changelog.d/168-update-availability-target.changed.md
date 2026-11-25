Update availability adds `unavailableForTarget` for releases without an
artifact for the current target. Handle `UpdateAvailability::UnavailableForTarget`
and `UpdateAvailabilityProjection::UnavailableForTarget` in exhaustive matches;
show the release as unavailable instead of offering install.
