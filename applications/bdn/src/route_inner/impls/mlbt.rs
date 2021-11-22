use std::cmp::min;
use std::collections::HashMap;
use std::hash::Hash;

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
    state: MlbtTermList,

    local_id: u64,

    wait_list: WaitList,

    mlbt_stat: MlbtStatList,
}


/// Two set of protocol states are maintained: MlbtState and WaitList
/// MlbtState hold the logical protocol state 
/// WaitList stores protocol data with a time limit
/// 
/// They are largely related because protocol data with a time limit are often
/// bonded to a certain protocol state. We will check WaitList in a loop and invoke
/// protocol state shift if any of them is timeout. Thus we can believe the protocol
/// states in other part of the code and do not need to think about timeout issues.


#[derive(Debug, PartialEq, Clone, Copy)]
enum MlbtTerm {
    Idle,                  // uninitialized, will try to join 
    Init(MergeSubTerm),   // initializing, will try to merge
    Wait,                  // initializing, wait for other nodes
    Estb((JoinSubTerm, BalanceSubTerm)),    // running
}


// must in Estb
#[derive(Debug, PartialEq, Clone, Copy)]
enum BalanceSubTerm {
    Idle,
    
    Grant,          // plan to grant a desc, wait for receiver
    GrantCheck,     // agree on grant, wait for grantee to check
    GrantPre,       // agree to recv a desc, wait for grantee check

    Retract,        // plan to retract a desc, wait for receiver
    RetractCheck,   // agree on retract, wait for retracted to check
    RetractPre,     // agree to give away a desc, wait for retracted check
}


#[derive(Debug, PartialEq, Clone, Copy)]
enum JoinSubTerm {
    Idle,               // not engaged in join procedure
    Request,            // has a pending join request
    PreJoin,            // accept a join request, wait for comfirmation
}


#[derive(Debug, PartialEq, Clone, Copy)]
enum MergeSubTerm {
    Idle,       // initial state
    Request,    // request to merge, wait for reply
    PreMerge,  // accept a REQUEST, wait for comfirm
    Check,      // agree on merge, wait for check
}


struct MlbtTermList {
    term_by_root: HashMap<Peer, MlbtTerm>,
}


impl MlbtTermList {

    pub fn new(route_ctl: &RouteTable) -> Self {
        Self {
            term_by_root: route_ctl.get_src_list().iter().fold(
                HashMap::new(),
                |mut init_map, s| {
                    init_map.insert(s.to_owned(), MlbtTerm::Idle);
                    init_map
                }
            )
        }
    }

    pub fn get(&self, root: &Peer) -> Option<&MlbtTerm> {
        self.term_by_root.get(root)
    }

    pub fn get_mut(&mut self, root: &Peer) -> Option<&mut MlbtTerm> {
        self.term_by_root.get_mut(root)
    }

    pub fn set(&mut self, root: &Peer, v: &MlbtTerm) -> Option<MlbtTerm> {
        self.term_by_root.insert(root.to_owned(), v.to_owned())
    }

}


impl RelayCtl for MlbtRelayCtlContext {

    fn new(route_ctl: &RouteTable) -> Self {
        Self {
            local_id: 0,
            
            state: MlbtTermList::new(route_ctl),

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


    // // try to join
    // fn idle_behaviour(&mut self, route_ctl: &mut RouteTable) {
        
    //     // simply try to join src 
    //     if let Some(src) = route_ctl.get_best_src() {
    //         let join_msg = self.join(&src, &src);
    //     }
    // }


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

        self.check_timers(route_ctl);

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


            RelayMsgKind::MERGE_CHECK => {
                let reply = self.merge_ck_cb(route_ctl, sender, &parse_ctl_message);
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

        for src in route_ctl.get_src_list() {
            match self.state.get(&src) {
                Some(MlbtTerm::Idle) => {
                    // try to join at INIT state
                }
    
                Some(MlbtTerm::Init(MergeSubTerm::Idle)) => {
                    // try to merge
                }
    
                Some(MlbtTerm::Estb((JoinSubTerm::Idle, _))) => {
                    // try to rebalance
                }
    
                _ => {
    
                }
            }
        }
        ret
    }

    fn relay_receipt(&mut self, route_ctl: &mut RouteTable, all_success: bool) {
        
        // relay finish time
        // can be used to update self relay time consumption

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
                self.wait_list.check(&root, WaitStateType::JoinWait)
            {
                if let WaitStateData::JoinWait((src, waitfor, _)) = timed_data {
                    self.join_wait_timeout_cb(route_ctl, &src, &waitfor);
                }
                else {
                    unreachable!()
                }
            }


            if let Some(WaitStateData::JoinPre((src, waitfor, _))) = 
                self.wait_list.check(&root, WaitStateType::JoinPre)
            {
                self.join_pre_timeout_cb(route_ctl, &src, &waitfor);   
            }


            if let Some(WaitStateData::MergeWait((src, waitfor, _))) = 
                self.wait_list.check(&root, WaitStateType::MergeWait)
            {
                self.merge_wait_timeout_cb(route_ctl, &src, &waitfor);
            }


            if let Some(WaitStateData::MergePre((src, waitfor, _))) = 
                self.wait_list.check(&root, WaitStateType::MergePre)
            {
                self.merge_pre_timeout_cb(route_ctl, &src, &waitfor);
            }


            if let Some(WaitStateData::MergeCheck((src, merge_target, _, _))) = 
                self.wait_list.check(&root, WaitStateType::MergeCheck)
            {
                self.merge_ck_res_timeout_cb(&src, &merge_target);
            }
    
            

        }
    }


    // form a join request
    fn join(&mut self, target: &Peer, src: &Peer) -> Option<(Peer, RelayCtlMessage)> {
    
        // if self.wait_list.is_waiting(src) {
        //     // todo: rethink join condition
        //     return None;
        // }

        let req_seq = self.seq();

        // log the time a join quest is made
        // todo: figure out send_to success / failed
        self.wait_list.set(
            src,
            WaitStateData::JoinWait((src.to_owned(), target.to_owned(), req_seq))
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
            warn!("MlbtRelayCtlContext::join_cb parse RelayMsgJoin failed: {}",
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
        match self.state.get(&join_msg.src()) {
            
            Some(MlbtTerm::Estb(_)) => {
            
                if self.wait_list.is_waiting(&join_msg.src()) {

                    // waiting for sender, one with bigger peer id accepts
                    // todo!(ack message can be skipped since recv a join req
                    // itself means has send capability and join intention)

                    if let Some(WaitStateData::JoinWait((_, cand, _))) = 
                        self.wait_list.get(&join_msg.src(), WaitStateType::JoinWait) 
                    {
                        if cand == *sender && route_ctl.local_id() > *sender {

                            let ack = msg.accept(self.seq());

                            self.wait_list.clear(&join_msg.src(), WaitStateType::JoinWait);
                            self.wait_list.set(
                                &join_msg.src(),
                                WaitStateData::JoinWait(
                                    (join_msg.src().clone(), sender.clone(), ack.msg_id())
                                )
                            );
                            
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

                self.wait_list.set(
                    &join_msg.src(),
                    WaitStateData::JoinWait(
                        (join_msg.src().clone(), sender.clone(), ack.msg_id())
                    )
                );

                Some((
                    sender.to_owned(),
                    ack
                ))
            }

            Some(MlbtTerm::Idle) | Some(MlbtTerm::Init(_)) | Some(MlbtTerm::Wait) => {
                // not ready to be subscribed, reject
                Some((
                    sender.to_owned(),
                    msg.reject(self.seq())
                ))
            }

            None => {
                warn!("MlbtRelayCtlContext::join_cb join msg refer to an unknown root");
                None
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
        incoming_id: u64, accepted: bool) -> Option<(Peer, RelayCtlMessage)>
    {

        // anyway, the timer should be cleared
        self.wait_list.clear(src, WaitStateType::JoinWait);

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
                    RelayMsgAccept::from_id(incoming_id) // safe unwrap
                )
            ));
        }

        // rejected
        // todo: rejected cb
        None
    }


    fn join_wait_timeout_cb(&mut self, route_ctl: &mut RouteTable, src: &Peer, waitfor: &Peer) -> Option<(Peer, RelayCtlMessage)> {
        // todo
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
         waitfor: &Peer, incoming_id: u64, accepted: bool)
    {

        // anyway, the timer should be cleared
        self.wait_list.clear(src, WaitStateType::JoinPre);

        if accepted {
            route_ctl.insert_relay(src, waitfor);
            return;
        }

        // rejected
        // todo rejected cb
    }


    fn join_pre_timeout_cb (&mut self, route_ctl: &mut RouteTable, src: &Peer,
        waitfor: &Peer)
    {
        //todo
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
                
                WaitStateData::JoinWait((src, waitfor, id)) => {
                    let reply = self.join_wait_cb(
                        route_ctl, &src, &waitfor, incoming_msg_id, pos);
                    
                    if reply.is_some() {
                        ret.push(reply.unwrap());
                    }
                    
                    return ret;
                }

                WaitStateData::JoinPre((src, waitfor, id)) => {
                    self.join_pre_cb(route_ctl, &src, &waitfor, 
                        incoming_msg_id, pos);
                    return ret;
                }

                WaitStateData::MergeWait((src, waitfor, id)) => {
                    let reply = self.merge_wait_cb(
                        route_ctl, &src, &waitfor, incoming_msg_id, pos);
                    
                    if reply.is_some() {
                        ret.push(reply.unwrap());
                    }
                    return ret;
                }

                WaitStateData::MergePre((src, waitfor, id)) => {
                    self.merge_pre_cb(route_ctl, &src, &waitfor, 
                        incoming_msg_id, pos);
                    return ret;
                }

                WaitStateData::MergeCheck((src, merge_target, confirm_id, id)) => {
                    self.merge_ck_res_cb(route_ctl, &src, &merge_target, confirm_id, id, pos);
                    return ret;
                }
            }
        }
        
        warn!("MlbtRelayCtlContext::accept_dispatcher \
            Unmatched accept message: {:?}", msg);
        return ret;                
    }


    // require to merge
    // do not check and updata state, do it before calling
    // but it checks the wait_list, is this a bad design?
    fn merge(&mut self, src: &Peer, target: &Peer) -> Option<(Peer, RelayCtlMessage)> {

        // // todo
        // if self.wait_list.is_waiting(src) {
        //     return None;
        // }

        self.state.set(src, &MlbtTerm::Init(MergeSubTerm::Request));

        let msg_seq = self.seq();

        // set timer
        self.wait_list.set(src, 
            WaitStateData::MergeWait((src.to_owned(), target.to_owned(), msg_seq))
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
            warn!("MlbtRelayCtlContext::merge_cb merge_thr at root {} is unknown", &merge_msg.src());
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

        let crt_root_state = self.state.get_mut(&merge_msg.src());
        // can accept, check self state
        match crt_root_state {

            // node is trying to merge, will accept and pre for comfirmation
            Some(MlbtTerm::Init(MergeSubTerm::Idle)) => {

                let handle = crt_root_state.unwrap();
                *handle = MlbtTerm::Init(MergeSubTerm::PreMerge);

                // todo!
                self.wait_list.set(
                    &merge_msg.src(), 
                    WaitStateData::MergePre((
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

            Some(MlbtTerm::Idle) | Some(MlbtTerm::Estb(_)) | Some(MlbtTerm::Wait)
                | Some(MlbtTerm::Init(_)) => 
            {
                // not in mergeable state, reject

                Some((
                    sender.to_owned(),
                    msg.reject(self.seq())
                ))

            }

            None => {
                warn!("MlbtRelayCtlContext::merge_cb Unknown src");
                None
            }
        }
    }


    // recv a merge request
    fn merge_wait_cb(&mut self, route_ctl: &mut RouteTable, src: &Peer, waitfor: &Peer,
        incoming_id: u64, accepted: bool) -> Option<(Peer, RelayCtlMessage)>
    {

        // anyway, the timer should be cleared
        self.wait_list.clear(src, WaitStateType::MergeWait);

        if accepted {
            
            let ack_seq = self.seq();

            // todo timer
            self.wait_list.set(
                src,
                WaitStateData::MergePre((
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
                    RelayMsgAccept::from_id(incoming_id) // safe unwrap
                )
            ))
        }

        else {
            // rejected
            // todo 
            None
        }
    }


    fn merge_wait_timeout_cb(&mut self, route_ctl: &mut RouteTable, src: &Peer, waitfor: &Peer) 
        -> Option<(Peer, RelayCtlMessage)>
    {
        //todo
        None
    }


    fn merge_pre_cb(&mut self, route_ctl: &mut RouteTable, src: &Peer,
        waitfor: &Peer, incoming_id: u64, accepted: bool)
    {
        
        // clear timer
        self.wait_list.clear(src, WaitStateType::MergePre);

        if accepted {
            // one with larger id become the new root
            if route_ctl.local_id() > *waitfor {
                // todo: whats more?
                route_ctl.insert_front_relay(src, waitfor);
            }
            else {

                // cannot be None if reaches here
                let handle = self.state.get_mut(src).unwrap();
                *handle = MlbtTerm::Wait;

                route_ctl.reg_delegate(src, waitfor);
            }
        }
        else {
            // todo: rejected cb
        }
    }


    fn merge_pre_timeout_cb(&mut self, route_ctl: &mut RouteTable, src: &Peer,
        waitfor: &Peer)
    {
        //todo
    }


    // send a check request to root
    fn merge_ck(&mut self, src: &Peer, target: &Peer, confirm_id: u64, weight: u64) -> Option<(Peer, RelayCtlMessage)> {
        // todo
        // if self.wait_list.is_waiting(src) {
        //     return None;
        // }

        let msg_seq = self.seq();

        // set timer
        self.wait_list.set(
            src,
            WaitStateData::MergeCheck((
                src.to_owned(),
                target.to_owned(),
                confirm_id,
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
    fn merge_ck_cb(&mut self, route_ctl: &mut RouteTable, sender: &Peer, 
        msg: &RelayCtlMessage) -> Option<(Peer, RelayCtlMessage)> 
    {

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

        match self.state.get(&route_ctl.local_id()).unwrap() {
            MlbtTerm::Init(_) => {
                let nw = merge_msg.weight();
                
                // if cw cannot unwrap, either route table is wrong or remote
                // message is wrong, ignore it
                let cw = self.mlbt_stat.relay_inv(&route_ctl.local_id())?;

                if nw > cw {
                    // reject
                    Some((
                        sender.to_owned(),
                        msg.reject(self.seq())
                    ))
                }
                else {
                    // accept
                    Some((
                        sender.to_owned(),
                        msg.accept(self.seq())
                    ))
                }
            }

            _ => {
                // wrong state, ignore it
                None
            }
        }
    }


    fn merge_ck_res_cb(&mut self, route_ctl: &mut RouteTable, src: &Peer, 
        merge_target: &Peer, confirm_id: u64, incoming_id: u64, accepted: bool) -> Option<(Peer, RelayCtlMessage)>
    {
        // clear timer
        self.wait_list.clear(src, WaitStateType::MergeCheck);

        let handle = self.state.get_mut(src).unwrap();

        if accepted {
            // accepted, can merge with merge_target
            *handle = MlbtTerm::Init(MergeSubTerm::Idle);
            let comfirm_msg = RelayCtlMessage::new(
                RelayMsgKind::ACCEPT,
                self.seq(), 
                RelayMsgAccept::from_id(confirm_id)
            );
            Some((merge_target.to_owned(), comfirm_msg))
        }
        else {
            // give up merge process
            *handle = MlbtTerm::Init(MergeSubTerm::Idle);
            
            // todo: update merge metric 

            let comfirm_msg = RelayCtlMessage::new(
                RelayMsgKind::REJECT,
                self.seq(), 
                RelayMsgAccept::from_id(confirm_id)
            );
            Some((merge_target.to_owned(), comfirm_msg))
        }
    }


    fn merge_ck_res_timeout_cb(&mut self, src: &Peer,
        merge_target: &Peer)
    {
        // todo
        debug!("MlbtRelayCtlContext::merge_ck_res_cb Do not receive reply to \
                MERGE_CHECK in time.");
    }


    fn check_balance(&mut self) {
        todo!()
    }
    
    // check grant condition on each given root
    fn check_grant(&mut self, src: &Peer, route_ctl: &mut RouteTable) {

        todo!();
        

        for desc in route_ctl.get_relay(src) {
            
        }
    }

    fn check_retract(&mut self, src: &Peer, route_ctl: &mut RouteTable) {
        todo!()
    }

}