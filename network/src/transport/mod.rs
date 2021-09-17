use std::{fmt::Debug, net::SocketAddr};
use crate::error::TransportError;
use futures::{AsyncRead, AsyncWrite};
use async_trait::async_trait;
use crate::identity::crypto;

pub struct IngressStream<S: AsyncRead + AsyncWrite + Send + Unpin + Debug, 
    C: crypto::Signer> 
{
    pub remote_addr: SocketAddr,
    pub stream: S,
    pub remote_pk: Option<<C as crypto::Signer>::PK>,
}

#[async_trait]
pub trait Transport: 'static + Clone + Copy 
{

    type Stream: AsyncRead + AsyncWrite + Send + Unpin + Debug;

    type Listener: Send + Unpin;

    type Signer: Send + crypto::Signer;

    async fn listen(_: SocketAddr) -> Result<Self::Listener, TransportError>;

    async fn connect(_: SocketAddr) -> Result<Self::Stream, TransportError>;

    async fn accept(_: &mut Self::Listener) -> 
        Result<IngressStream<Self::Stream, Self::Signer>, TransportError>;

}