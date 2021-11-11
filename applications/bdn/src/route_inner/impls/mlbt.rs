use std::cmp::min;

use crate::msg_header::MsgTypeKind;
use crate::msg_header::RelayMethodKind;

use crate::route::{AppLayerRouteUser, RouteTable, AppLayerRouteInner};

use crate::route_inner::{
    RelayCtl,
    impls::{
        mlbt_message::RelayMsgKind,
        mlbt_stat::MlbtStatList,
        mlbt_stat::MlbtStatMaintainer,
    }
};

use log::debug;
use log::warn;
use yulong_network::identity::Peer;

use super::mlbt_message::RelayMsgMerge;
use super::mlbt_message::RelayMsgMergeCheck;
use super::mlbt_message::{RelayCtlMessage, RelayMsgJoin,
    RelayMsgLeave, RelayMsgAccept, RelayMsgReject};

use super::mlbt_wait::{WaitList, TimedStates, WaitStateType, WaitStateData};

use yulong::utils::AsBytes;


pub struct MlbtRelayCtlContext {
    state: MlbtState,
    local_id: u64,

    wait_list: WaitList,

    mlbt_stat: MlbtStatList,
}


#[derive(Clone)]
enum MlbtState {
    IDLE,   // uninitialized, will try to join 
    INIT,   // initializing, will try to merge
    WAIT,   // initializing, wait for other nodes
    ESTB,   // running   
}


impl RelayCtl for MlbtRelayCtlContext {

    fn new() -> Self {
        Self {
            local_id: 0,
            state: MlbtState::IDLE,

            wait_list: WaitList::new(),

            mlbt_stat: MlbtStatList::new(),
        }
    }


    fn get_relay_method(&self) -> RelayMethodKind {
        RelayMethodKind::LOOKUP_TABLE_1
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
                if let Some((peer, ctl_msg_payload)) = reply {
                    // build a ctl msg and turn into bytes
                    ret.push((peer, ctl_msg_payload.into_bytes().unwrap()));
                }
                
            }

            RelayMsgKind::LEAVE => {
                self.leave_cb(route_ctl, sender, &parse_ctl_message);
            }

            RelayMsgKind::MERGE => {
                let reply = self.merge_cb(route_ctl, sender, &parse_ctl_message);
                if let Some((peer, ctl_msg_payload)) = reply {
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

    fn relay_receipt(&mut self, route_ctl: &mut RouteTable, all_success: bool) {
        
        // relay finish time
        
        // todo
    }


}

impl MlbtRelayCtlContext {

    fn seq(&mut self) -> u64 {
        self.local_id += 1;
        self.local_id
    }


    // reset timeouted timers and recover states
    fn check_timers(&mut self, route_ctl: &mut RouteTable) {

        // linear check all wait timers

        for root in route_ctl.get_src_list() {

            if let Some(timed_data) = 
                self.wait_list.check(&root, WaitStateType::JOIN_WAIT)
            {
                if let WaitStateData::JOIN_WAIT((src, waitfor, _)) = timed_data {
                    self.join_wait_cb(route_ctl, &src, &waitfor, None, false); // must be None
                }
                else {
                    unreachable!()
                }
            }


            if let Some(timed_data) = 
                self.wait_list.check(&root, WaitStateType::JOIN_PRE)
            {
                if let WaitStateData::JOIN_PRE((src, waitfor, _)) = timed_data {
                    self.join_pre_cb(route_ctl, &src, &waitfor, None, false);
                }
            }
    
            //todo

        }
    }


    // form a join request
    fn join(&mut self, target: &Peer, src: &Peer) -> Option<(Peer, RelayCtlMessage)> {
    
        if self.wait_list.is_waiting(src) {
            // todo: rethink join condition
            return None;
        }

        let req_seq = self.seq();

        // log the time a join quest is made
        // todo: figure out send_to success / failed
        self.wait_list.set(
            src,
            WaitStateData::JOIN_WAIT((src.to_owned(), target.to_owned(), req_seq))
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
            
                if self.wait_list.is_waiting(&join_msg.src()) {

                    // waiting for sender, one with bigger peer id accepts
                    // todo!(ack message can be skipped since recv a join req
                    // itself means has send capability and join intention)

                    if let Some(timed_data) = 
                        self.wait_list.get(&join_msg.src(), WaitStateType::JOIN_WAIT) 
                    {
                        if let WaitStateData::JOIN_WAIT((_, cand, _)) = timed_data {
                            if cand == *sender && route_ctl.local_id() > *sender {

                                let ack = msg.accept(self.seq());
    
                                self.wait_list.clear(&join_msg.src(), WaitStateType::JOIN_WAIT);
                                self.wait_list.set(
                                    &join_msg.src(),
                                    WaitStateData::JOIN_WAIT(
                                        (join_msg.src().clone(), sender.clone(), ack.msg_id())
                                    )
                                );
                                
                                return Some((
                                    sender.to_owned(),
                                    ack
                                ));
                            }
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

                self.wait_list.set(
                    &join_msg.src(),
                    WaitStateData::JOIN_WAIT(
                        (join_msg.src().clone(), sender.clone(), ack.msg_id())
                    )
                );

                Some((
                    sender.to_owned(),
                    ack
                ))
            }

            MlbtState::IDLE | MlbtState::INIT | MlbtState::WAIT => {
                // not ready to be subscribed, reject
                Some((
                    sender.to_owned(),
                    msg.reject(self.seq())
                ))
            }
        }
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
        self.wait_list.clear(src, WaitStateType::JOIN_WAIT);

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
        self.wait_list.clear(src, WaitStateType::JOIN_PRE);

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
    // do not need to ack leave
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

        if let Some(timed_data) = self.wait_list.get_by_id(decision_msg_ack) {

            match timed_data {
                
                WaitStateData::JOIN_WAIT((src, waitfor, id)) => {
                    let reply = self.join_wait_cb(
                        route_ctl, &src, &waitfor, Some(incoming_msg_id), pos);
                    
                    if reply.is_some() {
                        ret.push(reply.unwrap());
                    }
                    
                    return ret;
                }

                WaitStateData::JOIN_PRE((src, waitfor, id)) => {
                    self.join_pre_cb(route_ctl, &src, &waitfor, 
                        Some(incoming_msg_id), pos);
                    return ret;
                }

                WaitStateData::MERGE_WAIT((src, waitfor, id)) => {
                    let reply = self.merge_wait_cb(
                        route_ctl, &src, &waitfor, Some(incoming_msg_id), pos);
                    
                    if reply.is_some() {
                        ret.push(reply.unwrap());
                    }
                    return ret;
                }

                WaitStateData::MERGE_PRE((src, waitfor, id)) => {
                    self.merge_pre_cb(route_ctl, &src, &waitfor, 
                        Some(incoming_msg_id), pos);
                    return ret;
                }

                WaitStateData::MERGE_CHECK(_) => todo!(),
            }
        }
        
        warn!("MlbtRelayCtlContext::accept_dispatcher \
            Unmatched accept message: {:?}", msg);
        return ret;                
    }


    // require to merge
    fn merge(&mut self, src: &Peer, target: &Peer) -> Option<(Peer, RelayCtlMessage)> {

        // todo
        if self.wait_list.is_waiting(src) {
            return None;
        }


        let msg_seq = self.seq();

        // set timer
        self.wait_list.set(src, 
            WaitStateData::MERGE_WAIT((src.to_owned(), target.to_owned(), msg_seq))
        );

        let weight = self.mlbt_stat.relay_inv(src);
        if weight.is_none() {
            warn!("MlbtRelayCtlContext::merge relay_inv at root {} is unknown", src);
            return None;
        }
        let weight = weight.unwrap();   // checked, shadow it

        let merge_thrd = self.mlbt_stat.merge_thrd(src);
        if merge_thrd.is_none() {
            warn!("MlbtRelayCtlContext::merge merge_thr at root {} is unknown", src);
            return None;
        }
        let merge_thrd = merge_thrd.unwrap();

        Some((target.to_owned(), RelayCtlMessage::new(
            RelayMsgKind::MERGE,
            msg_seq,
            RelayMsgMerge::new(
                weight,
                merge_thrd,
                src
            )
        )))
    }


    fn merge_cb(&mut self, route_ctl: &mut RouteTable, sender: &Peer, 
        msg: &RelayCtlMessage) -> Option<(Peer, RelayCtlMessage)>
    {

        // decode merge request
        let merge_msg = RelayMsgMerge::from_bytes(&msg.payload());
        if merge_msg.is_err() {
            warn!("MlbtRelayCtlContext::merge_cb parse RelayMsgMerge failed: {}",
                merge_msg.unwrap_err());

            // ignore ill-formed messages
            return None; 
        }
        let merge_msg = merge_msg.unwrap();


        // todo! check if is waiting for other procedures
        if self.wait_list.is_waiting(&merge_msg.src()) {
            // todo
        }

        // check merge state
        let remote_weight = merge_msg.weight();
        
        let local_weight = self.mlbt_stat.relay_inv(&merge_msg.src());
        if local_weight.is_none() {
            warn!("MlbtRelayCtlContext::merge_cb relay_inv at root {} is unknown",
                &merge_msg.src());
            return None;
        }
        let local_weight = local_weight.unwrap();

        let merge_thrd = self.mlbt_stat.merge_thrd(&merge_msg.src());
        if merge_thrd.is_none() {
            warn!("MlbtRelayCtlContext::merge merge_thr at root {} is unknown", &merge_msg.src());
            return None;
        }
        let merge_thrd = merge_thrd.unwrap();


        let weight_diff: u64;
        if remote_weight > local_weight {
            weight_diff = remote_weight - local_weight;
        }
        else {
            weight_diff = local_weight - remote_weight;
        }
        
        let thrd = min(merge_msg.merge_thrd(), merge_thrd);

        let merge_cond = weight_diff < thrd;

        if !merge_cond {
            // rejected
            return Some((
                sender.to_owned(),
                msg.reject(self.seq())
            ));
        }

        // can accept, check self state
        match self.state.clone() {

            // node is trying to merge, will accept and pre for comfirmation
            MlbtState::INIT => {

                // todo!
                self.wait_list.set(
                    &merge_msg.src(), 
                    WaitStateData::MERGE_PRE((
                        merge_msg.src().clone(),
                        sender.to_owned(),
                        msg.msg_id()
                    ))
                );

                Some((
                    sender.to_owned(),
                    msg.accept(self.seq())
                ))
            }

            MlbtState::IDLE | MlbtState::ESTB | MlbtState::WAIT => {
                // not in mergeable state, reject

                Some((
                    sender.to_owned(),
                    msg.reject(self.seq())
                ))

            }

        }
    }


    // recv a merge request
    fn merge_wait_cb(&mut self, route_ctl: &mut RouteTable, src: &Peer, waitfor: &Peer,
        incoming_id: Option<u64>, accepted: bool) -> Option<(Peer, RelayCtlMessage)>
    {

        // anyway, the timer should be cleared
        self.wait_list.clear(src, WaitStateType::MERGE_WAIT);

        if incoming_id.is_none() {
            // timeout
            // todo! timeout cb
            return None;
        }

        if accepted {
            
            let ack_seq = self.seq();

            // todo timer
            self.wait_list.set(
                src,
                WaitStateData::MERGE_PRE((
                    src.to_owned(),
                    waitfor.to_owned(),
                    ack_seq
                ))
            );

            Some((
                waitfor.to_owned(),
                RelayCtlMessage::new(
                    RelayMsgKind::ACCEPT, 
                    ack_seq, 
                    RelayMsgAccept::from_id(incoming_id.unwrap()) // safe unwrap
                )
            ))
        }

        else {
            // rejected
            // todo 
            None
        }
    }


    fn merge_pre_cb(&mut self, route_ctl: &mut RouteTable, src: &Peer,
        waitfor: &Peer, incoming_id: Option<u64>, accepted: bool)
    {
        
        // clear timer
        self.wait_list.clear(src, WaitStateType::MERGE_PRE);

        if incoming_id.is_none() {
            // todo: timeout cb
            
            return;
        }

        if accepted {
            // one with larger id become the new root
            if route_ctl.local_id() > *waitfor {
                // todo: whats more?
                route_ctl.insert_front_relay(src, waitfor);
            }
            else {
                self.state = MlbtState::WAIT;
                route_ctl.reg_delegate(src, waitfor);
            }
        }
        else {
            // todo: rejected cb
        }
    }


    // send a check request to root
    fn merge_check(&mut self, src: &Peer, weight: u64) -> Option<(Peer, RelayCtlMessage)> {
        // todo
        if self.wait_list.is_waiting(src) {
            return None;
        }

        let msg_seq = self.seq();

        // set timer
        self.wait_list.set(
            src,
            WaitStateData::MERGE_CHECK((
                src.to_owned(),
                msg_seq
            ))
        );

        Some((src.to_owned(), RelayCtlMessage::new(
            RelayMsgKind::MERGE_CHECK,
            msg_seq,
            RelayMsgMergeCheck::new(weight)
        )))
    }


    // for root only
    fn merge_check_cb(sender: &Peer, msg: &RelayCtlMessage) -> Option<(Peer, RelayCtlMessage)> {

        // decode merge request
        let merge_msg = RelayMsgMergeCheck::from_bytes(&msg.payload());
        if merge_msg.is_err() {
            warn!("MlbtRelayCtlContext::merge_check_cb parse RelayMsgMergeCheck failed: {}",
                merge_msg.unwrap_err());

            // ignore ill-formed messages
            return None; 
        }
        let merge_msg = merge_msg.unwrap();

        // todo check self is src

        // let w = merge_msg.weight()

        None
    }

}