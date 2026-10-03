#![no_std]
#![no_main]
use dstd::prelude::*;
use dstd::io;

use asyncrt::net::{TcpListener, TcpStream};
use asyncrt::io::{AsyncReadExt, AsyncWriteExt};

asyncrt::main!(main);
async fn main() -> io::Result<()> {
    let server = TcpListener::bind("0.0.0.0:8080".parse().unwrap())?;
    loop {
        let (client, addr) = server.accept().await?;
        println!("Request from {addr}");
        asyncrt::spawn(handle(client));
    }
}

async fn handle(mut client: TcpStream) -> io::Result<()> {
    client.write(concat!(
        "HTTP/1.1 200 OK\r\n",
        "Connection: close\r\n",
        "Content-Length: 17\r\n",
        "Connection: keep-alive\r\n",
        "\r\n",
        "asyncrt says hi!\n",
    ).as_bytes()).await?;
    Ok(())
}
