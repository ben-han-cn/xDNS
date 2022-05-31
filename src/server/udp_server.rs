use super::handler::Handler;
use r53::{MessageRender, Request, Response};
use std::net::SocketAddr;
use tokio::net::UdpSocket;

const QUERY_BUFFER_LEN: usize = 512;
const RESPONSE_BUFFER_LEN: usize = 1232;

pub struct UdpServer<H: Handler> {
    handler: H,
}

impl<H: Handler> UdpServer<H> {
    pub fn new(handler: H) -> Self {
        UdpServer { handler }
    }

    pub async fn run(&mut self, addr: SocketAddr) {
        let socket = UdpSocket::bind(&addr).await.expect("bind addr failed");
        let mut req_buf = [0; QUERY_BUFFER_LEN];
        let mut resp_buf = [0; RESPONSE_BUFFER_LEN];
        loop {
            if let Ok((len, peer)) = socket.recv_from(&mut req_buf).await {
                if let Ok(request) = Request::from_wire(&req_buf[..len]) {
                    if let Ok(response) = self.handler.resolve(request).await {
                        let mut render = MessageRender::new(&mut resp_buf);
                        if let Ok(len) = response.to_wire(&mut render) {
                            socket.send_to(&resp_buf[..len], peer).await.unwrap();
                        }
                    }
                }
            }
        }
    }
}
