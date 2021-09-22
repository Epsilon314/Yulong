use std::{fmt::Debug, net::SocketAddr};
use crate::error::TransportError;
use futures::{AsyncRead, AsyncWrite};
use async_trait::async_trait;
use crate::identity::crypto;

pub struct IngressStream<S: AsyncRead + AsyncWrite + Send + Unpin + Debug> 
{
    pub remote_addr: SocketAddr,
    pub stream: S,
    pub remote_pk: crypto::PublicKey,
}

#[async_trait]
pub trait Transport: 'static + Clone + Copy + Unpin + Send
{

    type Stream: AsyncRead + AsyncWrite + Send + Unpin + Debug;

    type Listener: Send + Unpin;

    async fn listen(_: &SocketAddr) -> Result<Self::Listener, TransportError>;

    async fn connect(_: &SocketAddr) -> Result<Self::Stream, TransportError>;

    async fn accept(_: &mut Self::Listener) -> 
        Result<IngressStream<Self::Stream>, TransportError>;
}