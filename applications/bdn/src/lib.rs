mod message;
mod route;
pub mod overlay;

mod bdn_message {
    include!(concat!(env!("OUT_DIR"), "/bdn.rs"));
}

mod common {
    use std::net::IpAddr;
    use std::hash::Hash;
    
    #[derive(Debug, Clone, Copy, Eq)]
    pub struct SocketAddrBi {
        ip: IpAddr,
        lport: u16, // default listening port
        iport: Option<u16>, // incoming port
    }
    
    impl SocketAddrBi {
        pub fn new(ip: IpAddr, lport: u16, iport: Option<u16>) -> Self {
            Self{ip, lport, iport}
        }

        pub fn ip(&self) -> IpAddr {self.ip}

        pub fn listen_port(&self) -> u16 {self.lport}

        pub fn incoming_port(&self) -> Option<u16> {self.iport}
    }
    
    impl Hash for SocketAddrBi {
        fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
            self.ip.hash(state);
        }
    }
    
    impl PartialEq for SocketAddrBi{
        fn eq(&self, other: &Self) -> bool {
            self.ip == other.ip
        }
    }
    
    pub type MessageWithIp = (SocketAddrBi, Vec<u8>);
    
    pub const DEFAULT_BDN_PORT: u16 = 10450;
    pub const MSG_MAXLEN: usize = 2048; //bytes
}