use yulong::utils::AsBytes;
use yulong::utils::CasualTimer;
use yulong_bdn::msg_header::MsgTypeKind;
use yulong_bdn::msg_header::RelayMethodKind;
use yulong_network::identity::{Peer, Me};
use yulong_bdn::overlay::BDN;
use yulong_bdn::msg_header::MsgHeader;
use yulong_bdn::message::OverlayMessage;
use yulong_network::transport::Transport;
use yulong_bdn::route_inner::RelayCtl;

use crate::message::RaftClientRequest;
use crate::message::RaftMessage;
use crate::message::RaftMessageKind;

use crate::config::{
    PAYLOAD_MAX,
    CLIENT_TIMEOUT
};

pub struct RaftClientContext<T: Transport, R: RelayCtl> {
    network_handle: BDN<T, R>,
    local_id: Me,
    seq: u32,

    req_timer: CasualTimer,

    raft_cluster_member: Vec<Peer>
}


impl<T: Transport, R: RelayCtl> RaftClientContext<T, R> {
    
    pub async fn send_request(&mut self, recv_idx: usize, command: &[u8]) {
        
        let req = RaftClientRequest::new(
            command.to_vec(),
        );

        let raft_message = RaftMessage::new(
            RaftMessageKind::ClientRequest(req),
            self.seq()
        );

        if let Ok(msg_buf) = raft_message.into_bytes() {
            let header = MsgHeader::build(
                MsgTypeKind::PAYLOAD_MSG,
                false,
                RelayMethodKind::ALL,
                1,
                20
            ).unwrap();

            let mut message = OverlayMessage::new(
                header,
                self.local_id.peer(),
                self.local_id.peer(),
                &self.raft_cluster_member[recv_idx],
                &msg_buf
            );

            if recv_idx < self.raft_cluster_member.len() {
                self.network_handle.send_to(
                    &self.raft_cluster_member[recv_idx], 
                    &mut message
                ).await
            }
        }
    }


    // if peer is leader, it will log it and confirm,
    // if not, it will info the client who is leader now
    pub fn request_cb(&mut self) {todo!();}


    pub async fn send_test_request(&mut self, recv_idx: usize) {
        self.send_request(recv_idx, &Self::generate_test_request()).await
    }


    fn generate_test_request() -> Vec<u8> {
        (0..PAYLOAD_MAX).map(|_| { rand::random::<u8>() }).collect()
    }


    fn seq(&mut self) -> u32 {
        self.seq += 1;
        self.seq
    }

}