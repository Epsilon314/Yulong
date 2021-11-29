use std::collections::HashMap;

use log::warn;
use yulong_network::identity::Peer;
use yulong::utils::CasualTimer;

#[derive(Clone)]
pub enum WaitStateData {
    JoinWait((Peer, Peer, u64)),   // src, waitfor, require msg id
    JoinPre((Peer, Peer, u64)),   // src, subscriber, ack msg id

    MergeWait((Peer, Peer, u64)), // src, waitfor, ack msg id
    MergePre((Peer, Peer, u64)), // src, requirer, ack msg id
    // src, merge target, accept request msg id, require msg id
    MergeCheck((Peer, Peer, u64)),
    
    GrantWait((Peer, Peer, u64)), // src, receiver, request msg id
    GrantJoin((Peer, Peer, u64)), // src, recv, join msg id
    GrantRecv((Peer, Peer)), // src, expecting 
    GrantTotal((Peer, Peer)), // src, grantee

    RetractWait((Peer, Peer, u64)), // src, recv, request msg id
    RetractJoin((Peer, Peer, u64)), // src, recv, request msg id
    RetractRecv((Peer, Peer, u64)), // src, expecting, request msg id
    RetractTotal((Peer, Peer)), // src, grantee
}


// only know type, do not known associated meta-data
pub enum WaitStateType {
    JoinWait,
    JoinPre,

    MergeWait,
    MergePre,
    MergeCheck,
    
    GrantWait,
    GrantJoin,
    GrantRecv,
    GrantTotal,
    
    RetractWait,
    RetractJoin,
    RetractRecv,
    RetractTotal,
}

// todo: initial from config 
// note for reconstruct:
// MERGE_PRE_TO should exceed MERGE_CHECK_TO + MERGE_WAIT_TO
static JOIN_WAIT_TO: u128 = 2000; // ms
static JOIN_PRE_TO: u128 = 2000; // ms
static MERGE_WAIT_TO: u128 = 2000; // ms
static MERGE_PRE_TO: u128 = 4000; // ms
static MERGE_CHECK_TO: u128 = 2000; // ms

trait TimedStatesSingle {

    // get associated data of a type if any
    fn get(&self, state_type: WaitStateType) -> Option<WaitStateData>;

    // get the data if it is timeout, get None elsewise
    fn check(&self, state_type: WaitStateType) -> Option<WaitStateData>;

    // set state data and start the timer now
    fn set(&mut self, state: WaitStateData);

    // clear the timed state by type
    fn clear(&mut self, state_type: WaitStateType);

    fn check_id(&self, id: u64) -> Option<WaitStateData>;

}


// TimedStatesSingle expect that indexed by root Peer
// used as the interface
pub trait TimedStates {

    // get associated data of a type if any
    fn get(&self, peer: &Peer, state_type: WaitStateType) -> Option<WaitStateData>;

    // get the data if it is timeout, get None elsewise
    fn check(&self, peer: &Peer, state_type: WaitStateType) -> Option<WaitStateData>;

    // set state data and start the timer now
    fn set(&mut self, peer: &Peer, state: WaitStateData);

    // clear the timed state by type
    fn clear(&mut self, peer: &Peer, state_type: WaitStateType);

    fn get_by_id(&self, id: u64) -> Option<WaitStateData>;

}


#[derive(Clone)]
struct WaitState {
    wait_timer: CasualTimer,
    inner: WaitStateData,
}


impl WaitState {

    // create a timed state and start the timer, the state is considered stale after timeout
    // todo use macros to be generic over types ?
    fn new(kind: WaitStateData) -> Self {
        let mut ret: Self;

        match kind {
            WaitStateData::JoinWait(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(JOIN_WAIT_TO),
                    inner: WaitStateData::JoinWait(data),
                };
            }

            WaitStateData::JoinPre(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(JOIN_PRE_TO),
                    inner: WaitStateData::JoinPre(data),
                };
            },
            
            WaitStateData::MergeWait(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(MERGE_WAIT_TO),
                    inner: WaitStateData::MergeWait(data),
                };
            },
            
            WaitStateData::MergePre(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(MERGE_PRE_TO),
                    inner: WaitStateData::MergePre(data),
                };
            },
            
            WaitStateData::MergeCheck(data) => {
                ret = Self {
                    wait_timer: CasualTimer::new(MERGE_CHECK_TO),
                    inner: WaitStateData::MergeCheck(data),
                };
            },
            WaitStateData::GrantWait(_) => todo!(),
            WaitStateData::GrantJoin(_) => todo!(),
            WaitStateData::RetractWait(_) => todo!(),
            WaitStateData::RetractJoin(_) => todo!(),
            WaitStateData::GrantRecv(_) => todo!(),
            WaitStateData::GrantTotal(_) => todo!(),
            WaitStateData::RetractRecv(_) => todo!(),
            WaitStateData::RetractTotal(_) => todo!(),
            
        }

        ret.wait_timer.set_now();
        ret
    }

}

struct WaitStats {
    join_wait: Option<WaitState>,
    join_pre: Option<WaitState>,

    merge_wait: Option<WaitState>,
    merge_pre: Option<WaitState>,
    merge_check: Option<WaitState>,

    grant_wait: Option<WaitState>,
    grant_join: Option<WaitState>,
    retract_wait: Option<WaitState>,
    retract_join: Option<WaitState>,

}


impl WaitStats {
    pub fn new() -> Self {
        Self {
            join_wait: None,
            join_pre: None,

            merge_wait: None,
            merge_pre: None,
            merge_check: None,
            grant_wait: None,
            grant_join: None,
            retract_wait: None,
            retract_join: None,
        }
    }
}


impl TimedStatesSingle for WaitStats {

    fn get(&self, state_type: WaitStateType) -> Option<WaitStateData> {
        match state_type {
            WaitStateType::JoinWait => {
                match self.join_wait.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }

            WaitStateType::JoinPre => {
                match self.join_pre.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }

            WaitStateType::MergeWait => {
                match self.merge_wait.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }

            WaitStateType::MergePre => {
                match self.merge_pre.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }

            WaitStateType::MergeCheck => {
                match self.merge_check.clone() {
                    Some(stat) => Some(stat.inner),
                    None => None
                }
            }
            WaitStateType::GrantWait => todo!(),
            WaitStateType::GrantJoin => todo!(),
            WaitStateType::RetractWait => todo!(),
            WaitStateType::RetractJoin => todo!(),
            WaitStateType::GrantRecv => todo!(),
            WaitStateType::GrantTotal => todo!(),
            WaitStateType::RetractRecv => todo!(),
            WaitStateType::RetractTotal => todo!(),
        }
    }


    fn check(&self, state_type: WaitStateType) -> Option<WaitStateData> {
        match state_type {
            WaitStateType::JoinWait => {
                match self.join_wait.clone() {
                    Some(stat) => {
                        if stat.wait_timer.is_timeout() {
                            Some(stat.inner)
                        }
                        else {
                            None
                        }
                    }
                    None => None
                }
            }

            WaitStateType::JoinPre => {
                match self.join_pre.clone() {
                    Some(stat) => {
                        if stat.wait_timer.is_timeout() {
                            Some(stat.inner)
                        }
                        else {
                            None
                        }
                    }
                    None => None
                }
            }

            WaitStateType::MergeWait => {
                match self.merge_wait.clone() {
                    Some(stat) => {
                        if stat.wait_timer.is_timeout() {
                            Some(stat.inner)
                        }
                        else {
                            None
                        }
                    }
                    None => None
                }
            }

            WaitStateType::MergePre => {
                match self.merge_pre.clone() {
                    Some(stat) => {
                        if stat.wait_timer.is_timeout() {
                            Some(stat.inner)
                        }
                        else {
                            None
                        }
                    }
                    None => None
                }
            }

            WaitStateType::MergeCheck => {
                match self.merge_check.clone() {
                    Some(stat) => {
                        if stat.wait_timer.is_timeout() {
                            Some(stat.inner)
                        }
                        else {
                            None
                        }
                    }
                    None => None
                }
            }
            WaitStateType::GrantWait => todo!(),
            WaitStateType::GrantJoin => todo!(),
            WaitStateType::RetractWait => todo!(),
            WaitStateType::RetractJoin => todo!(),
            WaitStateType::GrantRecv => todo!(),
            WaitStateType::GrantTotal => todo!(),
            WaitStateType::RetractRecv => todo!(),
            WaitStateType::RetractTotal => todo!(),
        }
    }


    fn set(&mut self, state: WaitStateData) {
        match state {
            WaitStateData::JoinWait(_) => {
                self.join_wait = Some(WaitState::new(state))
            }

            WaitStateData::JoinPre(_) => {
                self.join_pre = Some(WaitState::new(state))
            }

            WaitStateData::MergeWait(_) => {
                self.merge_wait = Some(WaitState::new(state))
            }

            WaitStateData::MergePre(_) => {
                self.merge_pre = Some(WaitState::new(state))
            }

            WaitStateData::MergeCheck(_) => {
                self.merge_check = Some(WaitState::new(state))
            }
            WaitStateData::GrantWait(_) => todo!(),
            WaitStateData::GrantJoin(_) => todo!(),
            WaitStateData::RetractWait(_) => todo!(),
            WaitStateData::RetractJoin(_) => todo!(),
            WaitStateData::GrantRecv(_) => todo!(),
            WaitStateData::GrantTotal(_) => todo!(),
            WaitStateData::RetractRecv(_) => todo!(),
            WaitStateData::RetractTotal(_) => todo!(),
            
        }
    }


    fn clear(&mut self, state_type: WaitStateType) {
        match state_type {
            WaitStateType::JoinWait => {
                self.join_wait = None
            }

            WaitStateType::JoinPre => {
                self.join_pre = None
            }

            WaitStateType::MergeWait => {
                self.merge_wait = None
            }

            WaitStateType::MergePre => {
                self.merge_pre = None
            }

            WaitStateType::MergeCheck => {
                self.merge_check = None
            }
            WaitStateType::GrantWait => todo!(),
            WaitStateType::GrantJoin => todo!(),
            WaitStateType::RetractWait => todo!(),
            WaitStateType::RetractJoin => todo!(),
            WaitStateType::GrantRecv => todo!(),
            WaitStateType::GrantTotal => todo!(),
            WaitStateType::RetractRecv => todo!(),
            WaitStateType::RetractTotal => todo!(),
        }
    }

    fn check_id(&self, id: u64) -> Option<WaitStateData> {

        if let Some(join_wait_data) = &self.join_wait {
            match &join_wait_data.inner {
                WaitStateData::JoinWait((_, _, sid)) => {
                    if *sid == id {
                        return Some(join_wait_data.inner.clone())
                    }
                }
                _ => {unreachable!()}
            }
        }

        if let Some(join_pre_data) = &self.join_pre {
            match &join_pre_data.inner {
                WaitStateData::JoinPre((_, _, sid)) => {
                    if *sid == id {
                        return Some(join_pre_data.inner.clone())
                    }
                }
                _ => {unreachable!()}
            }
        }

        if let Some(merge_wait_data) = &self.merge_wait {
            match &merge_wait_data.inner {
                WaitStateData::MergeWait((_, _, sid)) => {
                    if *sid == id {
                        return Some(merge_wait_data.inner.clone())
                    }
                }
                _ => {unreachable!()}
            }
        }

        if let Some(merge_pre_data) = &self.merge_pre {
            match &merge_pre_data.inner {
                WaitStateData::MergePre((_, _, sid)) => {
                    if *sid == id {
                        return Some(merge_pre_data.inner.clone())
                    }
                }
                _ => {unreachable!()}
            }
        }

        if let Some(merge_check_data) = &self.merge_check {
            match &merge_check_data.inner {
                WaitStateData::MergeCheck((_, _, sid)) => {
                    if *sid == id {
                        return Some(merge_check_data.inner.clone())
                    }
                }
                _ => {unreachable!()}
            }
        }

        None
    }

}

pub struct WaitList {
    inner_list: HashMap<Peer, WaitStats>,
}


impl WaitList {

    pub fn new() -> Self {
        Self {
            inner_list: HashMap::new(),
        } 
    }


    // todo: update me
    pub fn is_waiting(&self, root: &Peer) -> bool {
        self.get(root, WaitStateType::JoinWait).is_some() ||
        self.get(root, WaitStateType::JoinPre).is_some() ||
        self.get(root, WaitStateType::MergeWait).is_some() ||
        self.get(root, WaitStateType::MergeWait).is_some() ||
        self.get(root, WaitStateType::MergeCheck).is_some()
    }

}


impl TimedStates for WaitList {
    
    fn get(&self, peer: &Peer, state_type: WaitStateType) -> Option<WaitStateData> {
        match self.inner_list.get(peer) {
            Some(states) => {
                states.get(state_type)
            }
            None => None
        }   
    }


    fn check(&self, peer: &Peer, state_type: WaitStateType) -> Option<WaitStateData> {
        match self.inner_list.get(peer) {
            Some(states) => {
                states.check(state_type)
            }
            None => None
        }
    }


    fn set(&mut self, peer: &Peer, state: WaitStateData) {
        match self.inner_list.get_mut(peer) {
            Some(states) => {
                states.set(state)
            }
            None => {
                // todo log it or throw errors up?
                warn!("")
            }
        }
    }

    fn clear(&mut self, peer: &Peer, state_type: WaitStateType) {
        match self.inner_list.get_mut(peer) {
            Some(states) => {
                states.clear(state_type)
            }
            None => {
                // todo log it or throw errors up?
                warn!("")
            }
        }
    }

    fn get_by_id(&self, id: u64) -> Option<WaitStateData> {
        for data in self.inner_list.values() {
            match data.check_id(id) {
                Some(data) => return Some(data),
                None => {}
            }
        }
        // no matching data
        None
    }

}