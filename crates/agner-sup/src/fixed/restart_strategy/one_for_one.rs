use std::collections::{HashSet, VecDeque};
use std::fmt;
use std::sync::Arc;

use agner_actors::{ActorID, ExitReason};
use agner_utils::std_error_pp::StdErrorPP;

use crate::fixed::restart_strategy::{
    Action, Decider, FrequencyPolicy, FrequencyStats, Instant, RestartStrategy,
};
#[derive(Debug, Clone, Default)]
pub struct OneForOne {
    pub frequency_policy: FrequencyPolicy,
}

pub struct OneForOneDecider {
    sup_id: ActorID,
    children: Box<[ActorID]>,
    failures: Box<[FrequencyStats]>,
    ignored_exits: HashSet<ActorID>,
    pending: VecDeque<Action>,
}

impl RestartStrategy for OneForOne {
    type Decider = OneForOneDecider;

    fn new_decider(&self, sup_id: ActorID, children: &[ActorID]) -> Self::Decider {
        let failures = children.iter().map(|_| self.frequency_policy.new_stats()).collect();
        OneForOneDecider {
            sup_id,
            children: children.into(),
            failures,
            ignored_exits: Default::default(),
            pending: Default::default(),
        }
    }
}

impl Decider for OneForOneDecider {
    fn next_action(&mut self) -> Option<super::Action> {
        self.pending.pop_front()
    }
    fn child_up(&mut self, _at: Instant, child_idx: usize, actor_id: ActorID) {
        self.children[child_idx] = actor_id;
    }
    fn actor_down(&mut self, at: Instant, actor_id: ActorID, exit_reason: ExitReason) {
        if self.sup_id == actor_id {
            log::info!("[{}] Requested shutdown", self);
            self.initiate_shutdown(exit_reason);
        } else if self.ignored_exits.remove(&actor_id) {
            log::trace!(
                "[{}] actor exited as expected {}, reason: {}",
                self,
                actor_id,
                exit_reason.pp()
            );
            return
        } else if let Some(idx) = self
            .children
            .iter()
            .enumerate()
            .find_map(|(idx, &id)| Some(idx).filter(|_| actor_id == id))
        {
            if self.failures[idx].report(at) {
                self.ignored_exits.extend(self.children.iter().copied());

                self.initiate_shutdown(ExitReason::shutdown_with_source(Arc::new(exit_reason)))
            } else {
                self.pending.push_back(Action::Start(idx));
            }
        } else {
            log::info!(
                "[{}] Unknown linked actor exited. Initiating shutdown. [reason: {}]",
                self,
                exit_reason.pp()
            );
            self.ignored_exits.extend(self.children.iter().copied());
            self.initiate_shutdown(exit_reason);
        }
    }
}

impl OneForOneDecider {
    fn initiate_shutdown(&mut self, exit_reason: ExitReason) {
        let arc_exit_reason = Arc::new(exit_reason.to_owned());
        self.pending.clear();
        self.pending.extend(
            self.children
                .iter()
                .copied()
                .enumerate()
                .rev()
                .map(|(child_idx, child_id)| {
                    Action::Stop(
                        child_idx,
                        child_id,
                        ExitReason::shutdown_with_source(arc_exit_reason.to_owned()),
                    )
                })
                .chain([Action::Exit(exit_reason)]),
        );
    }
}

impl fmt::Display for OneForOne {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "one-for-one: {}", self.frequency_policy)
    }
}

impl fmt::Display for OneForOneDecider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}|one-for-one", self.sup_id)
    }
}
