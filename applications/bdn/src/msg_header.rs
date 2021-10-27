use std::fmt::Display;

use log::warn;
use num_traits::{FromPrimitive, ToPrimitive};

/// Layout of msg_header:
/// 32 bits in total, from MSB to LSB:
/// MsgType 4 bits, RelayFlag 1 bit, RelayScheme 4 bits,
/// Fanout 8 bits, TTL 4 bits, Reserved 11 bits

const HEADER_LEN: u32 = 32;
const MSG_TYPE_LEN: u32 = 4;
const RELAY_FLAG_LEN: u32 = 1;
const RELAY_SCHEME_LEN: u32 = 4;
const FANOUT_LEN: u32 = 8;
const TTL_LEN: u32 = 4;

const MSG_TYPE_LSB: u32 = HEADER_LEN - MSG_TYPE_LEN;
const RELAY_FLAG_LSB: u32 = MSG_TYPE_LSB - RELAY_FLAG_LEN;
const RELAY_SCHEME_LSB: u32 = RELAY_FLAG_LSB - RELAY_SCHEME_LEN;
const FANOUT_LSB: u32 = RELAY_SCHEME_LSB - FANOUT_LEN;
const TTL_LSB: u32 = FANOUT_LSB - TTL_LEN;

const MSG_TYPE_MASK: u32 = ((1 << MSG_TYPE_LEN) - 1) << MSG_TYPE_LSB;
const RELAY_SCHEME_MASK: u32 = ((1 << RELAY_SCHEME_LEN) - 1) << RELAY_SCHEME_LSB;
const FANOUT_MASK: u32 = ((1 << FANOUT_LEN) - 1) << FANOUT_LSB;
const TTL_MASK: u32 = ((1 << TTL_LEN) - 1) << TTL_LSB;

pub struct MsgHeader {}

impl MsgHeader {
    fn set_header_field(head: &mut u32, value: u32, len: u32, lsb: u32) {

        // set target field to all-zero
        let unset_mask: u32 = !(((1 << len) - 1) << lsb);
        *head &= unset_mask;

        // set target field to value
        let set_mask = value << lsb;
        *head |= set_mask;
    }
}


impl MsgHeader {
    // build header from fields
    pub fn build(
        msg_type: MsgTypeKind, 
        is_relay: bool, 
        relay_method: RelayMethodKind, 
        fanout: u32, 
        ttl: u32) -> Option<u32> 
    {
        let mut header: u32 = 0;

        if MsgType::set_msg_type(&mut header, msg_type).is_none() {
            return None;
        }

        RelayFlag::set_relay_flag(&mut header, is_relay);

        if RelayMethod::set_relay_method(&mut header, relay_method).is_none() {
            return None;
        }

        if FanOut::set_fan_out(&mut header, fanout).is_none() {
            return None;
        }

        if TTL::set_ttl(&mut header, ttl).is_none() {
            return None;
        }

        Some(header)
    }
}

pub struct MsgType {}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, FromPrimitive, ToPrimitive, Debug)]
pub enum MsgTypeKind {
    ROUTE_MSG = 1,
    NET_MEASURE_MSG = 2,
    PAYLOAD_MSG = 3,
}


impl Display for MsgTypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            &MsgTypeKind::NET_MEASURE_MSG => write!(f, "NET_MEASURE_MSG"),
            &MsgTypeKind::ROUTE_MSG => write!(f, "ROUTE_MSG"),
            &MsgTypeKind::PAYLOAD_MSG => write!(f, "PAYLOAD_MSG"),
        }
    }
}


impl MsgType {

    pub fn get_msg_type(n: u32) -> Option<MsgTypeKind> {
        FromPrimitive::from_u32((n & MSG_TYPE_MASK) >> MSG_TYPE_LSB)
    }


    pub fn set_msg_type(head: &mut u32, kind: MsgTypeKind) -> Option<()> {
        if let Some(kind_num) = ToPrimitive::to_u32(&kind) {
            MsgHeader::set_header_field(head, kind_num, 
                MSG_TYPE_LEN, MSG_TYPE_LSB);
            Some(())
        }
        else {
            warn!("Failed to encode msg_type: {}", kind);
            None
        }
    }

}



pub struct RelayFlag {}

impl RelayFlag {

    pub fn get_relay_flag(n: u32) -> bool {
        (n & (1 << RELAY_FLAG_LSB)) != 0
    }

    pub fn set_relay_flag(head: &mut u32, flag: bool) {
        let value: u32;
        
        if flag {
            value = 1;
        }
        else {
            value = 0;
        }

        MsgHeader::set_header_field(head, value, 
            RELAY_FLAG_LEN, RELAY_FLAG_LSB);
    }
}


pub struct RelayMethod {}

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, FromPrimitive, ToPrimitive)]
pub enum RelayMethodKind {
    RANDOM = 1,
    LOOKUP_TABLE_1 = 2,
    LOOKUP_TABLE_2 = 3,
    KAD = 4,
    ALL = 5,
}


impl Display for RelayMethodKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            &RelayMethodKind::RANDOM => write!(f, "RANDOM"),
            &RelayMethodKind::LOOKUP_TABLE_1 => write!(f, "LOOKUP_TABLE_1"),
            &RelayMethodKind::LOOKUP_TABLE_2 => write!(f, "LOOKUP_TABLE_2"),
            &RelayMethodKind::KAD => write!(f, "KAD"),
            &RelayMethodKind::ALL => write!(f, "ALL"),
        }
    }
}


impl RelayMethod {
    
    pub fn get_relay_method(n: u32) -> Option<RelayMethodKind> {
        FromPrimitive::from_u32((n & RELAY_SCHEME_MASK) >> RELAY_SCHEME_LSB)
    }


    pub fn set_relay_method(head: &mut u32, kind: RelayMethodKind) -> Option<()> {
        if let Some(kind_num) = ToPrimitive::to_u32(&kind) {
            MsgHeader::set_header_field(head, kind_num, 
                RELAY_SCHEME_LEN, RELAY_SCHEME_LSB);
            Some(())
        }
        else {
            warn!("Failed to encode msg_type: {}", kind);
            None
        }
    }
}


pub struct FanOut {}


impl FanOut {

    pub fn get_fan_out(n: u32) -> u32 {
        (n & FANOUT_MASK) >> FANOUT_LSB
    }

    pub fn set_fan_out(head: &mut u32, fanout: u32) -> Option<()> {
        if fanout > (1 << FANOUT_LEN) - 1 {
            return None;
        }
        MsgHeader::set_header_field(head, fanout, 
            FANOUT_LEN, FANOUT_LSB);
        Some(())
    }
}


pub struct TTL {}


impl TTL {

    pub fn get_ttl(n: u32) -> u32 {
        (n & TTL_MASK) >> TTL_LSB
    }

    
    pub fn set_ttl(head: &mut u32, ttl: u32) -> Option<()> {
        if ttl > (1 << TTL_LEN) - 1 {
            return None;
        }
        MsgHeader::set_header_field(head, ttl,
            TTL_LEN, TTL_LSB);
        Some(())
    }

}