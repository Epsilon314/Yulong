
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

        self.check_timers(route_ctl);   // todo: time triggered sending quests?

        match parse_ctl_message.msg_type() {
            
            RelayMsgKind::ACCEPT => {
                let reply = self.decision_dispatcher(
                    route_ctl, &parse_ctl_message, true);
                for (peer, ctl_msg_payload) in reply {
                    ret.push((peer, ctl_msg_payload.into_bytes().unwrap()));
                }
            }

            RelayMsgKind::REJECT => {
                let reply = self.decision_dispatcher(
                    route_ctl, &parse_ctl_message, false);
                for (peer, ctl_msg_payload) in reply {
                    ret.push((peer, ctl_msg_payload.into_bytes().unwrap()));
                }
            }

            RelayMsgKind::JOIN => {
                let reply = self.join_cb(
                    route_ctl, sender, &parse_ctl_message);
                for (peer, ctl_msg_payload) in reply {
                    // build a ctl msg and turn into bytes
                    ret.push((peer, ctl_msg_payload.into_bytes().unwrap()));
                }
                
            }

            RelayMsgKind::LEAVE => {
                self.leave_cb(route_ctl, sender, &parse_ctl_message);
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

        // linear check all wait timers
        // todo! move as different functions if it grows too long

        if let Some(state) = self.wait_list.join_wait.clone() {

            if state.wait_timer.is_timeout() {
                
                match &state.inner {
                    WaitStateKind::JOIN_WAIT((src, waitfor, _)) => {                        
                        // as if is rejected
                        self.join_wait_cb(route_ctl, &src, &waitfor, None, false); // must be None
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
                    WaitStateKind::JOIN_PRE((src, waitfor, _)) => {
                        // as if is rejected
                        self.join_pre_cb(route_ctl, &src, &waitfor, None, false);
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


    // todo: use macro to be generic over WaitStateKind
    // helper func to check wait list


    // is in join_wait state, n.b. timeout has been checked several lines ago
    // thus we do not check it again 
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


    fn is_join_pre(&self) -> Option<(Peer, Peer, u64)> {
        match self.wait_list.join_pre.clone() {
            Some(state) => {
                if let WaitStateKind::JOIN_PRE(s) = state.inner {
                    Some(s)
                }
                else {
                    unreachable!()
                }
            }
            None => None
        }
    }


    // recv a join request
    fn join_cb(&mut self, route_ctl: &mut RouteTable, sender: &Peer, msg: &RelayCtlMessage)
        -> Option<(Peer, RelayCtlMessage)>     
    {
        let join_msg = RelayMsgJoin::from_bytes(&msg.payload());
        if join_msg.is_err() {
            warn!("MlbtRelayCtlContext::join_callback parse RelayMsgJoin failed: {}",
                join_msg.unwrap_err());
            // ignore ill-formed messages
            return None; 
        }
        let join_msg = join_msg.unwrap();

        // already have too many links, reject new ones
        if route_ctl.get_relay_count() >= RouteTable::MAX_LINK {
            return Some((
                sender.to_owned(),
                msg.reject(self.seq())
            ));
        }

        // accept it
        match self.state.clone() {
            
            MlbtState::ESTB => {
            
                if self.is_waiting() {

                    // waiting for sender, one with bigger peer id accepts
                    // todo!(ack message can be skipped since recv a join req
                    // itself means has send capability and join intention)
                    if let Some((_, cand, _)) = self.is_join_wait() {
                        if cand == *sender && route_ctl.local_id() > *sender {

                            let ack = msg.accept(self.seq());

                            self.wait_list.join_wait = None;
                            self.wait_list.join_pre = Some(WaitState::join_pre_timer(
                                &join_msg.src(), sender, ack.msg_id()));
                            
                            return Some((
                                sender.to_owned(),
                                ack
                            ));
                        }
                    }

                    // reject in other circumstances
                    return Some((
                        sender.to_owned(),
                        msg.reject(self.seq())
                    ));
                }

                // is a working node and state is clear, accept
                let ack = msg.accept(self.seq());

                self.wait_list.join_wait =  Some(WaitState::join_pre_timer(
                    &join_msg.src(), sender, ack.msg_id()));

                return Some((
                    sender.to_owned(),
                    msg.accept(self.seq())
                ));
            }

            MlbtState::IDLE | MlbtState::INIT | MlbtState::WAIT => {
                // not ready to be subscribed, reject
                return Some((
                    sender.to_owned(),
                    msg.reject(self.seq())
                ));
            }
        }
    }


    // form a join request
    fn join(&mut self, target: &Peer, src: &Peer) -> Option<(Peer, RelayCtlMessage)> {
        
        if self.is_join_pre().is_some() || self.is_join_wait().is_some() {
            // do not allow 
            return None;
        }

        let req_seq = self.seq();

        // log the time a join quest is made
        // todo: figure out send_to success / failed 
        self.wait_list.join_wait = Some(
            WaitState::join_wait_timer(src, target, req_seq)
        );

        Some((
            target.to_owned(),
            RelayCtlMessage::new(
                RelayMsgKind::JOIN,
                req_seq,
                RelayMsgJoin::new(src)
            )
        ))
    }


    // handle incoming reply to a prev join msg
    // it is ensured that the reply id matches what stores in wait_list
    //
    // src: related tree src
    // waitfor: remote peer
    // incoming_id: id of the related join request msg, None if timeout
    // bool: accept/reject
    fn join_wait_cb(&mut self, route_ctl: &mut RouteTable, src: &Peer, waitfor: &Peer,
        incoming_id: Option<u64>, accepted: bool) -> Option<(Peer, RelayCtlMessage)>
    {

        // anyway, the timer should be cleared
        self.wait_list.join_wait = None;

        if incoming_id.is_none() {
            // timeout

            // todo: timeout cb

            // do not need to send anything
            return None;
        }

        if accepted {
            // done for requiring side
            route_ctl.reg_delegate(src, waitfor);
            debug!("subscribed: {} through {}", src, waitfor);

            // notify receiving side to start relaying
            return Some((
                waitfor.to_owned(),
                RelayCtlMessage::new(
                    RelayMsgKind::ACCEPT, 
                    self.seq(), 
                    RelayMsgAccept::from_id(incoming_id.unwrap()) // safe unwrap
                )
            ));
        }

        // rejected
        // todo: rejected cb
        None
    }


    // handle incoming reply for join accept
    // do not reply in any circumstances
    //
    // src: related tree src
    // waitfor: remote peer
    // incoming_id: id of the related join request msg, None if timeout
    // bool: accept/reject
    fn join_pre_cb(&mut self, route_ctl: &mut RouteTable, src: &Peer,
         waitfor: &Peer, incoming_id: Option<u64>, accepted: bool)
    {

        // anyway, the timer should be cleared
        self.wait_list.join_pre = None;

        if incoming_id.is_none() {
            // todo: timeout cb
            return;
        }

        if accepted {
            route_ctl.insert_relay(src, waitfor);
            return;
        }

        // rejected
        // todo rejected cb
    }


    // received a leave request
    fn leave_cb(&mut self, route_ctl: &mut RouteTable, sender: &Peer, msg: &RelayCtlMessage)
    {
        match RelayMsgLeave::from_bytes(&msg.payload()) {
            Ok(leave_msg) => {
                route_ctl.remove_relay(&leave_msg.src(), sender);
            }

            Err(error) => {
                warn!("MlbtRelayCtlContext::join_callback parse RelayMsgLeave failed: {}", error);
                // ignore ill-formed messages 
            }
        }
    }


    // dispatch accept/reject msg to its cb according to the ack msg_id
    // return value is defined as a vec in case some cb may send several messages
    fn decision_dispatcher(&mut self, route_ctl: &mut RouteTable,
         msg: &RelayCtlMessage, pos: bool) -> Vec<(Peer, RelayCtlMessage)>
    {
        
        let mut ret: Vec<(Peer, RelayCtlMessage)> = vec![];

        // first decode the message

        let incoming_msg_id = msg.msg_id();
        let decision_msg_ack: u64;

        if pos {
            match RelayMsgAccept::from_bytes(&msg.payload()) {
                Ok(msg) => {
                    decision_msg_ack = msg.ack();
                }
                Err(error) => {
                    warn!("MlbtRelayCtlContext::decision_dispatcher\
                        Failed to parse RelayMsgAccept {}", error);
                    return ret;
                }
            }
        }
        else {
            match RelayMsgReject::from_bytes(&msg.payload()) {
                Ok(msg) => {
                    decision_msg_ack = msg.ack();
                }
                Err(error) => {
                    warn!("MlbtRelayCtlContext::decision_dispatcher\
                        Failed to parse RelayMsgReject {}", error);
                    return ret;
                }
            }
        }

        // check all pending wait linearly (and return early), if none of them matches
        // it may be ill-formed

        if let Some((src, waitfor, id)) = self.is_join_wait() {
            if decision_msg_ack == id {

                let reply = self.join_wait_cb(
                    route_ctl, &src, &waitfor, Some(incoming_msg_id), pos);
                
                if reply.is_some() {
                    ret.push(reply.unwrap());
                }
                
                return ret;
            }
        }


        if let Some((src, waitfor, id)) = self.is_join_pre() {
            if decision_msg_ack == id {

                self.join_pre_cb(route_ctl, &src, &waitfor, 
                    Some(incoming_msg_id), pos);
                return ret;
            }
        }
        
        warn!("MlbtRelayCtlContext::accept_dispatcher \
            Unmatched accept message: {:?}", msg);
        return ret;                
    }

}