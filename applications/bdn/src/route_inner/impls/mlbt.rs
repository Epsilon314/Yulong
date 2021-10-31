use core::time;
use std::time::SystemTime;

use crate::msg_header::MsgTypeKind;
use crate::msg_header::RelayMethodKind;
use crate::route::AppLayerRouteUser;
use crate::route_inner::RelayCtl;
use crate::route::RouteTable;
use crate::route::AppLayerRouteInner;
use crate::route_inner::impls::mlbt_message::RelayMsgKind;

use log::debug;
use log::warn;
use yulong_network::identity::Peer;

use super::mlbt_message::{RelayCtlMessage, RelayMsgJoin,
    RelayMsgLeave, RelayMsgAccept, RelayMsgReject};


use yulong::utils::AsBytes;
use yulong::utils::CasualTimer;

pub struct MlbtRelayCtlContext {
    state: MlbtState,
    local_id: u64,

    wait_list: WaitList,
}


#[derive(Clone)]
enum MlbtState {
    IDLE,   // uninitialized, will try to join 
    INIT,   // initializing, will try to merge
    WAIT,   // initializing, wait for other nodes
    ESTB,   // running   
}


#[derive(Clone)]
enum WaitStateKind {
    JOIN_WAIT((Peer, Peer, u64)),   // src, waitfor, require msg id
    JOIN_PRE((Peer, Peer, u64)),   // src, subscriber, ack msg id
}


#[derive(Clone)]
struct WaitState {
    wait_timer: CasualTimer,
    inner: WaitStateKind,
}


struct WaitList {
    join_wait: Option<WaitState>,
    join_pre: Option<WaitState>,
}


impl WaitList {
    fn new() -> Self {
        Self {
            join_wait: None,
            join_pre: None,
        }
    }
}


impl WaitState {
    
    fn join_wait_timer(src: &Peer, waitfor: &Peer, prev_msg_id: u64) -> Self {
        let mut ret = Self {
            wait_timer: CasualTimer::new(MlbtRelayCtlContext::JOIN_WAIT_TO),
            inner: WaitStateKind::JOIN_WAIT((src.to_owned(), waitfor.to_owned(), prev_msg_id)),
        };
        ret.wait_timer.set_now();
        ret
    }


    fn join_pre_timer(src: &Peer, waitfor: &Peer, prev_msg_id: u64) -> Self {
        let mut ret = Self {
            wait_timer: CasualTimer::new(MlbtRelayCtlContext::JOIN_PRE_TO),
            inner: WaitStateKind::JOIN_PRE((src.to_owned(), waitfor.to_owned(), prev_msg_id)),
        };
        ret.wait_timer.set_now();
        ret
    }

}


impl RelayCtl for MlbtRelayCtlContext {

    fn new() -> Self {
        Self {
            local_id: 0,
            state: MlbtState::IDLE,

            wait_list: WaitList::new(),
        }
    }

    fn bootstrap(&mut self, route_ctl: &mut RouteTable) -> Vec<(Peer, Vec<u8>)> {
        todo!()
    }

    fn relay_ctl_callback(&mut self, route_ctl: &mut RouteTable, sender: &Peer,
        msg: &[u8]) -> Vec<(Peer, Vec<u8>)>     
    {

        let mut ret: Vec<(Peer, Vec<u8>)> = vec![];
        let parse_ctl_message = RelayCtlMessage::from_bytes(msg);
        if parse_ctl_message.is_err() {
            warn!("MlbtRelayCtlContext::relay_ctl_callback fail to parse RelayCtlMessage {:?}", msg);
            return ret;
        }
        
        // err is processed, shadow it with some
        let parse_ctl_message = parse_ctl_message.unwrap();

        // todo! timeout
        // if is not in idle and timeout is reached
        self.check_timers(route_ctl);

        match parse_ctl_message.msg_type() {
            
            // todo pending callback dispatcher
            RelayMsgKind::ACCEPT => {

            }

            RelayMsgKind::REJECT => {
                
            }

            RelayMsgKind::JOIN => {
                let replys = self.join_callback(
                    route_ctl, sender, &parse_ctl_message);
                for (peer, ctl_msg_payload) in replys {
                    // build a ctl msg and turn into bytes
                    ret.push((peer, ctl_msg_payload.into_bytes().unwrap()));
                }
                
            }

            RelayMsgKind::LEAVE => {
                let replys = self.leave_callback(
                    route_ctl, sender, &parse_ctl_message);
                for (peer, ctl_msg_payload) in replys {
                    // build a ctl msg and turn into bytes
                    ret.push((peer, ctl_msg_payload.into_bytes().unwrap()));
                }
            }
        

            _ => {

            }
        }

        ret
    }


    // called on temporal manner
    fn heartbeat(&self, route_ctl: &RouteTable) -> Vec<(Peer, Vec<u8>)> {
        
        let mut ret: Vec<(Peer, Vec<u8>)> = vec![];

        // check for temporal tasks

        match self.state {
            MlbtState::IDLE => {
                // try to join at INIT state
            }

            MlbtState::INIT => {
                // try to merge
            }

            MlbtState::ESTB => {
                // try to rebalance
            }

            _ => {

            }
        }

        ret
    }


}

impl MlbtRelayCtlContext {

    const JOIN_WAIT_TO: u128 = 2000; // ms
    const JOIN_PRE_TO: u128 = 2000; // ms

    fn seq(&mut self) -> u64 {
        self.local_id += 1;
        self.local_id
    }


    // reset timeouted timers and recover states
    fn check_timers(&mut self, route_ctl: &mut RouteTable) {

        
        if let Some(state) = self.wait_list.join_wait.clone() {
            if state.wait_timer.is_timeout() {
                
                match &state.inner {
                    WaitStateKind::JOIN_WAIT((src, waitfor, _)) => {
                        // todo! what's the previous state before join_wait?
                                                
                        self.state = MlbtState::IDLE;   // try to join other nodes
                                                
                        // as if is rejected
                        self.join_pending(route_ctl, &src, &waitfor, None);
                    }
                    _ => {unreachable!()}
                }

                // clear timeout timer
                self.wait_list.join_wait = None;
            }
        }


        if let Some(state) = self.wait_list.join_pre.clone() {
            if state.wait_timer.is_timeout() {
                match state.inner {
                    WaitStateKind::JOIN_PRE((_, _, _)) => {
                        // only estb nodes will accept join request and enter this state
                        self.state = MlbtState::ESTB;
                    }
                    _ => {unreachable!()}
                }

                // clear timeout timer
                self.wait_list.join_pre = None;
            }
        }

    }


    fn is_waiting(&self) -> bool {
        self.wait_list.join_pre.is_some() || self.wait_list.join_wait.is_some()
    }


    // todo use macro to be generic 
    fn is_join_wait(&self) -> Option<(Peer, Peer, u64)> {
        match self.wait_list.join_wait.clone() {
            Some(state) => {
                if let WaitStateKind::JOIN_WAIT(s) = state.inner {
                    Some(s)
                }
                else {
                    unreachable!()
                }
            }
            None => None
        }
    }


    fn join_callback(&mut self, route_ctl: &mut RouteTable, sender: &Peer, msg: &RelayCtlMessage)
        -> Vec<(Peer, RelayCtlMessage)>     
    {
        let mut ret: Vec<(Peer, RelayCtlMessage)> = vec![];
        
        let join_msg = RelayMsgJoin::from_bytes(&msg.payload());
        if join_msg.is_err() {
            warn!("MlbtRelayCtlContext::join_callback parse RelayMsgJoin failed: {}",
                join_msg.unwrap_err());
            // ignore ill-formed messages
            return ret; 
        }
        let join_msg = join_msg.unwrap();

        // already have too many links, reject new ones
        if route_ctl.get_relay_count() >= RouteTable::MAX_LINK {
            ret.push((
                sender.to_owned(),
                msg.reject(self.seq())
            ));
            return ret; 
        }

        // accept it
        match self.state.clone() {
            
            MlbtState::ESTB => {
            
                if self.is_waiting() {
                    if let Some((_, cand, _)) = self.is_join_wait() {
   
                        // waiting for sender, one with bigger peer id accepts
                        if cand == *sender && route_ctl.local_id() > *sender {

                            let ack = msg.accept(self.seq());

                            self.wait_list.join_wait = None;
                            self.wait_list.join_pre = Some(WaitState::join_pre_timer(
                                &join_msg.src(), sender, ack.msg_id()));

                            ret.push((
                                sender.to_owned(),
                                ack
                            ));

                            // return early
                            return ret;
                        }
                    }
                    // reject other join requests
                    ret.push((
                        sender.to_owned(),
                        msg.reject(self.seq())
                    ));
                    return ret;
                }

                // is a working node and state is clear, accept
                let ack = msg.accept(self.seq());

                self.wait_list.join_wait =  Some(WaitState::join_pre_timer(
                    &join_msg.src(), sender, ack.msg_id()));

                ret.push((
                    sender.to_owned(),
                    msg.accept(self.seq())
                ));
            }

            MlbtState::IDLE | MlbtState::INIT | MlbtState::WAIT => {
                // not ready to be subscribed, reject
                ret.push((
                    sender.to_owned(),
                    msg.reject(self.seq())
                ));
            }
        }
        ret
    }


    fn join(&mut self, target: &Peer, src: &Peer) -> Vec<(Peer, RelayCtlMessage)> {
        
        let mut ret: Vec<(Peer, RelayCtlMessage)> = vec![];
        let req_seq = self.seq();

        // log the time a join quest is made
        // todo: figure out send_to success / failed 
        self.wait_list.join_wait = Some(
            WaitState::join_wait_timer(src, target, req_seq)
        );

        ret.push((
            target.to_owned(),
            RelayCtlMessage::new(
                RelayMsgKind::JOIN,
                req_seq,
                RelayMsgJoin::new(src)
            )
        ));

        ret
    }


    // handle incoming reply to a prev join msg
    fn join_pending(&mut self, route_ctl: &mut RouteTable,
         src: &Peer, waitfor: &Peer, msg_id: Option<u64>) -> Vec<(Peer, RelayCtlMessage)> 
    {
        
        let mut ret: Vec<(Peer, RelayCtlMessage)> = vec![];

        match msg_id {

            // accepted
            Some(id) => {
                
                // done for requiring side
                route_ctl.reg_delegate(src, waitfor);
                debug!("subscribed: {} through {}", src, waitfor);

                // notify receiving side to start relaying
                ret.push((
                    waitfor.to_owned(),
                    RelayCtlMessage::new(
                        RelayMsgKind::ACCEPT, 
                        self.seq(), 
                        RelayMsgAccept::from_id(id)
                    )
                ));
            }

            // rejected
            None => {

            }
        }
        ret
    }


    fn join_ack_pending(&mut self, route_ctl: &mut RouteTable,
        src: &Peer, waitfor: &Peer, msg_id: Option<u64>) -> Vec<(Peer, RelayCtlMessage)>
    {
        todo!()
    }


    fn leave_callback(&mut self, route_ctl: &mut RouteTable, sender: &Peer, msg: &RelayCtlMessage)
        -> Vec<(Peer, RelayCtlMessage)> 
    {
        let mut ret: Vec<(Peer, RelayCtlMessage)> = vec![];
        
        match RelayMsgLeave::from_bytes(&msg.payload()) {
            Ok(leave_msg) => {

                route_ctl.remove_relay(&leave_msg.src(), sender);

                // acknowledge relay entry has been removed (todo: make this optional)
                ret.push((
                    sender.to_owned(),
                    msg.accept(self.seq())
                ));
            }

            Err(error) => {
                warn!("MlbtRelayCtlContext::join_callback parse RelayMsgLeave failed: {}", error);
                // ignore ill-formed messages 
            }
        }
        
        ret
    }


    fn accept_dispatcher(&mut self, msg: &RelayCtlMessage)  {

        if let Some((src, waitfor, id)) = self.is_join_wait() {
            
            let acp_msg_id = msg.msg_id();
            
            match RelayMsgAccept::from_bytes(&msg.payload()) {
                Ok(accept_msg) => {
                    
                    // everything is in-order
                    if accept_msg.ack() == id {
                    }
                
                }
                Err(error) => {
                    warn!("MlbtRelayCtlContext::accept_dispatcher\
                        Failed to parse RelayMsgAccept {}", error)
                }
            }
            return;
        }
        
        warn!("MlbtRelayCtlContext::accept_dispatcher \
            Unmatched accept message: {:?}", msg);                
    }
    
}