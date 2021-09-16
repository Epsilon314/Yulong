use yulong_network::transport::{Transport, IngressStream};
use yulong_network::error::TransportError;
use futures::{AsyncRead, AsyncWrite};
use std::{pin::Pin};
use async_trait::async_trait;
use tokio;


#[derive(Clone, Copy)]
pub struct TcpContext {}


#[derive(Debug)]
pub struct TcpStream {
    pub tokio_tcp_stream: tokio::net::TcpStream
}


impl TcpStream {

    fn from(tokio_tcp_stream: tokio::net::TcpStream) -> Self {
        Self{tokio_tcp_stream}
    }


    #[allow(dead_code)]
    fn into(self: TcpStream) -> tokio::net::TcpStream {
        self.tokio_tcp_stream
    }
}


impl AsyncRead for TcpStream {

    fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &mut [u8],
        ) -> std::task::Poll<std::io::Result<usize>> {

        let mut tokio_buf = tokio::io::ReadBuf::new(buf);

        futures::ready!(tokio::io::AsyncRead::poll_read(
            Pin::new(&mut self.tokio_tcp_stream), cx, &mut tokio_buf))?;

        std::task::Poll::Ready(Ok(tokio_buf.filled().len()))
    }
}


impl AsyncWrite for TcpStream {

    fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut std::task::Context<'_>,
            buf: &[u8],
        ) -> std::task::Poll<std::io::Result<usize>> {

        tokio::io::AsyncWrite::poll_write(
            Pin::new(&mut self.tokio_tcp_stream), cx, buf)
    }


    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) ->   std::task::Poll<std::io::Result<()>> {

        tokio::io::AsyncWrite::poll_flush(
            Pin::new(&mut self.tokio_tcp_stream), cx)
    }


    fn poll_close(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::io::Result<()>> {
        tokio::io::AsyncWrite::poll_shutdown(
            Pin::new(&mut self.tokio_tcp_stream), cx)
    }
}

#[async_trait]
impl Transport for TcpContext {

    type Stream = TcpStream;
    type Listener = tokio::net::TcpListener;

    async fn listen(addr: std::net::SocketAddr) -> Result<Self::Listener, TransportError> {
        match tokio::net::TcpListener::bind(addr).await {
            Ok(std_listener) => {
                Ok(std_listener)
            }
            Err(error) => {
                Err(TransportError::new(
                    format!("Transport error happened when listening on {:?}", addr),
                    error))
            }
        }
    }


    async fn connect(addr: std::net::SocketAddr) -> Result<Self::Stream, TransportError> {

        match tokio::net::TcpStream::connect(addr).await {
            Ok(std_stream) => {
                Ok(Self::Stream::from(std_stream))
            }
            Err(error) => {
                Err(TransportError::new(
                format!("Transport Error happens when connecting to {:?}", addr), error))
            }
        }
    }


    async fn accept(listener: &mut Self::Listener) 
        -> Result<IngressStream<Self::Stream>, TransportError> {

        match listener.accept().await {
            Ok((stream, remote_addr)) => {
                let stream = Self::Stream::from(stream);
                Ok(IngressStream{remote_addr, stream})
            }
            Err(error) => {
                Err(TransportError::new(
                    format!("Transport Error happens when polling accept on {:?}", listener), 
                    error))
            }
        }
    }
}
