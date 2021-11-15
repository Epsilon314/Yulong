use std::vec;

use crate::message::{
    PbftMessage,
    PbftMsgKind
};
use crate::quorum::{QuorumCollector, VoteBox, VoteResult};

use log::{debug, warn};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};

use yulong_network::identity::Peer;
use yulong_network::identity::Me;
use yulong_network::identity::crypto::GenericSigner;

use yulong::utils::AsBytes;
use yulong::utils::CasualTimer;

use yulong_bdn::overlay::BDN;
use yulong_bdn::message::OverlayMessage;
use yulong_bdn::msg_header::{MsgHeader, MsgTypeKind, RelayMethodKind};
use yulong_bdn::route_inner::impls::mlbt::MlbtRelayCtlContext;
use yulong_bdn::route::AppLayerRouteUser;

use yulong_tcp::TcpContext;


#[derive(FromPrimitive, ToPrimitive, PartialEq, Debug, Clone, Copy)]
enum PbftStage {
    IDLE = 8,

    REQUEST = 0,
    PRE_PREPARE = 1,
    PREPARE = 2,
    COMMIT = 3,
    REPLY = 4,

    BLAME = 5,
    NEW_EPOCH = 6,
    CONFIRM_EPOCH = 7,
}

pub struct PbftContext<S: GenericSigner> {
    network_handle: BDN<TcpContext, MlbtRelayCtlContext>,
    signer: S,

    local_id: Me,

    seq: u32,

    round: u32,
    stage: PbftStage,

    total_node: u32,
    quorum: u32,

    primary_id: Peer,
    prepare_vote_box: VoteBox,
    commit_vote_box: VoteBox,
    blame_vote_box: VoteBox,
    new_epoch_vote_box: VoteBox,
    reply_vote_boxes: Vec<VoteBox>,

    preprepare_timer: CasualTimer,
    prepare_timer: CasualTimer,



}


impl<S: GenericSigner> PbftContext<S> {

    // call this every tick
    pub fn heartbeat(&mut self) {

        // todo check timeout

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
        self.preprepare_timer.reset();
        self.prepare_timer.reset();
    }


    fn pbft_msg_cb(&mut self, msg: PbftMessage) {
        match msg.msg_type() {
            PbftMsgKind::REQUEST => self.request_cb(msg),
            PbftMsgKind::PRE_PREPARE => todo!(),
            PbftMsgKind::PREPARE => todo!(),
            PbftMsgKind::COMMIT => todo!(),
            PbftMsgKind::REPLY => todo!(),
            PbftMsgKind::BLAME => todo!(),
            PbftMsgKind::NEW_EPOCH => todo!(),
            PbftMsgKind::CONFIRM_EPOCH => todo!(),
        }
    }


    fn request_cb(&mut self, msg: PbftMessage) {
        if *self.local_id.peer() == self.primary_id && self.stage == PbftStage::IDLE {
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
        // todo
    }


    fn request_timeout_cb(&mut self) {
        todo!()
    }


    fn preprepare_timeout_cb(&mut self) {
        todo!()
    }


    fn prepare_timeout_cb(&mut self) {
        todo!()
    }


    fn discard_round(&mut self) {
        todo!()
    }


    fn new_round(&mut self) {
        todo!()
    }


    fn send_reply(&mut self) {
        todo!()
    }


    fn send_new_epoch(&mut self) {
        todo!()
    }


    fn is_backup_primary(&mut self) -> bool {
        todo!()
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

}