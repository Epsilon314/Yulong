use async_trait::async_trait;
use directories_next;
use futures::ready;
use futures::StreamExt;
use futures::{AsyncRead, AsyncWrite};
use quinn::{RecvStream, SendStream};
use yulong_network::error::DumbError;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::{fs, io};
use yulong_network::error::TransportError;
use yulong_network::identity::crypto::PublicKey;
use yulong_network::transport::{IngressStream, Transport};

#[derive(Clone, Copy)]
pub struct QuicContext {}

#[derive(Debug)]
pub struct QuicStream {
    send_stream: SendStream,
    recv_stream: RecvStream,
}

impl AsyncRead for QuicStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        let recv = &mut self.get_mut().recv_stream;
        let len = ready!(AsyncRead::poll_read(Pin::new(recv), cx, buf))?;

        Poll::Ready(Ok(len))
    }
}

impl AsyncWrite for QuicStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        let send = &mut self.get_mut().send_stream;
        AsyncWrite::poll_write(Pin::new(send), cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let send = &mut self.get_mut().send_stream;
        AsyncWrite::poll_flush(Pin::new(send), cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        let send = &mut self.get_mut().send_stream;
        AsyncWrite::poll_close(Pin::new(send), cx)
    }
}

#[async_trait]
impl Transport for QuicContext {
    type Stream = QuicStream;

    type Listener = quinn::Incoming;

    async fn listen(addr: &std::net::SocketAddr) -> Result<Self::Listener, TransportError> {
        let mut transport_config = quinn::TransportConfig::default();
        transport_config.max_concurrent_uni_streams(0).unwrap();

        let mut server_config = quinn::ServerConfig::default();
        server_config.transport = Arc::new(transport_config);
        let mut server_config = quinn::ServerConfigBuilder::new(server_config);
        server_config.protocols(&[b"hq-29"]);

        let dirs = directories_next::ProjectDirs::from("", "quic", "transport").unwrap();
        let path = dirs.data_local_dir();
        let cert_path = path.join("cert.der");
        let key_path = path.join("key.der");
        let (cert, key) = match fs::read(&cert_path).and_then(|x| Ok((x, fs::read(&key_path)?))) {
            Ok(x) => x,
            Err(ref e) if e.kind() == io::ErrorKind::NotFound => {
                println!("generating self-signed certificate");
                let cert = rcgen::generate_simple_self_signed(vec!["quic".into()]).unwrap();
                let key = cert.serialize_private_key_der();
                let cert = cert.serialize_der().unwrap();
                fs::create_dir_all(&path).expect("failed to create certificate directory");
                fs::write(&cert_path, &cert).expect("failed to write certificate");
                fs::write(&key_path, &key).expect("failed to write private key");
                (cert, key)
            }
            Err(err) => {
                return Err(TransportError::new("failed to read certificate", err));
            }
        };

        let key = quinn::PrivateKey::from_der(&key).unwrap();
        let cert = quinn::Certificate::from_der(&cert).unwrap();
        server_config
            .certificate(quinn::CertificateChain::from_certs(vec![cert]), key)
            .unwrap();

        let mut endpoint = quinn::Endpoint::builder();
        endpoint.listen(server_config.build());

        let (endpoint, incoming) = endpoint.bind(&addr).unwrap();
        println!("listening on {}", endpoint.local_addr().unwrap());

        Ok(incoming)
    }

    async fn connect(addr: &std::net::SocketAddr) -> Result<Self::Stream, TransportError> {
        let mut endpoint = quinn::Endpoint::builder();
        let mut client_config = quinn::ClientConfigBuilder::default();
        client_config.protocols(&[b"hq-29"]);

        let dirs = directories_next::ProjectDirs::from("", "quic", "transport").unwrap();
        match fs::read(dirs.data_local_dir().join("cert.der")) {
            Ok(cert) => {
                client_config
                    .add_certificate_authority(quinn::Certificate::from_der(&cert).unwrap())
                    .unwrap();
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Err(TransportError::new("local server certificate not found", err));
            }
            Err(err) => {
                return Err(TransportError::new("failed to open local server certificate", err));
            }
        }

        endpoint.default_client_config(client_config.build());
        let (endpoint, _) = endpoint.bind(&"0.0.0.0:0".parse().unwrap()).unwrap();
        let new_conn = endpoint.connect(&addr, "quic").unwrap().await.unwrap();
        let quinn::NewConnection {
            connection: conn, ..
        } = new_conn;

        let (send, recv) = conn.clone().open_bi().await.unwrap();

        Ok(QuicStream {
            send_stream: send,
            recv_stream: recv,
        })
    }

    async fn accept(
        listener: &mut Self::Listener,
    ) -> Result<IngressStream<Self::Stream>, TransportError> {
        if let Some(conn) = listener.next().await {
            let quinn::NewConnection {
                connection,
                mut bi_streams,
                ..
            } = conn.await.unwrap();
            let remote_addr = connection.remote_address();

            if let Some(stream) = bi_streams.next().await {
                let (send, recv) = match stream {
                    Err(quinn::ConnectionError::ApplicationClosed(_)) => {
                        return Err(TransportError::new("Application Closed", DumbError));
                    }
                    Err(err) => {
                        return Err(TransportError::new("Other error", err));
                    }
                    Ok(s) => s,
                };

                return Ok(IngressStream {
                    remote_addr,
                    stream: QuicStream {
                        send_stream: send,
                        recv_stream: recv,
                    },
                    // todo: retrieve pk
                    remote_pk: PublicKey::NoKey,
                });
            }
        }

        unreachable!()
    }
}
