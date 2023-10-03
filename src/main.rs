use anyhow::{anyhow, Result};
use fastwebsockets::upgrade;
use fastwebsockets::OpCode;
use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::Body;
use hyper::Request;
use hyper::Response;
use rustls::{Certificate, PrivateKey};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_rustls::rustls;
use tokio_rustls::TlsAcceptor;

async fn handle_client(fut: upgrade::UpgradeFut) -> Result<()> {
    let mut ws = fut.await?;
    ws.set_writev(false);
    let mut ws = fastwebsockets::FragmentCollector::new(ws);

    loop {
        let frame = ws.read_frame().await?;
        match frame.opcode {
            OpCode::Close => break,
            OpCode::Text | OpCode::Binary => {
                ws.write_frame(frame).await?;
            }
            _ => {}
        }
    }

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
