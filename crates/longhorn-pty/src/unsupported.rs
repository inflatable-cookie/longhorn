use std::sync::mpsc::Receiver;

use crate::api::{CommandSpec, Failure, PtyEvent, PtySession, PtySize};

pub(crate) fn open(_: CommandSpec) -> Result<(PtySession, Receiver<PtyEvent>), Failure> {
    Err(Failure::Unsupported)
}

pub(crate) fn write(_: &PtySession, _: &[u8]) -> Result<(), Failure> {
    Err(Failure::Unsupported)
}

pub(crate) fn resize(_: &PtySession, _: PtySize) -> Result<(), Failure> {
    Err(Failure::Unsupported)
}

pub(crate) fn close(_: &PtySession) -> Result<(), Failure> {
    Err(Failure::Unsupported)
}
