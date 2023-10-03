use anyhow::{anyhow, bail, ensure, Result};
use fastwebsockets::OpCode;
use fastwebsockets::{upgrade, WebSocketError};
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::Body;
use hyper::Request;
use hyper::Response;
use rustls::{Certificate, PrivateKey};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use std::time::Duration;

use tokio::net::TcpListener;
use tokio::time::{sleep_until, Instant};
use tokio_rustls::rustls;
use tokio_rustls::TlsAcceptor;

async fn handle_client(fut: upgrade::UpgradeFut) -> Result<()> {
    const CONNECTION_TIMEOUT: u64 = 10;
    const DEFUSE_TIMEOUT: u64 = 10;
    const UPDATE_SIZE: usize = 17;

    let mut ws = fut.await?;
    ws.set_writev(false);

    let mut initial_data = Vec::new();
    loop {
        let frame = ws.read_frame().await?;
        match frame.opcode {
            OpCode::Binary | OpCode::Continuation => {
                initial_data.extend_from_slice(&frame.payload);
                if frame.fin {
                    break;
                }
            }
            _ => bail!("bad setup message {:?}", frame.opcode),
        }
    }
    dbg!(initial_data.len());
    let mut last_message = [u8::MAX; UPDATE_SIZE];
    let mut defuse_timeout = None;
    let mut connection_timeout = Instant::now() + Duration::from_secs(CONNECTION_TIMEOUT);
    let alert = loop {
        let timeout = std::iter::once(connection_timeout)
            .chain(defuse_timeout.into_iter())
            .min()
            .unwrap();
        let frame = tokio::select! {
            frame = ws.read_frame() =>{
                frame
            }
            _t = sleep_until(timeout) =>{
                break true;
            }
        };
        let frame = match frame {
            Ok(frame) => frame,
            Err(e @ (
                WebSocketError::InvalidFragment |
                WebSocketError::InvalidUTF8 |
                WebSocketError::InvalidContinuationFrame |
                WebSocketError::InvalidStatusCode |
                WebSocketError::InvalidUpgradeHeader |
                WebSocketError::InvalidConnectionHeader |
                WebSocketError::InvalidCloseFrame |
                WebSocketError::InvalidCloseCode |
                WebSocketError::ReservedBitsNotZero |
                WebSocketError::ControlFrameFragmented |
                WebSocketError::PingFrameTooLarge |
                WebSocketError::FrameTooLarge |
                WebSocketError::InvalidSecWebsocketVersion |
                WebSocketError::InvalidValue |
                WebSocketError::MissingSecWebSocketKey |
                WebSocketError::HTTPError(_) // should only occur during upgrade
            )) => {
                return Err(e.into());
            }
            Err(WebSocketError::ConnectionClosed | WebSocketError::UnexpectedEOF | WebSocketError::IoError(_)) => {
                break true;
            }
        };
        match frame.opcode {
            OpCode::Close => {
                break true;
            }
            OpCode::Binary => {
                let payload = &*frame.payload;
                ensure!(payload.len() == UPDATE_SIZE);
                last_message.copy_from_slice(payload);
                match payload[0] {
                    0 => defuse_timeout = None,
                    1 => {
                        if defuse_timeout.is_none() {
                            defuse_timeout =
                                Some(Instant::now() + Duration::from_secs(DEFUSE_TIMEOUT));
                        }
                    }
                    2 => {
                        break true;
                    }
                    3 => {
                        break false;
                    }
                    _ => bail!("unrecognized code"),
                }
                connection_timeout = Instant::now() + Duration::from_secs(CONNECTION_TIMEOUT);
            }
            _ => bail!("bad message {:?}", frame.opcode),
        }
    };
    dbg!(alert, last_message);
    Ok(())
}

async fn server_upgrade(mut req: Request<Body>) -> Result<Response<Body>> {
    let (response, fut) = upgrade::upgrade(&mut req)?;

    tokio::spawn(async move {
        if let Err(e) = handle_client(fut).await {
            eprintln!("Error in websocket connection: {}", e);
        }
    });

    Ok(response)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let acceptor = tls_acceptor()?;
    let address = "0.0.0.0:4444";
    let listener = TcpListener::bind(address).await?;
    println!("Server started, listening on {}", address);
    loop {
        let (stream, _) = listener.accept().await?;
        println!("Client connected");
        let acceptor = acceptor.clone();
        tokio::spawn(async move {
            let stream = acceptor.accept(stream).await.unwrap();
            let conn_fut = Http::new()
                .serve_connection(stream, service_fn(server_upgrade))
                .with_upgrades();
            if let Err(e) = conn_fut.await {
                println!("An error occurred: {:?}", e);
            }
        });
    }
}

fn tls_acceptor() -> Result<TlsAcceptor> {
    let key = load_private_key_from_file("key.pem")?;
    let certs = load_certificates_from_pem("cert.pem")?;
    let config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;
    Ok(TlsAcceptor::from(Arc::new(config)))
}

fn load_certificates_from_pem(path: &str) -> std::io::Result<Vec<Certificate>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let certs = rustls_pemfile::certs(&mut reader)?;
    Ok(certs.into_iter().map(Certificate).collect())
}

fn load_private_key_from_file(path: &str) -> Result<PrivateKey> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut keys = rustls_pemfile::pkcs8_private_keys(&mut reader)?;

    match keys.len() {
        0 => Err(anyhow!("No PKCS8-encoded private key found in {path}")),
        1 => Ok(PrivateKey(keys.remove(0))),
        _ => Err(anyhow!(
            "More than one PKCS8-encoded private key found in {path}"
        )),
    }
}
