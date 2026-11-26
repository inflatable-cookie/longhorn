A bounded Git call and a clone cancellation end the child's process group
before the child is reaped. A process-group id is no longer signalled after its
leader was collected.
