use futures::{AsyncWriteExt};

use yulong::utils::{
    bidirct_hashmap::BidirctHashmap,
};

use yulong_network::{
    identity::Me,
    identity::Peer, 
    identity::crypto::AsBytes,
    transport::Transport,
};

use std::{
    collections::HashMap,
    net::{SocketAddr, IpAddr, Ipv4Addr,},
    sync::mpsc
};

use log::{warn, info, debug};
use async_std::{io::BufReader};

use crate::{message, msg_header::MsgTypeKind, route::Route};
use crate::common::{SocketAddrBi, MessageWithIp};
use crate::configs::{DEFAULT_BDN_PORT, MSG_MAXLEN};

pub struct BDN<T: Transport> {

    local_identity: Me,

    // peer's listening socket
    address_book: BidirctHashmap<Peer, SocketAddrBi>,

    w_stream: HashMap<Peer, <T as Transport>::Stream>,

    #[allow(dead_code)]
    msg_sender: mpsc::Sender<MessageWithIp>,
    msg_receiver: mpsc::Receiver<MessageWithIp>,

    route: Route<T>,
}


impl<T: Transport> BDN<T> {
    
    pub fn new() -> Self {

        let (sender, receiver) = 
            mpsc::channel::<MessageWithIp>();

        Self {
            local_identity: Me::new(),
            
            address_book: BidirctHashmap::<Peer, SocketAddrBi>::new(),
            
            w_stream: HashMap::<Peer, <T as Transport>::Stream>::new(),
            msg_sender: sender,
            msg_receiver: receiver,
            route: Route::new(),
        }
    }


    // accept incoming connections and spawn tasks to serve them
    pub async fn listen(listen_port: u16, msg_sender: mpsc::Sender<MessageWithIp>) {

        let mut listener = T::listen(
            &SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), listen_port)
        ).await.ok().unwrap();

        info!("BDN listening on {}", listen_port);

        while match T::accept(&mut listener).await {
            
            Ok(istream) => {
                // a new incoming connection

                let ip = istream.remote_addr.ip();
                let incoming_port = istream.remote_addr.port();
                let socket = SocketAddrBi::new(ip, DEFAULT_BDN_PORT, Some(incoming_port));

                let sender = msg_sender.clone();
                
                async_std::task::spawn(
                    async move {
                        Self::handle_ingress(istream.stream, sender, socket).await;
                });

                true
            }

            Err(error) => {
                warn!("BDN::run: {}", error);
                true
            }

        } {}
    }


    pub async fn connect(&mut self) {
        for (peer, addr) in self.address_book.iter() {
            if self.w_stream.contains_key(peer) {
                continue;
            }

            let con_socket = SocketAddr::new(addr.ip(), addr.listen_port());

            match T::connect(&con_socket).await {
                Ok(stream) => {
                    self.w_stream.insert(
                        peer.clone(),
                        stream
                    );
                }
                Err(error) => {
                    warn!("BDN::connect: {}", error);
                }
            }
        }
    }


    pub async fn send_to(&mut self, dst: &Peer, msg: &message::OverlayMessage) {

        let msg_bytes = msg.into_bytes();

        if msg_bytes.is_err() {
            warn!("BDN::send_to: {}", msg_bytes.unwrap_err());
            return;
        }
        let msg_bytes = msg_bytes.unwrap();

        // send through an existing stream

        if let Some(wstream) = self.w_stream.get_mut(&dst) {
            // use existing connection to dst
            wstream.write_all(&msg_bytes).await.unwrap_or_else(|err| {
                warn!("BDN::send_to write error: {}", err);
            });
            return;
        }

        // no established stream, connect and send
        let addr = self.address_book.get_by_key(&dst);
        
        if addr.is_none() {
            warn!("BDN::send_to unknown dst: {:?}", &dst.get_id());
            return;
        }

        let addr = addr.unwrap();
        let con_socket = SocketAddr::new(addr.ip(), addr.listen_port());

        match T::connect(&con_socket).await {
            
            Ok(mut wstream) => {
                wstream.write_all(&msg_bytes).await
                    .unwrap_or_else( |err| {
                        warn!("BDN::send_to write error: {}", err);
                    });
                self.w_stream.insert(dst.clone(), wstream);
            }

            Err(error) => {
                warn!("BDN::send_to encounter an error when connecting {}. Error: {}", con_socket, error);
            }
        }
    }


    pub async fn broadcast(&mut self, src: &Peer, msg: &message::OverlayMessage) {

        let relay_list = self.route.get_relay(&src, &self.local_identity.peer);

        for peer in relay_list {
            self.send_to(&peer, msg).await
        }
    }


    pub async fn send_to_indirect(&mut self, dst: &Peer, msg: &mut message::OverlayMessage) {
        
        let next = self.route.get_next_hop(&dst);

        if next.is_none() {
            warn!("Send to {} failed: No route.", &dst);
            return;
        }
        self.send_to(&next.unwrap(), msg).await;
    }

    pub async fn handle_ingress(
        s: <T as Transport>::Stream,
        sender: mpsc::Sender<MessageWithIp>,
        remote_sock: SocketAddrBi
    ) {
        
        // incoming stream obviously has an incoming port, safe unwrap
        info!("Start to serve {}{}", remote_sock.ip(), remote_sock.incoming_port().unwrap());

        // create a message reader with an inner buffered reader
        let mut msg_reader = message::MessageReader::<T>::new(
            BufReader::with_capacity(MSG_MAXLEN, s)
        );

        loop {

            // read one message at a time, including deserialization 
            let msg= msg_reader.read_message().await;

            // encounter an ill-formed message
            if msg.is_err() {
                warn!("BDN::handle_ingress: {}", msg.unwrap_err());
                continue;
            }

            // EOF, end this processing task
            let msg = msg.unwrap();
            if msg.is_none() {
                break;
            }

            let overlay_msg = msg.unwrap();

            debug!("BDN::handle_ingress: receive {} bytes payload",
                overlay_msg.payload().len());

            match sender.send((remote_sock ,overlay_msg)) {
                Ok(_) => {}
                Err(error) => {
                    warn!("BDN::handle_ingress: {}", error);
                    // log the error and continue, only EOF will shutdown the listening thread
                }
            };
        }
    }
}

impl<T: Transport> Iterator for BDN<T> {
    type Item = MessageWithIp;

    fn next(&mut self) -> Option<Self::Item> {
        
        // get a message from the receiver queue
        let msg = self.msg_receiver.recv();
        
        // no more messages in the receiver queue
        if msg.is_err() {
            return None;
        }

        // relay messages

        // clone for modification
        let (from, mut incoming_msg) = msg.clone().unwrap();

        // todo: move to a relayer trait & impl it for each relay method

        if incoming_msg.is_relay() {

            let src = &incoming_msg.src();
            incoming_msg.set_from(&self.local_identity.peer);
    
            if let Some(peer) = self.address_book.get_by_value(&from) {
                let relay_list = self.route.get_relay(src, peer);
                for next_node in relay_list {
    
                    // send it in sequence
                    async_std::task::block_on(
                        self.send_to(&next_node, &incoming_msg));
                }
            }
            else {
                warn!("BDN::next unknown upstream node: {}", &from)
            }

        }

        // dispatch messages
        // payload_msg is returned to the caller
        // error_msg is ignored

        match incoming_msg.get_type() {
            
            Ok(MsgTypeKind::PAYLOAD_MSG) => {
                Some(msg.unwrap())
            }

            Ok(MsgTypeKind::ROUTE_MSG) => {
                //pass it to route module
                let reply_list = self.route.handle_route_message(&incoming_msg);
                None
            }

            Ok(MsgTypeKind::NET_MEASURE_MSG) => {
                // Todo net measure 
                None
            }
            
            Err(error) => {
                // cannot parse msg_type from msg header, skip this message
                warn!("BDN::next bad msg_type {}", error);
                None
            }
        }
    }
}


#[cfg(test)]
mod test {
    use std::{net::{IpAddr}, str::FromStr};

    use crate::{message, overlay::SocketAddrBi};

    use super::BDN;
    use yulong_tcp::TcpContext;
    use yulong_quic::QuicContext;

    
    use async_std::{self};
    use yulong::log;
    use yulong_network::identity::Peer;

    #[async_std::test]
    async fn bdn_1() {

        log::setup_logger("bdn_test1").unwrap();
        
        let mut bdn = BDN::<QuicContext>::new();

        let peer = Peer::from_random();
        let socket = SocketAddrBi::new(IpAddr::from_str("127.0.0.1").unwrap(), 9002_u16, None);

        println!("New BDN client at: {:?}", bdn.local_identity.peer.get_id());
        bdn.address_book.insert(
            peer.clone(),
            socket,
        );

        let payload = [42_u8; 1900];
        
        let server = async_std::task::spawn(
            BDN::<QuicContext>::listen(9001, bdn.msg_sender.clone())
        );

        let mut m1 = message::OverlayMessage::new(
            0, &peer, &peer, &peer, &[1,2,3]);

        let mut m2 = message::OverlayMessage::new(
            0, &peer, &peer, &peer, &[1,2,3,4,5,6]);

        let mut m3 = message::OverlayMessage::new(
            0, &peer, &peer, &peer, &payload);

        let mut m4 = message::OverlayMessage::new(
            0, &peer, &peer, &peer, &[1,2,3]);

        bdn.connect().await;
        bdn.send_to(&peer, &mut m1).await;
        bdn.send_to(&peer, &mut m2).await;
        bdn.send_to(&peer, &mut m3).await;
        bdn.send_to(&peer, &mut m4).await;

        server.await;
    }

    #[async_std::test]
    async fn bdn_2() {

        log::setup_logger("bdn_test2").unwrap();
        
        let mut bdn = BDN::<QuicContext>::new();

        let peer = Peer::from_random();
        let socket = SocketAddrBi::new(IpAddr::from_str("127.0.0.1").unwrap(), 9001_u16, None);

        println!("New BDN client at: {:?}", bdn.local_identity.peer.get_id());
        bdn.address_book.insert(
            peer.clone(),
            socket,
        );

        let payload = [42_u8; 1900];
        
        let server = async_std::task::spawn(
            BDN::<QuicContext>::listen(9002, bdn.msg_sender.clone())
        );
        
        let mut m1 = message::OverlayMessage::new(
            0, &peer, &peer, &peer, &[1,2,3]);

        let mut m2 = message::OverlayMessage::new(
            0, &peer, &peer, &peer, &[1,2,3,4,5,6]);

        let mut m3 = message::OverlayMessage::new(
            0, &peer, &peer, &peer, &payload);

        let mut m4 = message::OverlayMessage::new(
            0, &peer, &peer, &peer, &[1,2,3]);

        bdn.connect().await;
        bdn.send_to(&peer, &mut m1).await;
        bdn.send_to(&peer, &mut m2).await;
        bdn.send_to(&peer, &mut m3).await;
        bdn.send_to(&peer, &mut m4).await;

        server.await;
    }

}