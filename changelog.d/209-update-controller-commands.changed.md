`UpdateController` commands now need a started controller: outside a started
lifetime they send nothing and resolve `failed` with
`UpdateControllerUnavailableError`. `apply` with a version that is not staged
resolves `failed` with `UpdateNotStagedError` instead of a `rejected`
`noOffer` outcome. Call `start()` before commands, and handle local failures
under `status: "failed"`.
