#[cfg(test)]
mod test {

    use std::{net::IpAddr, str::FromStr};

    use crate::{message, common::SocketAddrBi, route::AppLayerRouteUser};
    use crate::route::AppLayerRouteInner;
    use crate::msg_header::MsgHeader;
    use crate::msg_header::MsgTypeKind;
    use crate::msg_header::RelayMethodKind;

    use crate::overlay::BDN;
    use crate::route_inner::impls::mlbt::MlbtRelayCtlContext;
    use ::log::debug;
    use yulong_quic::QuicContext;
    use yulong_tcp::TcpContext;

    use async_std::{self};
    use yulong::log;
    use yulong_network::identity::Peer;

    #[async_std::test]
    async fn node1() {
        log::setup_logger("bdn_test1").unwrap();

        let mut bdn = BDN::<TcpContext, MlbtRelayCtlContext>::new();
        let self_id = Peer::from_bytes(&[1]);
        bdn.local_identity.set_peer(self_id.clone());

        let peer = Peer::from_bytes(&[2]);
        let socket = SocketAddrBi::new(IpAddr::from_str("127.0.0.1").unwrap(), 9002_u16, None);

        println!(
            "New BDN client at: {:?}",
            bdn.local_identity.peer().get_id()
        );

        bdn.address_book.insert(&peer, &socket);

        let payload = vec![42_u8; 10 * 1024 * 1024 - 200];

        bdn.connect().await;

        let header = MsgHeader::build(
            MsgTypeKind::PAYLOAD_MSG,
            true,
            RelayMethodKind::LOOKUP_TABLE_1,
            1,
            15).unwrap();

        let mut m = message::OverlayMessage::new(
            header,
            &self_id,
            &self_id,
            &Peer::BROADCAST_ID,
            &payload,
        );

        bdn.send_to(&peer, &mut m).await;
    }

    #[async_std::test]
    async fn node2() {

        log::setup_logger("bdn_test2").unwrap();

        let mut bdn = BDN::<TcpContext, MlbtRelayCtlContext>::new();
        let self_id = Peer::from_bytes(&[2]);
        bdn.local_identity.set_peer(self_id);

        let peer3 = Peer::from_bytes(&[3]);
        let socket3 = SocketAddrBi::new(IpAddr::from_str("121.36.95.93").unwrap(), 9003_u16, None);

        let peer4 = Peer::from_bytes(&[4]);
        let socket4 = SocketAddrBi::new(IpAddr::from_str("121.36.95.93").unwrap(), 9004_u16, None);

        let peer5 = Peer::from_bytes(&[5]);
        let socket5 = SocketAddrBi::new(IpAddr::from_str("121.36.95.93").unwrap(), 9005_u16, None);

        let peer6 = Peer::from_bytes(&[6]);
        let socket6 = SocketAddrBi::new(IpAddr::from_str("121.36.95.93").unwrap(), 9006_u16, None);

        let peer7 = Peer::from_bytes(&[7]);
        let socket7 = SocketAddrBi::new(IpAddr::from_str("121.36.95.93").unwrap(), 9007_u16, None);

        let peer8 = Peer::from_bytes(&[8]);
        let socket8 = SocketAddrBi::new(IpAddr::from_str("121.36.95.93").unwrap(), 9008_u16, None);

        let peer9 = Peer::from_bytes(&[9]);
        let socket9 = SocketAddrBi::new(IpAddr::from_str("121.36.95.93").unwrap(), 9009_u16, None);

        let peer10 = Peer::from_bytes(&[10]);
        let socket10 = SocketAddrBi::new(IpAddr::from_str("121.36.95.93").unwrap(), 9010_u16, None);

        let peer1 = Peer::from_bytes(&[1]);

        bdn.address_book.insert(&peer3, &socket3);
        bdn.address_book.insert(&peer4, &socket4);
        bdn.address_book.insert(&peer5, &socket5);
        bdn.address_book.insert(&peer6, &socket6);
        bdn.address_book.insert(&peer7, &socket7);
        bdn.address_book.insert(&peer8, &socket8);
        bdn.address_book.insert(&peer9, &socket9);
        bdn.address_book.insert(&peer10, &socket10);

        bdn.route.insert_relay(&peer1, &peer3);
        bdn.route.insert_relay(&peer1, &peer4);
        bdn.route.insert_relay(&peer1, &peer5);
        bdn.route.insert_relay(&peer1, &peer6);
        bdn.route.insert_relay(&peer1, &peer7);
        bdn.route.insert_relay(&peer1, &peer8);
        bdn.route.insert_relay(&peer1, &peer9);
        bdn.route.insert_relay(&peer1, &peer10);

        bdn.connect().await;

        let server = async_std::task::spawn(BDN::<TcpContext, MlbtRelayCtlContext>::listen(
            9002,
            bdn.msg_sender.clone(),
        ));

        println!(
            "New BDN client at: {:?}",
            bdn.local_identity.peer().get_id()
        );

        loop {
            if let Some(msg) = bdn.next() {
                // debug!("recv:\n{}", &msg);
            }
        }
    }

    #[async_std::test]
    async fn node3() {
        log::setup_logger("bdn_test3").unwrap();

        let mut bdn = BDN::<TcpContext, MlbtRelayCtlContext>::new();

        bdn.connect().await;

        let server = async_std::task::spawn(BDN::<TcpContext, MlbtRelayCtlContext>::listen(
            9003,
            bdn.msg_sender.clone(),
        ));

        println!(
            "New BDN client at: {:?}",
            bdn.local_identity.peer().get_id()
        );

        loop {
            if let Some(msg) = bdn.next() {
                // debug!("recv:\n{}", &msg);
            }
        }
    }

    #[async_std::test]
    async fn node4() {
        log::setup_logger("bdn_test4").unwrap();

        let mut bdn = BDN::<TcpContext, MlbtRelayCtlContext>::new();

        bdn.connect().await;

        let server = async_std::task::spawn(BDN::<TcpContext, MlbtRelayCtlContext>::listen(
            9004,
            bdn.msg_sender.clone(),
        ));

        println!(
            "New BDN client at: {:?}",
            bdn.local_identity.peer().get_id()
        );

        loop {
            if let Some(msg) = bdn.next() {
                // debug!("recv:\n{}", &msg);
            }
        }
    }

    #[async_std::test]
    async fn node5() {
        log::setup_logger("bdn_test5").unwrap();

        let mut bdn = BDN::<TcpContext, MlbtRelayCtlContext>::new();

        bdn.connect().await;

        let server = async_std::task::spawn(BDN::<TcpContext, MlbtRelayCtlContext>::listen(
            9005,
            bdn.msg_sender.clone(),
        ));

        println!(
            "New BDN client at: {:?}",
            bdn.local_identity.peer().get_id()
        );

        loop {
            if let Some(msg) = bdn.next() {
                // debug!("recv:\n{}", &msg);
            }
        }
    }

    #[async_std::test]
    async fn node6() {
        log::setup_logger("bdn_test6").unwrap();

        let mut bdn = BDN::<TcpContext, MlbtRelayCtlContext>::new();

        bdn.connect().await;

        let server = async_std::task::spawn(BDN::<TcpContext, MlbtRelayCtlContext>::listen(
            9006,
            bdn.msg_sender.clone(),
        ));

        println!(
            "New BDN client at: {:?}",
            bdn.local_identity.peer().get_id()
        );

        loop {
            if let Some(msg) = bdn.next() {
                // debug!("recv:\n{}", &msg);
            }
        }
    }

}