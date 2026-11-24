`AgentControlHandle::shutdown` now always joins the server thread, so a startup
failure such as a port already in use is reported as the original
`AgentControlShutdownError::Serve` error. It previously returned
`AlreadyShutdown` once the thread had exited, and the failure was lost.
