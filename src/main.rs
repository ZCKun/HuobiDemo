mod types;

use std::str::from_utf8;

use async_trait::async_trait;

use futures_util::{Future, SinkExt, StreamExt, future, pin_mut, stream::{SplitSink, SplitStream}};
use inflate::inflate_bytes;
use tokio::net::TcpStream;
use tokio_native_tls::TlsStream;
use tokio_tungstenite::{WebSocketStream, client_async, connect_async};
use tungstenite::Message;
use url::Url;

#[async_trait]
trait Adapter {
    async fn create_tls_stream(&self, url: &Url) -> tokio_native_tls::TlsStream<tokio::net::TcpStream> {
        let host = url.host().unwrap().to_string();
        let socket_address = url.socket_addrs(||None).unwrap().pop().unwrap();

        let tcp_stream = tokio::net::TcpStream::connect(socket_address).await.expect("Connect socket address failed.");

        let native_tls_builder = native_tls::TlsConnector::builder();
        let tls_conn = tokio_native_tls::TlsConnector::from(native_tls_builder.build().unwrap());

        let conn_stream = tls_conn.connect(host.as_str(), tcp_stream).await.unwrap();

        conn_stream
    }

    async fn on_message(&mut self, data: &Vec<u8>);
}


struct OKex {
    url: Url,
    stream: Option<WebSocketStream<tokio_native_tls::TlsStream<tokio::net::TcpStream>>>,
}

impl OKex {
    fn new(url: &str) -> OKex {
        Self {
            url: url::Url::parse(url).unwrap(),
            stream: None,
        }
    }

    async fn connect(&mut self) {
        let tls_stream = self.create_tls_stream(&self.url).await;
        let stream = client_async(&self.url, tls_stream).await.expect("Connect failed.").0;
        self.stream = Some(stream);
    }

    pub async fn subscribe(&mut self, args: Vec<&str>) {
        let (mut write, _) = self.stream.as_mut().unwrap().split();
        let msg = format!(r#"{{
            "op": "subscribe",
            "args": {:#?}
        }}"#, args);
        write.send(Message::Text(msg + "\n")).await.unwrap();
    }

    pub async fn start<F, Fut>(&mut self, callback: F) 
    where
        F: Fn(Message, &mut futures_util::stream::SplitSink<&mut WebSocketStream<tokio_native_tls::TlsStream<tokio::net::TcpStream>>, Message>) -> Fut,
        Fut: Future<Output = ()>,
    {
        self.connect().await;
        println!("Connected!");

        let (mut write, read) = self.stream.as_mut().unwrap().split();
        read.fold(write, |mut write, m| async move {
            on_message(m.unwrap(), &mut write).await;
            write
        }).await;
            /*read.for_each(|message| async {
                let data = message.unwrap().into_data();
                on_message(message.unwrap(), write);
            }).await;*/
    }
}

#[async_trait]
impl Adapter for OKex {

    async fn on_message(&mut self, data: &Vec<u8>) {

    }
}

async fn on_message(message: Message,
    write: &mut futures_util::stream::SplitSink<&mut WebSocketStream<tokio_native_tls::TlsStream<tokio::net::TcpStream>>, Message>) {
    let data = message.into_data();
    println!("{:?}", data);
}


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

async fn subscribe(write: &mut SplitSink<WebSocketStream<TlsStream<TcpStream>>, Message>, args: &Vec<&str>) {
    let msg = format!(r#"{{
        "op": "subscribe",
        "args": {:?}
    }}"#, args);
    write.send(Message::Text(msg.to_string() + "\n")).await.unwrap();
    println!("sent, msg:{}", msg);
}

async fn listen(read: &mut SplitStream<WebSocketStream<TlsStream<TcpStream>>>) {
    read.for_each(|message| async {
        let data = message.unwrap().into_data();
        let decoded = inflate_bytes(&data).unwrap();
        println!("{}", from_utf8(&decoded).unwrap());
    }).await;
}

#[tokio::main]
async fn main() {
    let url = Url::parse("wss://real.okex.com:8443/ws/v3").unwrap();

    let tls_stream = create_tls_stream(&url).await;
    let stream = client_async(&url, tls_stream).await.expect("connect failed.").0;
    println!("connected!");

    let (mut write, mut read) = stream.split();

    let args = vec!["spot/ticker:ETH-USDT", "spot/ticker:BTC-USDT"];
    subscribe(&mut write, &args).await;
    let ws_to_listen = listen(&mut read);

    println!("Hello");
    ws_to_listen.await;
}
