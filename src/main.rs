mod api;
mod auth;
mod recursor;
mod server;

use std::net::SocketAddr;
use std::thread;

use clap::{arg, Command};
use tokio::runtime::{self, Runtime};
use tokio::signal;

use auth::Auth;
use recursor::Recursor;
use server::UdpServer;

fn main() {
    let matches = Command::new("xdns")
        .about("xdns a dns server")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .allow_external_subcommands(true)
        .subcommand(
            Command::new("auth")
                .about("authority dns server")
                .arg(arg!(--dns <DNS> "dns server addr"))
                .arg(arg!(--http <HTTP> "http server addr"))
                .arg_required_else_help(true),
        )
        .subcommand(
            Command::new("recursor")
                .about("recursive dns server")
                .arg(arg!(--dns <DNS> "dns server addr"))
                .arg(arg!(--http <HTTP> "http server addr"))
                .arg_required_else_help(true),
        )
        .get_matches();

    match matches.subcommand() {
        Some(("auth", sub_matches)) => {
            let dns_addr = sub_matches
                .value_of("dns")
                .unwrap()
                .parse::<SocketAddr>()
                .unwrap();
            let cmd_addr = sub_matches
                .value_of("http")
                .unwrap()
                .parse::<SocketAddr>()
                .unwrap();
            start_auth(cmd_addr, dns_addr);
        }

        Some(("recursor", sub_matches)) => {
            let dns_addr = sub_matches
                .value_of("dns")
                .unwrap()
                .parse::<SocketAddr>()
                .unwrap();
            let cmd_addr = sub_matches
                .value_of("http")
                .unwrap()
                .parse::<SocketAddr>()
                .unwrap();
            start_recursor(cmd_addr, dns_addr);
        }

        _ => unreachable!(),
    }
}

fn start_auth(cmd_addr: SocketAddr, dns_addr: SocketAddr) {
    let auth = Auth::new();
    {
        let auth = auth.clone();
        thread::spawn(move || {
            runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(api::start_auth_api(auth, cmd_addr));
        });
    }

    Runtime::new().unwrap().block_on(async move {
        tokio::spawn(async move { UdpServer::new(auth).run(dns_addr).await });

        match signal::ctrl_c().await {
            Ok(()) => {
                println!("get stop signal, bye!");
            }
            Err(err) => {
                panic!("listen shutdown signal failed: {}", err);
            }
        }
    })
}

fn start_recursor(cmd_addr: SocketAddr, dns_addr: SocketAddr) {
    let recursor = Recursor::new();
    {
        let recursor = recursor.clone();
        thread::spawn(move || {
            runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(api::start_recursor_api(recursor, cmd_addr));
        });
    }

    Runtime::new().unwrap().block_on(async move {
        let recursor_clone = recursor.clone();
        tokio::spawn(async move { UdpServer::new(recursor).run(dns_addr).await });

        tokio::spawn(async move { recursor_clone.collect_query_statistic().await });

        match signal::ctrl_c().await {
            Ok(()) => {
                println!("get stop signal, bye!");
            }
            Err(err) => {
                panic!("listen shutdown signal failed: {}", err);
            }
        }
    })
}
