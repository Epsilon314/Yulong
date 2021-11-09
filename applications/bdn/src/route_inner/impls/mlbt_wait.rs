use yulong_network::identity::Peer;
use yulong::utils::CasualTimer;


#[derive(Clone)]
pub enum WaitStateKind {
    JOIN_WAIT((Peer, Peer, u64)),   // src, waitfor, require msg id
    JOIN_PRE((Peer, Peer, u64)),   // src, subscriber, ack msg id
    MERGE_WAIT((Peer, Peer, u64)), // src, waitfor, ack msg id
    MERGE_PRE((Peer, Peer, u64)), // src, requirer, ack msg id
}


#[derive(Clone)]
struct WaitState {
    wait_timer: CasualTimer,
    inner: WaitStateKind,
}


impl WaitState {

    const JOIN_WAIT_TO: u128 = 2000; // ms
    const JOIN_PRE_TO: u128 = 2000; // ms
    const MERGE_WAIT_TO: u128 = 2000; // ms
    const MERGE_PRE_TO: u128 = 2000; // ms
    
    fn join_wait_timer(src: &Peer, waitfor: &Peer, prev_msg_id: u64) -> Self {
        let mut ret = Self {
            wait_timer: CasualTimer::new(Self::JOIN_WAIT_TO),
            inner: WaitStateKind::JOIN_WAIT((src.to_owned(), waitfor.to_owned(), prev_msg_id)),
        };
        ret.wait_timer.set_now();
        ret
    }


    fn join_pre_timer(src: &Peer, waitfor: &Peer, prev_msg_id: u64) -> Self {
        let mut ret = Self {
            wait_timer: CasualTimer::new(Self::JOIN_PRE_TO),
            inner: WaitStateKind::JOIN_PRE((src.to_owned(), waitfor.to_owned(), prev_msg_id)),
        };
        ret.wait_timer.set_now();
        ret
    }


    fn merge_wait_timer(src: &Peer, waitfor: &Peer, prev_msg_id: u64) -> Self {
        let mut ret = Self {
            wait_timer: CasualTimer::new(Self::MERGE_WAIT_TO),
            inner: WaitStateKind::MERGE_WAIT((src.to_owned(), waitfor.to_owned(), prev_msg_id)),
        };
        ret.wait_timer.set_now();
        ret
    }


    fn merge_pre_timer(src: &Peer, waitfor: &Peer, prev_msg_id: u64) -> Self {
        let mut ret = Self {
            wait_timer: CasualTimer::new(Self::MERGE_PRE_TO),
            inner: WaitStateKind::MERGE_PRE((src.to_owned(), waitfor.to_owned(), prev_msg_id)),
        };
        ret.wait_timer.set_now();
        ret
    }


}


pub struct WaitList {
    join_wait: Option<WaitState>,
    join_pre: Option<WaitState>,

    merge_wait: Option<WaitState>,
    merge_pre: Option<WaitState>,
}


impl WaitList {
    
    pub fn new() -> Self {
        Self {
            join_wait: None,
            join_pre: None,

            merge_wait: None,
            merge_pre: None,
        }
    }


    pub fn is_waiting(&self) -> bool {
        self.get_join_wait().is_some() ||
        self.get_join_pre().is_some() ||
        self.get_merge_wait().is_some() ||
        self.get_merge_pre().is_some()
    }


    
    pub fn get_join_wait(&self) -> Option<(Peer, Peer, u64)> {
        if self.join_wait.is_some() {
            match self.join_wait.clone().unwrap().inner {
                WaitStateKind::JOIN_WAIT(s) => Some(s),
                _ => unreachable!()
            }
        }
        else {
            None
        }
    }


    pub fn set_join_wait(&mut self, src: &Peer, waitfor: &Peer, prev_msg_id: u64) {
        self.join_wait = Some(WaitState::join_wait_timer(src, waitfor, prev_msg_id));
    }


    pub fn clear_join_wait(&mut self) {self.join_wait = None;}


    pub fn check_join_wait(&mut self) -> Option<(Peer, Peer, u64)> {

        if let Some(state) = self.join_wait.clone() {

            if state.wait_timer.is_timeout() {

                // clear timer
                self.clear_join_wait();
                
                match &state.inner {
                    WaitStateKind::JOIN_WAIT(stored_value) => {                        
                        // as if is rejected
                        return Some(stored_value.to_owned());
                    }
                    _ => {unreachable!()}
                }
            }
        }
        // timer not set or not timeout
        None
    }


    pub fn get_join_pre(&self) -> Option<(Peer, Peer, u64)> {
        if self.join_pre.is_some() {
            match self.join_pre.clone().unwrap().inner {
                WaitStateKind::JOIN_PRE(s) => Some(s),
                _ => unreachable!()
            }
        }
        else {
            None
        }
    }


    pub fn set_join_pre(&mut self, src: &Peer, waitfor: &Peer, prev_msg_id: u64) { 
        self.join_pre = Some(WaitState::join_pre_timer(src, waitfor, prev_msg_id));
    }


    pub fn clear_join_pre(&mut self) {self.join_pre = None;}


    pub fn check_join_pre(&mut self) -> Option<(Peer, Peer, u64)> {

        if let Some(state) = self.join_pre.clone() {

            if state.wait_timer.is_timeout() {

                // clear timer
                self.join_pre = None;
                
                match &state.inner {
                    WaitStateKind::JOIN_PRE(stored_value) => {                        
                        // as if is rejected
                        return Some(stored_value.to_owned());
                    }
                    _ => {unreachable!()}
                }
            }
        }
        // timer not set or not timeout
        None
    }


    pub fn get_merge_wait(&self) -> Option<(Peer, Peer, u64)> {
        if self.merge_wait.is_some() {
            match self.merge_wait.clone().unwrap().inner {
                WaitStateKind::MERGE_WAIT(s) => Some(s),
                _ => unreachable!()
            }
        }
        else {
            None
        }
    }


    pub fn set_merge_wait(&mut self, src: &Peer, waitfor: &Peer, prev_msg_id: u64) {
        self.merge_wait = Some(WaitState::merge_wait_timer(src, waitfor, prev_msg_id));
    }


    pub fn clear_merge_wait(&mut self) {self.merge_wait = None;}


    pub fn check_merge_wait(&mut self) -> Option<(Peer, Peer, u64)> {
        if let Some(state) = self.merge_wait.clone() {

            if state.wait_timer.is_timeout() {

                // clear timer
                self.merge_wait = None;
                
                match &state.inner {
                    WaitStateKind::MERGE_WAIT(stored_value) => {                        
                        // as if is rejected
                        return Some(stored_value.to_owned());
                    }
                    _ => {unreachable!()}
                }
            }
        }
        // timer not set or not timeout
        None
    }


    pub fn get_merge_pre(&self) -> Option<(Peer, Peer, u64)> {
        if self.merge_pre.is_some() {
            match self.merge_pre.clone().unwrap().inner {
                WaitStateKind::MERGE_PRE(s) => Some(s),
                _ => unreachable!()
            }
        }
        else {
            None
        }
    }


    pub fn set_merge_pre(&mut self, src: &Peer, waitfor: &Peer, prev_msg_id: u64) {
        self.merge_pre = Some(WaitState::merge_pre_timer(src, waitfor, prev_msg_id));
    }


    pub fn clear_merge_pre(&mut self) {self.merge_pre = None;}


    pub fn check_merge_pre(&mut self) -> Option<(Peer, Peer, u64)> {
        if let Some(state) = self.merge_pre.clone() {

            if state.wait_timer.is_timeout() {

                // clear timer
                self.merge_pre = None;
                
                match &state.inner {
                    WaitStateKind::MERGE_PRE(stored_value) => {                        
                        // as if is rejected
                        return Some(stored_value.to_owned());
                    }
                    _ => {unreachable!()}
                }
            }
        }
        // timer not set or not timeout
        None
    }

}
