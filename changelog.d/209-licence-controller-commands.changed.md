`LicenceController` commands now need a started controller: outside a started
lifetime they send nothing and resolve `failed` with
`LicenceControllerUnavailableError`. Call `start()` before commands.
