use std::time::SystemTime;

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

    join_wait_timer: CasualTimer,
    join_pre_timer: CasualTimer,
}


#[derive(Clone)]
enum MlbtState {
    IDLE,   // uninitialized, will try to join 
    INIT,   // initializing, will try to merge
    WAIT,   // initializing, wait for other nodes
    ESTB,   // running
    JOIN_WAIT((Peer, Peer, u64)),   // src, waitfor, require msg id
    JOIN_PRE((Peer, Peer, u64)),   // src, subscriber, ack msg id
}


impl RelayCtl for MlbtRelayCtlContext {

    fn new() -> Self {
        Self {
            local_id: 0,
            state: MlbtState::IDLE,
            join_wait_timer: CasualTimer::new(Self::JOIN_WAIT_TO),
            join_pre_timer: CasualTimer::new(Self::JOIN_PRE_TO),
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
    fn heartbeat(&mut self, route_ctl: &mut RouteTable) -> Vec<(Peer, Vec<u8>)> {
        
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
        
        // clone because we may mutate self.state
        match self.state.clone() {
            
            MlbtState::JOIN_WAIT((src, waitfor, _)) => {
                if self.join_wait_timer.is_timeout() {
                    
                    // todo! what's the previous state before join_wait?
                    
                    self.state = MlbtState::IDLE;   // try to join other nodes
                    
                    // as if is rejected
                    self.join_pending(route_ctl, &src, &waitfor, None);

                }
            }


            MlbtState::JOIN_PRE((src, subscriber, _)) => {
                if self.join_pre_timer.is_timeout() {

                    self.state = MlbtState::ESTB;
                }
            }

            _ => {

            }
        
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
            MlbtState::JOIN_WAIT((src, cand, _)) => {
                
                // waiting for sender, one with bigger peer id accepts
                if cand == *sender && route_ctl.local_id() > *sender {

                    let ack = msg.accept(self.seq());
                    self.state = MlbtState::JOIN_PRE(
                        (join_msg.src(), sender.to_owned(), ack.msg_id())
                    );

                    ret.push((
                        sender.to_owned(),
                        ack
                    ));

                    // return early
                    return ret;
                }

                // reject other join requests
                ret.push((
                    sender.to_owned(),
                    msg.reject(self.seq())
                ));
            }

            MlbtState::ESTB => {
                // is a working node and state is clear, accept

                let ack = msg.accept(self.seq());
                self.state = MlbtState::JOIN_PRE(
                    (join_msg.src(), sender.to_owned(), ack.msg_id())
                );

                ret.push((
                    sender.to_owned(),
                    msg.accept(self.seq())
                ));
            }

            MlbtState::IDLE | MlbtState::INIT | MlbtState::WAIT | MlbtState::JOIN_PRE(_) => {
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
        self.join_wait_timer.set_now();

        // todo: figure out send_to success / failed 
        self.state = MlbtState::JOIN_WAIT((src.to_owned(), target.to_owned(), req_seq));

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
    
}