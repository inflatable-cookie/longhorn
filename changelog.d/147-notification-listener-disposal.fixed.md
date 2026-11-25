Command listener registration and disposal survive synchronous throws. A
listener that throws no longer strands the registrations that came before it,
a throwing unlisten no longer skips the others, and a registration failure keeps
its original error.
