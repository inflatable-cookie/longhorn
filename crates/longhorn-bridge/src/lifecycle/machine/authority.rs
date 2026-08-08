use crate::{
    BridgeAuthorityRequirement, BridgeLifecycleError, BridgeLifecycleErrorCode,
    BridgeNegotiationReceipt, BridgeRequiredAuthority, DomainAvailability, ExecutionAuthority,
    ReadAuthority, WriteAuthority,
};

pub(crate) fn validate_requirements(
    receipt: &BridgeNegotiationReceipt,
    requirements: &[BridgeAuthorityRequirement],
) -> Result<(), BridgeLifecycleError> {
    for requirement in requirements {
        let authority = receipt
            .domain_authorities()
            .iter()
            .find(|authority| authority.domain_id() == requirement.domain_id());
        let available = authority.is_some_and(|authority| {
            authority.availability() != DomainAvailability::Offline
                && match requirement.authority() {
                    BridgeRequiredAuthority::Available => true,
                    BridgeRequiredAuthority::Readable => {
                        authority.read_authority() != ReadAuthority::None
                    }
                    BridgeRequiredAuthority::AuthoritativeRead => {
                        authority.read_authority() == ReadAuthority::Authoritative
                    }
                    BridgeRequiredAuthority::Writable => {
                        authority.write_authority() == WriteAuthority::Authoritative
                    }
                    BridgeRequiredAuthority::Executable => {
                        authority.execution_authority() == ExecutionAuthority::Executor
                    }
                }
        });
        if !available {
            return Err(BridgeLifecycleError::new(
                BridgeLifecycleErrorCode::RequiredAuthorityUnavailable,
                format!(
                    "required {:?} authority unavailable for domain {}",
                    requirement.authority(),
                    requirement.domain_id()
                ),
            ));
        }
    }
    Ok(())
}
