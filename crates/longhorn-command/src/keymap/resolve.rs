//! Physical key resolution against an effective keymap.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    CommandContextSnapshot, CommandKeyboardInput, CommandKeyboardMode, CommandPlatform,
    CommandReservedChordPolicy, CommandTextInputPolicy,
};

use super::{
    CommandBindingCandidate, CommandBindingSource, CommandBindingWinner,
    CommandCandidateDisposition, CommandEffectiveKeymap, CommandInvocation, CommandKeyResolution,
    CommandKeyResolutionError, CommandKeyboardGate, CommandKeymapConflict, conflict_from_matches,
    context_is_descendant, gated,
};

impl CommandEffectiveKeymap {
    /// Resolves one physical press against current context and injected gates.
    pub fn resolve(
        &self,
        platform: CommandPlatform,
        input: &CommandKeyboardInput,
        context: &CommandContextSnapshot,
        mode: CommandKeyboardMode,
        reserved: &impl CommandReservedChordPolicy,
    ) -> Result<CommandKeyResolution, CommandKeyResolutionError> {
        self.validate_context(context)?;

        if input.repeat {
            return Ok(gated(CommandKeyboardGate::Repeat));
        }
        if input.composing {
            return Ok(gated(CommandKeyboardGate::Composition));
        }
        if reserved.is_reserved(platform, &input.chord) {
            return Ok(gated(CommandKeyboardGate::Reserved));
        }
        if mode == CommandKeyboardMode::Capture {
            return Ok(CommandKeyResolution::Captured {
                chord: input.chord.clone(),
                label: input.chord.label(platform),
            });
        }

        let positions: BTreeMap<_, _> = context
            .path()
            .enumerate()
            .map(|(index, context_id)| (context_id, index))
            .collect();
        let mut matches: Vec<_> = self
            .bindings
            .iter()
            .filter(|binding| binding.platform.includes(platform))
            .filter_map(|binding| {
                let specificity = positions.get(&binding.context_id).copied()?;
                let chord = binding
                    .trigger
                    .resolve(platform)
                    .expect("effective binding modifiers were validated");
                (chord == input.chord).then_some((binding, specificity))
            })
            .collect();
        if matches.is_empty() {
            return Ok(CommandKeyResolution::Unbound);
        }
        matches.sort_by(|(left, left_specificity), (right, right_specificity)| {
            right_specificity
                .cmp(left_specificity)
                .then_with(|| left.id.cmp(&right.id))
        });

        let winning_specificity = matches[0].1;
        let winning_context = matches[0].0.context_id.clone();
        let winning_invocations: BTreeSet<_> = matches
            .iter()
            .take_while(|(_, specificity)| *specificity == winning_specificity)
            .map(|(binding, _)| binding.invocation.clone())
            .collect();
        let is_conflict = winning_invocations.len() > 1;
        let representative_id = matches[0].0.id.clone();
        let mut candidates = matches
            .iter()
            .map(|(binding, specificity)| CommandBindingCandidate {
                binding_id: binding.id.clone(),
                source: binding.source.clone(),
                matched_context_id: binding.context_id.clone(),
                specificity: *specificity,
                invocation: binding.invocation.clone(),
                disposition: if *specificity < winning_specificity {
                    CommandCandidateDisposition::Shadowed {
                        by_context_id: winning_context.clone(),
                    }
                } else if is_conflict {
                    CommandCandidateDisposition::Conflict
                } else if binding.id == representative_id {
                    CommandCandidateDisposition::Winner
                } else {
                    CommandCandidateDisposition::Equivalent
                },
            })
            .collect::<Vec<_>>();

        if is_conflict {
            let conflict = conflict_from_matches(
                platform,
                input.chord.clone(),
                winning_context,
                &matches,
                winning_specificity,
            );
            return Ok(CommandKeyResolution::Conflict {
                conflict,
                candidates,
            });
        }

        let binding = matches[0].0;
        let winner = CommandBindingWinner {
            binding_id: binding.id.clone(),
            matched_context_id: binding.context_id.clone(),
            invocation: binding.invocation.clone(),
        };
        if input.editable_text
            && self
                .text_input_policies
                .get(&winner.invocation.command_id)
                .is_some_and(|policy| *policy == CommandTextInputPolicy::Blocked)
        {
            return Ok(CommandKeyResolution::Gated {
                gate: CommandKeyboardGate::TextInput,
                candidates: {
                    candidates.shrink_to_fit();
                    candidates
                },
            });
        }

        Ok(CommandKeyResolution::Resolved { winner, candidates })
    }

    pub(crate) fn validate_context(
        &self,
        snapshot: &CommandContextSnapshot,
    ) -> Result<(), CommandKeyResolutionError> {
        let mut previous = None;
        for context_id in snapshot.path() {
            let Some(parent) = self.context_parents.get(context_id) else {
                return Err(CommandKeyResolutionError::UnknownContext);
            };
            if parent.as_ref() != previous {
                return Err(CommandKeyResolutionError::InvalidContextPath);
            }
            previous = Some(context_id);
        }
        Ok(())
    }
}
