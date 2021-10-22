use std::time::SystemTime;

use crate::route::AppLayerRouteUser;
use crate::route_inner::RelayCtl;
use crate::route::RouteTable;
use crate::route::AppLayerRouteInner;
use crate::route_inner::impls::mlbt_message::RelayMsgKind;

use log::warn;
use yulong_network::identity::Peer;

use super::mlbt_message::{RelayCtlMessage, RelayMsgJoin,
    RelayMsgLeave, RelayMsgAccept, RelayMsgReject};


use yulong::utils::AsBytes;

pub struct MlbtRelayCtlContext {
    state: MlbtState,
    local_id: u64,

    timers: CtlMsgCasualTimer,
}


enum MlbtState {
    IDLE,   // uninitialized
    INIT,   // initializing
    ESTB,   // running
    JOIN_WAIT((Peer, u64)),
}


struct CtlMsgCasualTimer {
    last_seen: Option<SystemTime>,
}


impl CtlMsgCasualTimer {
    fn new() -> Self {
        Self {
            last_seen: None,
        }
    }

    fn set_now(&mut self) {
        self.last_seen = Some(SystemTime::now());
    }
}


impl RelayCtl for MlbtRelayCtlContext {

    fn new() -> Self {
        Self {
            local_id: 0,
            state: MlbtState::IDLE,
            timers: CtlMsgCasualTimer::new(),
        }
    }

    fn bootstrap(&mut self, route_ctl: &mut RouteTable) -> Vec<(Peer, Vec<u8>)> {
        todo!()
    }

    fn relay_ctl_callback(&mut self, route_ctl: &mut RouteTable, sender: &Peer,
        msg: &[u8]) -> Vec<(Peer, Vec<u8>)>     
    {

        let ret: Vec<(Peer, Vec<u8>)> = vec![];
        let parse_ctl_message = RelayCtlMessage::from_bytes(msg);
        if parse_ctl_message.is_err() {
            warn!("MlbtRelayCtlContext::relay_ctl_callback fail to parse RelayCtlMessage {:?}", msg);
            return ret;
        }
        
        // err is processed, shadow it with some
        let parse_ctl_message = parse_ctl_message.unwrap();

        // todo! timeout
        // if is not in idle and timeout is reached

        match parse_ctl_message.msg_type() {
            
            // todo pending callback dispatcher
            RelayMsgKind::ACCEPT => {

            }

            RelayMsgKind::REJECT => {
                
            }

            RelayMsgKind::JOIN => {

                // process join at any state

                let replys = self.join_callback(
                    route_ctl, sender, &parse_ctl_message);
                for (peer, ctl_msg_payload) in replys {
                    // build a ctl msg and turn into bytes
                }
                
            }

            RelayMsgKind::LEAVE => {

                // process leave at any state

                let replys = self.leave_callback(
                    route_ctl, sender, &parse_ctl_message);
                for (peer, ctl_msg_payload) in replys {
                    // build a ctl msg and turn into bytes
                }
            }
        

            _ => {

            }
        }

        todo!();
        ret
    }


    // called on temporal manner
    fn heartbeat(&mut self, route_ctl: &mut RouteTable) -> Vec<(Peer, Vec<u8>)> {
        todo!()
    }


}

impl MlbtRelayCtlContext {

    fn seq(&mut self) -> u64 {
        self.local_id += 1;
        self.local_id
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
        match &self.state {
            MlbtState::JOIN_WAIT((cand, _)) => {
                
                // waiting for sender, one with bigger peer id accepts
                if *cand == *sender && route_ctl.local_id() > *sender {
                    ret.push((
                        sender.to_owned(),
                        msg.accept(self.seq())
                    ));

                    return ret;
                }

                // reject other join requests
                ret.push((
                    sender.to_owned(),
                    msg.reject(self.seq())
                ));
                return ret;
            }

            _ => {
                // nothing special, add relay to route_table
                route_ctl.insert_relay(&join_msg.src(), sender);

                ret.push((
                    sender.to_owned(),
                    msg.accept(self.seq())
                ));
            }
        }
        ret
    }


    fn join(&mut self, target: &Peer, src: &Peer) -> Vec<(Peer, RelayCtlMessage)> {
        
        let mut ret: Vec<(Peer, RelayCtlMessage)> = vec![];
        let req_seq = self.seq();

        // log the time a join quest is made
        self.timers.set_now();

        // todo: figure out send_to success / failed 
        self.state = MlbtState::JOIN_WAIT((target.to_owned(), req_seq));

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
    fn join_pending(&mut self, waitfor: &Peer, res: bool) -> Vec<(Peer, RelayCtlMessage)> {
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
    
}