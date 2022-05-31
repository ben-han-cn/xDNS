mod api;
mod auth;
mod server;

use std::net::SocketAddr;
use std::thread;

use tokio::runtime::{self, Runtime};

use auth::Auth;
use server::UdpServer;

fn main() {
    let auth = Auth::new();
    {
        let auth = auth.clone();
        thread::spawn(move || {
            runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(api::start(auth));
        });
    }

    Runtime::new().unwrap().block_on(async {
        UdpServer::new(auth)
            .run("127.0.0.1:5555".parse::<SocketAddr>().unwrap())
            .await
    })
}
