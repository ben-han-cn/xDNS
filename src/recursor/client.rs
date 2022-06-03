use std::{net::SocketAddr, time::Duration};

use anyhow::{self, bail};
use tokio::{net::UdpSocket, time::timeout};

use r53::{MessageRender, Request, Response};

const DEFAULT_RECV_TIMEOUT: Duration = Duration::from_secs(3);
const QUERY_BUFFER_LEN: usize = 512;
const RESPONSE_BUFFER_LEN: usize = 1232;

pub async fn roundtrip(req: &Request, target: SocketAddr) -> anyhow::Result<Response> {
    let mut req_buf = [0; QUERY_BUFFER_LEN];
    let mut resp_buf = [0; RESPONSE_BUFFER_LEN];

    let mut render = MessageRender::new(&mut req_buf);
    let len = req.to_wire(&mut render)?;
    let socket = UdpSocket::bind(&("0.0.0.0:0".parse::<SocketAddr>().unwrap())).await?;
    socket.connect(target).await?;
    socket.send(&req_buf[..len]).await?;

    let result = timeout(DEFAULT_RECV_TIMEOUT, socket.recv(&mut resp_buf)).await?;
    match result {
        Ok(size) => Response::from_wire(&resp_buf[..size]),
        Err(e) => {
            bail!(e);
        }
    }
}
