#![no_std]
#![no_main]
use dstd::prelude::*;
use dstd::io;

use asyncrt::net::{TcpListener, TcpStream};
use asyncrt::io::{AsyncReadExt, AsyncWriteExt};

asyncrt::main!(main);
async fn main() -> io::Result<()> {
    let server = TcpListener::bind("0.0.0.0:1234".parse().unwrap())?;
    loop {
        let (conn, _addr) = server.accept().await?;
        asyncrt::spawn(handle(conn));
    }
}

async fn handle(mut conn: TcpStream) -> io::Result<()> {
    let mut buf = vec![0; 4096];
    loop {
        let nr = conn.read(&mut buf).await?;
        if nr == 0 { break; }
        conn.write(&buf[..nr]).await?;
    }
    Ok(())
}
