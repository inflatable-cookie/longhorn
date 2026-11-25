Single-adapter restore can return `RestoreAdapterError::Coordination`.
Handle the variant and retry a busy coordinator after it becomes available.
