use futures::AsyncWriteExt;

use yulong::utils::{bidirct_hashmap::BidirctHashmap, AsBytes, CasualTimer};

use yulong_network::{identity::Me, identity::Peer, transport::Transport};

use std::{
    collections::HashMap,
    collections::BinaryHeap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::mpsc,
    time::{Duration, SystemTime, Instant},
};

use async_std::io::BufReader;
use log::{debug, info, warn};

use crate::common::{MessageWithIp, SocketAddrBi};
use crate::configs::{DEFAULT_BDN_PORT, MSG_MAXLEN};
use crate::{
    message::{self, OverlayMessage, MsgWithPriority},
    msg_header::MsgTypeKind,
    route::AppLayerRouteUser,
    route::Route,
};

use bytes::{Bytes};

use crate::route_inner::RelayCtl;

// todo: interface is not done, so make pub for now, change it back later
pub struct BDN<T: Transport, R: RelayCtl> {
    pub local_identity: Me,

    // peer's listening socket
    pub address_book: BidirctHashmap<Peer, SocketAddrBi>,

    w_stream: HashMap<Peer, <T as Transport>::Stream>,

    pub msg_sender: mpsc::Sender<MessageWithIp>,
    msg_receiver: mpsc::Receiver<MessageWithIp>,

    use_send_buffer: bool,
    use_zero_copy: bool,

    send_buffer: BinaryHeap::<MsgWithPriority>,

    pub route: Route<R>,

    heartbeat_timer: CasualTimer,
}

impl<T: Transport, R: RelayCtl> BDN<T, R> {

    const HEARTBEAT_INV: u128 = 5000; // ms

    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel::<MessageWithIp>();

        // todo: read from config or generate new
        let id = Me::new();

        let mut timer = CasualTimer::new(Self::HEARTBEAT_INV);
        timer.set_now();

        Self {
            local_identity: id.clone(),

            address_book: BidirctHashmap::<Peer, SocketAddrBi>::new(),

            w_stream: HashMap::<Peer, <T as Transport>::Stream>::new(),
            msg_sender: sender,
            msg_receiver: receiver,
            use_send_buffer: true,
            use_zero_copy: false,
            send_buffer: BinaryHeap::new(),
            route: Route::new(&id.peer()),
            heartbeat_timer: timer,
        }
    }


    // accept incoming connections and spawn tasks to serve them
    pub async fn listen(listen_port: u16, msg_sender: mpsc::Sender<MessageWithIp>) {
        let mut listener = T::listen(&SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
            listen_port,
        ))
        .await
        .ok()
        .unwrap();

        info!("BDN listening on {}", listen_port);

        while match T::accept(&mut listener).await {
            Ok(istream) => {
                // a new incoming connection

                let ip = istream.remote_addr.ip();
                let incoming_port = istream.remote_addr.port();
                let socket = SocketAddrBi::new(ip, DEFAULT_BDN_PORT, Some(incoming_port));

                let sender = msg_sender.clone();

                async_std::task::spawn(async move {
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
                    self.w_stream.insert(peer.clone(), stream);
                }
                Err(error) => {
                    warn!("BDN::connect: {}", error);
                }
            }
        }
    }


    // choose block send or buffered send according to config
    pub fn send_to_auto(&mut self, dst: &Peer, msg: &mut message::OverlayMessage, pri: Option<u32>) {
        if self.use_send_buffer {
            self.send_to_buffered(dst, msg, pri.unwrap())
        }
        else {
            async_std::task::block_on(
                self.send_to(dst, msg)
            );
        }
    }


    pub fn prepare_msg_bytes(msg: &mut message::OverlayMessage) -> Vec<u8> {
        msg.set_timestamp_now();
        msg.into_bytes().unwrap()
    }


    pub async fn send_to_raw_message(&mut self, dst: &Peer, msg_bytes: &[u8]) {
        if let Some(wstream) = self.w_stream.get_mut(&dst) {
            // use existing connection to dst
            wstream.write_all(msg_bytes).await.unwrap_or_else(|err| {
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
                wstream.write_all(msg_bytes).await.unwrap_or_else(|err| {
                    warn!("BDN::send_to write error: {}", err);
                });
                self.w_stream.insert(dst.clone(), wstream);
            }

            Err(error) => {
                warn!(
                    "BDN::send_to encounter an error when connecting {}. Error: {}",
                    con_socket, error
                );
            }
        }
    }


    // async send
    pub async fn send_to(&mut self, dst: &Peer, msg: &mut message::OverlayMessage) {
        // allow fail
        msg.set_timestamp_now();

        // todo: msg is cloned here, which may not be efficient
        let msg_bytes = msg.into_bytes();

        if msg_bytes.is_err() {
            warn!("BDN::send_to: {}", msg_bytes.unwrap_err());
            return;
        }
        let msg_bytes = msg_bytes.unwrap();

        // send through an existing stream

        debug!("BDN::send_to: {} bytes", msg_bytes.len());

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
                wstream.write_all(&msg_bytes).await.unwrap_or_else(|err| {
                    warn!("BDN::send_to write error: {}", err);
                });
                self.w_stream.insert(dst.clone(), wstream);
            }

            Err(error) => {
                warn!(
                    "BDN::send_to encounter an error when connecting {}. Error: {}",
                    con_socket, error
                );
            }
        }
    }


    // buffered send
    pub fn send_to_buffered(&mut self, dst: &Peer, msg: &mut message::OverlayMessage, pri: u32) {
        self.send_buffer.push(MsgWithPriority::new(dst, pri, msg.to_owned()));
    }


    // send one buffered msg
    pub fn send_buffered_once(&mut self) {
        if let Some(send_task) = self.send_buffer.pop() {
            async_std::task::block_on(
                self.send_to(send_task.dst(), &mut send_task.msg().to_owned())
            );
        }
    }


    // flush send buffer
    pub async fn flush_send_buffer(&mut self) {
        while let Some(send_task) = self.send_buffer.pop() {
            self.send_to(send_task.dst(), &mut send_task.msg().to_owned()).await;
        }
    }


    pub async fn relay_on(&mut self, src: &Peer, msg: &mut message::OverlayMessage) {
        let relay_list = self.route.get_relay(&src);

        if self.use_zero_copy {

            
            let raw_msg = Bytes::from(Self::prepare_msg_bytes(msg));

            for peer in relay_list {
                self.send_to_raw_message(&peer, &raw_msg).await
            }
        }
        else {
            for peer in relay_list {
                self.send_to(&peer, msg).await
            }
        }
    }

   
    pub async fn broadcast(&mut self, msg: &mut message::OverlayMessage) {
        // get src and broadcast

        // make sure the relay flag is set
        msg.set_relay(true);

        if let Some(src) = self.route.get_best_src() {
            msg.set_src(&src);

            // following two field is actually not used
            // we set them just for easy to understand and debug
            msg.set_from(self.local_identity.peer());
            msg.set_dst(&Peer::BROADCAST_ID);

            // todo: short path for src is self
            self.send_to(&src, msg).await;
        } else {
            warn!("BDN::broadcast failed because it cannot find a feasible root");
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
        remote_sock: SocketAddrBi,
    ) {
        // incoming stream obviously has an incoming port, safe unwrap
        info!(
            "Start to serve {}{}",
            remote_sock.ip(),
            remote_sock.incoming_port().unwrap()
        );

        // create a message reader with an inner buffered reader
        let mut msg_reader =
            message::MessageReader::<T>::new(BufReader::with_capacity(MSG_MAXLEN, s));

        loop {
            // read one message at a time, including deserialization
            let msg = msg_reader.read_message().await;

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

            debug!(
                "BDN::handle_ingress: receive {} bytes payload",
                overlay_msg.payload().len()
            );

            match sender.send((remote_sock, overlay_msg.clone())) {
                Ok(_) => {}
                Err(error) => {
                    warn!("BDN::handle_ingress: {}", error);
                    // log the error and continue, only EOF will shutdown the listening thread
                }
            };
        }
    }
}

/// Main Event Loop for BDN
/// poll it to activate BDN
///
/// BDN will not actually process incoming messages but only store it until
/// you poll it.
impl<T: Transport, R: RelayCtl> Iterator for BDN<T, R> {
    type Item = OverlayMessage;

    fn next(&mut self) -> Option<Self::Item> {
        // check heartbeat timer
        self.check_heartbeat();

        // get a message from the receiver queue
        let msg = self.msg_receiver.recv();

        // no more messages in the receiver queue
        if msg.is_err() {
            return None;
        }

        // update the identity-address map of incoming node
        // return None if cannot figure out the identity of incoming node
        let incoming_msg = self.from_id_handler(msg.unwrap());
        if incoming_msg.is_none() {
            return None;
        }
        let incoming_msg = incoming_msg.unwrap();


        
        // relay module will take a clone in case it changes the message before relaying it
        self.relay_handler(incoming_msg.clone());

        // todo: flush policy
        async_std::task::block_on(
            self.flush_send_buffer()
        );

        // dispatch messages
        match incoming_msg.get_type() {
            // payload_msg is returned to the caller
            Ok(MsgTypeKind::PAYLOAD_MSG) => Some(incoming_msg),

            Ok(MsgTypeKind::ROUTE_MSG) => {
                // hand it to route module
                self.route_message_dispatcher(incoming_msg);
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

// inner method for main event loop
impl<T: Transport, R: RelayCtl> BDN<T, R> {
    fn from_id_handler(&mut self, msg: (SocketAddrBi, OverlayMessage)) -> Option<OverlayMessage> {
        // clone for modification
        let (from_addr, mut incoming_msg) = msg.to_owned();

        let carried_idt = incoming_msg.from();

        // do not carry an common peer id, use addr to get peer id
        if !carried_idt.common() {
            let stored_peer = self.address_book.get_by_value(&from_addr);
            if stored_peer.is_none() {
                // unknown from, return early
                warn!(
                    "BDN::from_id_handler unknown from peer at address: {}",
                    from_addr
                );
                return None;
            }
            incoming_msg.set_from(stored_peer.unwrap());
        }
        // contains from peer id, believe carried identity (todo: check sign aforehead)
        else {
            // if carried peer is unknown, add it to address book
            // else update it
            let prev_addr = self.address_book.get_by_key(&carried_idt);
            if prev_addr.is_none() {
                self.address_book.insert(&carried_idt, &from_addr);
            } else {
                if *prev_addr.unwrap() != from_addr {
                    info!(
                        "BDN::from_id_handler Peer {} moved from {} to {}",
                        carried_idt,
                        prev_addr.unwrap(),
                        from_addr
                    );
                    self.address_book.update_by_key(&carried_idt, &from_addr);
                }
            }
        }

        Some(incoming_msg)
    }

    fn relay_handler(&mut self, mut incoming_msg: OverlayMessage) {
        // relay messages
        // handle relay messages in sequence
        if incoming_msg.is_relay() {

            let relay_start = Instant::now();

            let src = &incoming_msg.src();
            incoming_msg.set_from(&self.local_identity.peer());

            let relay_list = self.route.get_relay(src);
            for next_node in relay_list {
                // send it in sequence
                async_std::task::block_on(self.send_to(&next_node, &mut incoming_msg));
            }

            // todo: now send_to intercept all errors so we cannot pass on send failures
            // to route module
            self.route.relay_receipt(true);

            debug!("Relay time consumption: {}", relay_start.elapsed().as_millis());
        }
    }

    fn route_message_dispatcher(&mut self, incoming_msg: OverlayMessage) {
        // pass it to route module
        let reply_list = self.route.handle_route_message(&incoming_msg);

        // send reply immediately and in sequence
        for mut msg in reply_list {
            msg.set_src(&self.local_identity.peer());
            msg.set_from(&self.local_identity.peer());

            async_std::task::block_on(self.send_to(&msg.dst(), &mut msg));
        }
    }

    fn check_heartbeat(&mut self) {
        if self.heartbeat_timer.is_timeout() {
            let send_list = self.route.invoke_heartbeat();

            for mut msg in send_list {
                async_std::task::block_on(self.send_to(&msg.dst(), &mut msg));
            }

            self.heartbeat_timer.set_now();
        }
    }
}

// can we just define a deref to route, or in other word is AppLayerRouteUser
// the only trait we want to delegate?
impl<T: Transport, R: RelayCtl> AppLayerRouteUser for BDN<T, R> {
    type Host = Peer;

    fn get_delegate(&self, src: &Self::Host) -> Option<Self::Host> {
        self.route.get_delegate(src)
    }

    fn get_best_src(&self) -> Option<Self::Host> {
        self.route.get_best_src()
    }

    fn get_src_list(&self) -> Vec<Self::Host> {
        self.route.get_src_list()
    }

    fn get_next_hop(&self, dst: &Self::Host) -> Option<Self::Host> {
        self.route.get_next_hop(dst)
    }

    fn get_relay(&self, src: &Self::Host) -> Vec<Self::Host> {
        self.route.get_relay(src)
    }
}

#[cfg(test)]
mod test {
    use std::{net::IpAddr, str::FromStr};

    use crate::{message, overlay::SocketAddrBi, route::AppLayerRouteUser};

    use super::BDN;
    use crate::route_inner::impls::mlbt::MlbtRelayCtlContext;
    use ::log::debug;
    use yulong_quic::QuicContext;
    use yulong_tcp::TcpContext;

    use async_std::{self};
    use yulong::log;
    use yulong_network::identity::Peer;

    #[async_std::test]
    async fn bdn_1() {
        log::setup_logger("bdn_test1").unwrap();

        let mut bdn = BDN::<QuicContext, MlbtRelayCtlContext>::new();

        let peer = Peer::from_random();
        let socket = SocketAddrBi::new(IpAddr::from_str("127.0.0.1").unwrap(), 9002_u16, None);

        println!(
            "New BDN client at: {:?}",
            bdn.local_identity.peer().get_id()
        );
        bdn.address_book.insert(&peer, &socket);

        let payload = [42_u8; 1900];

        let server = async_std::task::spawn(BDN::<QuicContext, MlbtRelayCtlContext>::listen(
            9001,
            bdn.msg_sender.clone(),
        ));

        let mut m1 = message::OverlayMessage::new(
            0b00110000000000000000000000000000,
            &peer,
            &peer,
            &peer,
            &[1, 2, 3],
        );

        let mut m2 = message::OverlayMessage::new(
            0b00110000000000000000000000000000,
            &peer,
            &peer,
            &peer,
            &[1, 2, 3, 4, 5, 6],
        );

        let mut m3 = message::OverlayMessage::new(
            0b00110000000000000000000000000000,
            &peer,
            &peer,
            &peer,
            &payload,
        );

        let mut m4 = message::OverlayMessage::new(
            0b00110000000000000000000000000000,
            &peer,
            &peer,
            &peer,
            &[1, 2, 3],
        );

        bdn.connect().await;
        bdn.send_to(&peer, &mut m1).await;
        bdn.send_to(&peer, &mut m2).await;
        bdn.send_to(&peer, &mut m3).await;
        bdn.send_to(&peer, &mut m4).await;

        loop {
            if let Some(msg) = bdn.next() {
                debug!("recv:\n{}", &msg);
            }
        }
    }

    #[async_std::test]
    async fn bdn_2() {
        log::setup_logger("bdn_test2").unwrap();

        let mut bdn = BDN::<QuicContext, MlbtRelayCtlContext>::new();

        let peer = Peer::from_random();
        let socket = SocketAddrBi::new(IpAddr::from_str("127.0.0.1").unwrap(), 9001_u16, None);

        println!(
            "New BDN client at: {:?}",
            bdn.local_identity.peer().get_id()
        );
        bdn.address_book.insert(&peer, &socket);

        let payload = [42_u8; 1900];

        let server = async_std::task::spawn(BDN::<QuicContext, MlbtRelayCtlContext>::listen(
            9002,
            bdn.msg_sender.clone(),
        ));

        let mut m1 = message::OverlayMessage::new(
            0b00110000000000000000000000000000,
            &peer,
            &peer,
            &peer,
            &[1, 2, 3],
        );

        let mut m2 = message::OverlayMessage::new(
            0b00110000000000000000000000000000,
            &peer,
            &peer,
            &peer,
            &[1, 2, 3, 4, 5, 6],
        );

        let mut m3 = message::OverlayMessage::new(
            0b00110000000000000000000000000000,
            &peer,
            &peer,
            &peer,
            &payload,
        );

        let mut m4 = message::OverlayMessage::new(
            0b00110000000000000000000000000000,
            &peer,
            &peer,
            &peer,
            &[1, 2, 3],
        );

        bdn.connect().await;
        
        bdn.send_to(&peer, &mut m1).await;
        bdn.send_to(&peer, &mut m2).await;
        bdn.send_to(&peer, &mut m3).await;
        bdn.send_to(&peer, &mut m4).await;

        loop {
            if let Some(msg) = bdn.next() {
                debug!("recv:\n{}", &msg);
            }
        }
    }
}
