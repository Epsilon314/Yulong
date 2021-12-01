use std::vec;
use rand;
use crate::message::{
    PbftMessage,
    PbftMsgKind
};
use crate::participants::{Participants, ParticipantsStore};
use crate::quorum::{QuorumCollector, VoteBox, VoteBoxes, VoteResult};
use crate::store::{Store, StoreService};

use log::{debug, info, warn};

use yulong_network::identity::Peer;
use yulong_network::identity::Me;
use yulong_network::identity::crypto::GenericSigner;

use yulong::utils::AsBytes;
use yulong::utils::CasualTimer;

use yulong_bdn::overlay::BDN;
use yulong_bdn::message::OverlayMessage;
use yulong_bdn::msg_header::{MsgHeader, MsgTypeKind, RelayMethodKind};
use yulong_bdn::route::AppLayerRouteUser;
use yulong_network::transport::Transport;
use yulong_bdn::route_inner::RelayCtl;



#[derive(PartialEq, Debug, Clone, Copy)]
enum PbftStage {
    IDLE,

    REQUEST,
    PRE_PREPARE,
    PREPARE,
    COMMIT,
    REPLY,

    BLAME,
    NEW_EPOCH,
    CONFIRM_EPOCH,
}

const PAYLOAD_MAX: usize = 500; // bytes

pub struct PbftContext<S, T, R> 
    where
        S: GenericSigner,
        T: Transport,
        R: RelayCtl
{
    network_handle: BDN<T, R>,
    signer: S,

    local_id: Me,

    seq: u32,

    round: u32,
    stage: PbftStage,

    total_node: u32,
    total_node_set: Participants,

    commit_log: Store,

    quorum_size: u32,

    primary_id: Peer,

    prepare_vote_box: VoteBox,
    commit_vote_box: VoteBox,
    blame_vote_box: VoteBox,
    new_epoch_vote_box: VoteBox,
    reply_vote_boxes: VoteBoxes,

    request_timer: CasualTimer,
    preprepare_timer: CasualTimer,
    prepare_timer: CasualTimer,

    test: bool,

}


impl<S, T, R> PbftContext<S, T, R> 
    where
        S: GenericSigner,
        T: Transport,
        R: RelayCtl
{

    // call this every tick
    pub fn heartbeat(&mut self) {

        // check timeout
        self.check_all_timer();     

        let msg = self.network_handle.next();
        if let Some(msg) = msg {
            let raw_payload = msg.payload();
            // todo

            match PbftMessage::from_bytes(&raw_payload) {
                Ok(pbft_msg) => {
                    self.pbft_msg_cb(pbft_msg)
                }

                Err(error) => {
                    warn!("PbftContext::heartbeat Decode msg error: {}", error);
                }
            }
        }
    }


    fn reset_all_timer(&mut self) {
        self.request_timer.reset();
        self.preprepare_timer.reset();
        self.prepare_timer.reset();
    }


    fn check_all_timer(&mut self) {
        
        if self.request_timer.is_timeout() {
            self.request_timer.reset();
            self.request_timeout_cb();
        }

        if self.preprepare_timer.is_timeout() {
            self.preprepare_timer.reset();
            self.preprepare_timeout_cb();
        }

        if self.prepare_timer.is_timeout() {
            self.prepare_timer.reset();
            self.prepare_timeout_cb();
        }

    }


    fn pbft_msg_cb(&mut self, mut msg: PbftMessage) {

        let peer_id = msg.signer_id_mut();
        self.total_node_set.query_pk(peer_id);

        if !msg.verify() {
            warn!("PbftContext::pbft_msg_cb fail to verify signature.");
            // return;
        }

        match msg.msg_type() {

            PbftMsgKind::REQUEST => self.request_cb(msg),
            
            PbftMsgKind::PRE_PREPARE => self.prepare_cb(msg),
            
            PbftMsgKind::PREPARE => self.prepare_cb(msg),
            
            PbftMsgKind::COMMIT => self.commit_cb(msg),
            
            PbftMsgKind::REPLY => self.replay_cb(msg),
            
            PbftMsgKind::BLAME => self.blame_cb(msg),
            
            PbftMsgKind::NEW_EPOCH => self.new_epoch_cb(msg),
            
            PbftMsgKind::CONFIRM_EPOCH => self.confirm_epoch_cb(msg),
        
        }
    }


    fn request_cb(&mut self, msg: PbftMessage) {
        if *self.local_id.peer() == self.primary_id && self.stage == PbftStage::IDLE {

            self.commit_log.pending(self.round, msg.payload());

            let mut reply_msg = PbftMessage::new(
                self.round,
                self.seq(),
                PbftMsgKind::PRE_PREPARE,
                self.local_id.peer().clone(),
                msg.payload().to_vec()
            );

            reply_msg.sign(
                &self.signer, 
                self.local_id.private_key(), 
                self.local_id.peer().pubkey()
            );

            async_std::task::block_on(
                self.broadcast(reply_msg)
            );
            
            self.stage = PbftStage::PRE_PREPARE;

            self.reset_all_timer();
            self.preprepare_timer.set_now();
        }
    }


    fn preprepare_cb(&mut self, msg: PbftMessage) {
        if *msg.signer_id() == self.primary_id {
            if msg.round() == self.round {
                if self.stage == PbftStage::IDLE || 
                    self.stage == PbftStage::REQUEST 
                {
                    // everything is right

                    self.commit_log.pending(self.round, msg.payload());

                    // do not need to include the payload since it has been
                    //  broadcasted with PRE_PREPARE msg
                    let mut reply_msg = PbftMessage::new(
                        self.round,
                        self.seq(),
                        PbftMsgKind::PREPARE,
                        self.local_id.peer().clone(),
                        vec![]
                    );

                    reply_msg.sign(
                        &self.signer, 
                        self.local_id.private_key(), 
                        self.local_id.peer().pubkey()
                    );
                    
                    async_std::task::block_on(
                        self.broadcast(reply_msg)
                    );
                    
                    self.stage = PbftStage::PRE_PREPARE;

                    self.reset_all_timer();
                    self.preprepare_timer.set_now();

                    // in case message received is quite out-of-order
                    self.prepared(false);
                }
                else {
                    debug!("PbftContext::preprepareCb duplicated preprepare");
                }
            }
            else if msg.round() > self.round {
                // perhaps lag back
                // believe the primary

                // give up stale states
                self.discard_round();

                self.round = msg.round();

                let mut reply_msg = PbftMessage::new(
                    self.round,
                    self.seq(),
                    PbftMsgKind::PREPARE,
                    self.local_id.peer().clone(),
                    vec![]
                );

                reply_msg.sign(
                    &self.signer, 
                    self.local_id.private_key(), 
                    self.local_id.peer().pubkey()
                );
                
                async_std::task::block_on(
                    self.broadcast(reply_msg)
                );

                self.stage = PbftStage::PRE_PREPARE;

                self.reset_all_timer();
                self.preprepare_timer.set_now();

                // in case message received is quite out-of-order
                self.prepared(false);
            }
            else {
                // msg round < node round
                // either conflict or rather old msg
                // ignore it
                debug!("PbftContext::preprepareCb stale preprepare msg");
            }
        }
    }


    fn prepared(&mut self, shortcut: bool) {
        if shortcut || (self.prepare_vote_box.result() == VoteResult::PASS &&
            self.stage == PbftStage::PRE_PREPARE)
        {
            let mut reply_msg = PbftMessage::new(
                self.round,
                self.seq(),
                PbftMsgKind::COMMIT,
                self.local_id.peer().clone(),
                vec![]
            );

            reply_msg.sign(
                &self.signer, 
                self.local_id.private_key(), 
                self.local_id.peer().pubkey()
            );

            async_std::task::block_on(
                self.broadcast(reply_msg)
            );

            self.stage = PbftStage::PREPARE;

            self.reset_all_timer();
            self.prepare_timer.set_now();

            self.committed();
        }
    }


    fn prepare_cb(&mut self, msg: PbftMessage) {
        if msg.round() == self.round {
            self.prepare_vote_box.vote(msg.signer_id(), true);
            if self.prepare_vote_box.result() == VoteResult::PASS &&
                self.stage == PbftStage::PRE_PREPARE 
            {
                self.prepared(true);
            }
        }
    }


    fn committed(&mut self) {
        if self.commit_vote_box.result() == VoteResult::PASS && self.stage == PbftStage::PREPARE {
            self.stage = PbftStage::COMMIT;
            self.reset_all_timer();

            self.commit_log.commit(self.round);

            if *self.local_id.peer() != self.primary_id {
                self.send_reply();
                self.new_round();
            }
        }
    }


    fn commit_cb(&mut self, msg: PbftMessage) {
        if msg.round() == self.round {
            self.commit_vote_box.vote(msg.signer_id(), true);
            if self.commit_vote_box.result() == VoteResult::PASS && self.stage == PbftStage::PREPARE {
                self.committed();
            }
        }
    }


    fn blame_cb(&mut self, msg: PbftMessage) {
        // todo: view change is not fully implemented
        
        if msg.round() == self.round {
            self.blame_vote_box.vote(msg.signer_id(), true);
            if self.blame_vote_box.result() == VoteResult::PASS {
                if self.is_backup_primary() {
                    // claim to be the new primary
                    self.stage = PbftStage::NEW_EPOCH;
                    self.send_new_epoch();
                }
                else {
                    // aware of a epoch-changing event
                    // todo timeout
                    self.stage = PbftStage::NEW_EPOCH;
                }
            }
        }
    }


    fn replay_cb(&mut self, msg: PbftMessage) {
        
        self.reply_vote_boxes.vote(msg.signer_id(), true, msg.round());
        if self.reply_vote_boxes.count_pos(msg.round()) == self.total_node - 1 {
            // all committed
            info!("All Committed");
            info!("round: {}", msg.round());
        }

        if *self.local_id.peer() == self.primary_id && msg.round() == self.round {
            if self.reply_vote_boxes.result(self.round) == VoteResult::PASS {
                info!("Majority Committed");
                info!("round: {}", self.round);

                self.new_round();

                if self.test {
                    self.send_request(&Self::test_msg());
                }
            }
        }

    }


    fn new_epoch_cb(&mut self, msg: PbftMessage) {
        if msg.round() == self.round && *msg.signer_id() == self.get_backup_primary() {
            let mut reply_msg = PbftMessage::new(
                self.round,
                self.seq(),
                PbftMsgKind::CONFIRM_EPOCH,
                self.local_id.peer().clone(),
                vec![]
            );

            reply_msg.sign(
                &self.signer, 
                self.local_id.private_key(), 
                self.local_id.peer().pubkey()
            );

            self.discard_round();
            self.primary_id = msg.signer_id().to_owned();

            async_std::task::block_on(
                self.send_to_primary(reply_msg)
            );
        }
    }


    fn confirm_epoch_cb(&mut self, msg: PbftMessage) {
        self.new_epoch_vote_box.vote(msg.signer_id(), true);
        if self.new_epoch_vote_box.result() == VoteResult::PASS {
            self.discard_round();
            self.primary_id = msg.signer_id().to_owned();

            
            let mut payload: Vec<u8>;
            
            let saved_req = self.commit_log.get_pending(self.round);
            if saved_req.is_some() {
                payload = saved_req.unwrap().to_vec();
            }
            else {
                // todo: resume the request
                // the request is send to original primary
                // it may or maybe be shared through preprepare
                // therefore we may need to ak for the request from its original proposal

                // use random bytes as a temp placeholder
                payload = Self::test_msg();
            }

            self.send_request(&payload);
        }
    }


    fn request_timeout_cb(&mut self) {
        self.send_blame();
    }


    fn preprepare_timeout_cb(&mut self) {
        self.send_blame();
    }


    fn prepare_timeout_cb(&mut self) {
        self.send_blame();
    }


    fn discard_round(&mut self) {
        self.stage = PbftStage::IDLE;

        self.prepare_vote_box.reset();
        self.commit_vote_box.reset();
        self.new_epoch_vote_box.reset();

        // todo clear reply vote for this round
    }


    fn new_round(&mut self) {
        
        self.round += 1;
        self.stage = PbftStage::IDLE;

        self.prepare_vote_box.reset();
        self.commit_vote_box.reset();
        self.blame_vote_box.reset();
        self.new_epoch_vote_box.reset();
    }


    pub fn send_request(&mut self, payload: &[u8]) {
        let mut msg = PbftMessage::new(
            self.round,
            self.seq(),
            PbftMsgKind::REQUEST,
            self.local_id.peer().clone(),
            payload.to_vec()
        );

        msg.sign(
            &self.signer, 
            self.local_id.private_key(), 
            self.local_id.peer().pubkey()
        );

        async_std::task::block_on(
            self.send_to_primary(msg)
        );

        self.stage = PbftStage::REQUEST;
        self.commit_log.pending(self.round, payload);

        self.reset_all_timer();
        self.request_timer.set_now();
    }


    fn send_reply(&mut self) {
        if self.stage == PbftStage::COMMIT {

            let mut msg = PbftMessage::new(
                self.round,
                self.seq(),
                PbftMsgKind::REPLY,
                self.local_id.peer().clone(),
                vec![]
            );
    
            msg.sign(
                &self.signer, 
                self.local_id.private_key(), 
                self.local_id.peer().pubkey()
            );

            async_std::task::block_on(
                self.send_to_primary(msg)
            );
        }
    }


    fn send_blame(&mut self) {
        let mut msg = PbftMessage::new(
            self.round,
            self.seq(),
            PbftMsgKind::BLAME,
            self.local_id.peer().clone(),
            vec![]
        );

        msg.sign(
            &self.signer, 
            self.local_id.private_key(), 
            self.local_id.peer().pubkey()
        );

        async_std::task::block_on(
            self.broadcast(msg)
        );
    }


    fn send_new_epoch(&mut self) {
        // todo: get a proof that the leader is faulty, or gather enough blames

        if self.is_backup_primary() {

            let mut msg = PbftMessage::new(
                self.round,
                self.seq(),
                PbftMsgKind::NEW_EPOCH,
                self.local_id.peer().clone(),
                vec![]
            );
    
            msg.sign(
                &self.signer, 
                self.local_id.private_key(), 
                self.local_id.peer().pubkey()
            );

            async_std::task::block_on(
                self.broadcast(msg)
            );
        }

    }


    fn is_backup_primary(&mut self) -> bool {
        let backup_id = self.round % self.total_node;
        backup_id == self.total_node_set.get_idx(self.local_id.peer()).unwrap()
    }

    fn get_backup_primary(&mut self) -> Peer {
        // determinately pick a peer from node set using current round
        
        let backup_id = self.round % self.total_node;
        self.total_node_set.nth(backup_id).unwrap().to_owned()
    }

    async fn broadcast(&mut self, msg: PbftMessage) {
        if let Ok(msg_buf) = msg.into_bytes() {
            let header = MsgHeader::build(
                MsgTypeKind::PAYLOAD_MSG,
                true,
                RelayMethodKind::LOOKUP_TABLE_1,
                1,
                20
            ).unwrap();
    
            let mut overlay_msg = OverlayMessage::new(
                header,
                self.local_id.peer(),
                self.local_id.peer(),
                &Peer::BROADCAST_ID,
                &msg_buf);
    
            self.network_handle.broadcast(&mut overlay_msg).await
        }
        else {
            warn!("PbftContext::broadcast ill-formed msg: {:?}", msg);
        }
        
    }


    async fn send_to_direct(&mut self, msg: PbftMessage, target: &Peer) {
        if let Ok(msg_buf) = msg.into_bytes() {
            let header = MsgHeader::build(
                MsgTypeKind::PAYLOAD_MSG,
                false,
                RelayMethodKind::ALL,
                1,
                20
            ).unwrap();
    
            let mut overlay_msg = OverlayMessage::new(
                header,
                self.local_id.peer(),
                self.local_id.peer(),
                target,
                &msg_buf);
    
            self.network_handle.send_to(target, &mut overlay_msg).await;
        }
        else {
            warn!("PbftContext::send_to_direct ill-formed msg: {:?}", msg);
        }
    }


    async fn send_to_primary(&mut self, msg: PbftMessage) {
        let target = self.primary_id.clone();
        self.send_to_direct(msg, &target).await
    }


    async fn send_to_root(&mut self, msg: PbftMessage) {
        if let Some(target) = self.network_handle.get_best_src() {
            self.send_to_direct(msg, &target).await
        }
        else {
            warn!("PbftContext::send_to_root cannot find a valid root")
        }
    }

    fn seq(&mut self) -> u32 {
        self.seq += 1;
        self.seq
    }


    fn test_msg() -> Vec<u8> {

        // todo: this is not an efficient way to generate bytes...
        // for byte vec of several special size, you can use rand::gen or rand::fill
        // find better ways

        (0..PAYLOAD_MAX).map(|_| { rand::random::<u8>() }).collect()
    }

}