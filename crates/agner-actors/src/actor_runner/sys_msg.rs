use tokio::sync::oneshot;

use crate::actor_id::ActorID;
use crate::exit_reason::ExitReason;

use super::Backend;

#[derive(Debug)]
pub enum SysMsg {
    Link(ActorID),
    Unlink(ActorID),
    SigExit(ActorID, ExitReason),
    Wait(oneshot::Sender<ExitReason>),

    GetInfo(oneshot::Sender<ActorInfo>),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ActorInfo {
    pub actor_id: ActorID,
    pub m_queue_len: (usize, usize),
    pub s_queue_len: (usize, usize),
    pub c_queue_len: (usize, usize),
    pub tasks_count: usize,
    pub trap_exit: bool,
    pub links: Box<[ActorID]>,
    pub waits_len: usize,
}

impl<M> Backend<M> {
    pub(super) async fn send_sys_msg(&self, to: ActorID, sys_msg: SysMsg) -> bool {
        if let Some(system) = self.system_opt.rc_upgrade() {
            system.send_sys_msg(to, sys_msg).await
        } else {
            false
        }
    }
}
