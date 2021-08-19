mod types;

use std::io::Read;

use flate2::read::GzDecoder;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::client_async;
use tungstenite::Message;
use url::Url;

use crate::types::Quote;

// 
async fn create_tls_stream(url: &Url) -> tokio_native_tls::TlsStream<tokio::net::TcpStream> {
    let host = url.host().unwrap().to_string();
    let socket_address = url.socket_addrs(||None).unwrap().pop().unwrap();

    let tcp_stream = tokio::net::TcpStream::connect(socket_address).await.expect("Connect socket address failed.");

    let native_tls_builder = native_tls::TlsConnector::builder();
    let tls_conn = tokio_native_tls::TlsConnector::from(native_tls_builder.build().unwrap());

    let conn_stream = tls_conn.connect(host.as_str(), tcp_stream).await.unwrap();

    conn_stream
}

#[tokio::main]
async fn main() {

    let url = url::Url::parse("wss://api.huobi.pro/ws").unwrap();

    let tls_stream = create_tls_stream(&url).await;
    let stream = client_async(url, tls_stream).await.expect("Connect failed.").0;

    let (mut write, read) = stream.split();

    println!("Connected!");
    write.send(Message::Text(r#"{
        "sub": "market.btcusdt.depth.step0",
        "id": "id1"
    }"#.to_string() + "\n")).await.unwrap();

    println!("Sent!");

    let r = read.for_each(|message| async {
        let data = message.unwrap().into_data();
        // data is gzip type
        // decompression
        let mut decoder = GzDecoder::new(&data[..]);
        let mut buf = String::new();
        match decoder.read_to_string(&mut buf) {
            Ok(_) => {
                // json parse
                let quote = serde_json::from_str::<Quote>(buf.as_str());
                match quote {
                    Ok(_) => println!("{:?}", quote),
                    Err(e) => println!("found error:{}", e)
                }
            },
            Err(_) => {}
        }

    });

    r.await
}
