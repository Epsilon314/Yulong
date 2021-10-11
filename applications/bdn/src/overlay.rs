use futures::{AsyncWriteExt};
use yulong::utils::{
    bidirct_hashmap::BidirctHashmap,
};
use yulong_network::{identity::{Me, Peer, crypto::AsBytes}, transport::Transport};
use std::{collections::HashMap, net::{
        SocketAddr, IpAddr, Ipv4Addr,
    }, sync::mpsc};
use log::{warn, info, debug};
use async_std::{io::BufReader};
use crate::{message, route::Route};
use crate::common::{SocketAddrBi, DEFAULT_BDN_PORT, MSG_MAXLEN, MessageWithIp};


pub struct BDN<T: Transport> {

    local_identity: Me,

    // peer's listening socket
    address_book: BidirctHashmap<Peer, SocketAddrBi>,

    w_stream: HashMap<Peer, <T as Transport>::Stream>,

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

    pub async fn send_to(&mut self, dst: &Peer, msg: &mut message::OverlayMessage) {
        
        msg.set_from(&self.local_identity.peer);

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


    pub async fn broadcast(&mut self, src: &Peer, msg: &mut message::OverlayMessage) {

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

        // create a message reader with an inner buffered reader
        let mut msg_reader = message::MessageReader::<T>::new(
            BufReader::with_capacity(MSG_MAXLEN, s)
        );

        loop {

            // read one message at a time
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

            let raw_msg = msg.unwrap().payload();

            debug!("BDN::handle_ingress: read {} bytes", raw_msg.len());
            let send_res = sender.send((remote_sock ,raw_msg));
            if send_res.is_err() {
                warn!("BDN::handle_ingress: {}", send_res.unwrap_err());
            }
        }
    }
}

impl<T: Transport> Iterator for BDN<T> {
    type Item = MessageWithIp;

    fn next(&mut self) -> Option<Self::Item> {
        
        let msg = self.msg_receiver.recv();
        
        if msg.is_err() {
            warn!("BDN::next error {}", msg.unwrap_err());
            return None;
        }

        // relay stuff

        // get source from message

        let (from, dec_msg) = msg.clone().unwrap();
        if let Some(peer) = self.address_book.get_by_value(&from) {
            // let relay_list = self.route.get_relay(, peer);
        }
        else {
            // log unknown 
        }

        Some(msg.unwrap())
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
    use std::net::SocketAddr;

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