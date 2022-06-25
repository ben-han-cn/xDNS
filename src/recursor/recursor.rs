use crate::server::Handler;
use anyhow::{self, bail};
use async_trait::async_trait;
use r53::{DomainTree, FindResultFlag, Name, Rcode, Request, Response, ResponseBuilder};
use reqwest;
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};
use tokio::time;

use super::cache::MessageCache;
use super::client::roundtrip;
use super::query_statistic::{QueryInfo, QueryStatistic};

const MESSAGE_CACHE_SIZE: usize = 40960;
const QUERY_INFO_CACHE_SIZE: usize = 1000;
const REPORT_HTTP_PATH: &'static str = "/filemarket/v1/dns/set";

#[derive(Clone)]
pub struct Recursor {
    inner: Arc<RecursorInner>,
}

struct RecursorInner {
    forwarders: RwLock<DomainTree<SocketAddr>>,
    cache: Mutex<MessageCache>,
    query_stat: Mutex<QueryStatistic>,
}

impl RecursorInner {
    pub fn new() -> Self {
        Self {
            forwarders: RwLock::new(DomainTree::new()),
            cache: Mutex::new(MessageCache::new(MESSAGE_CACHE_SIZE)),
            query_stat: Mutex::new(QueryStatistic::new(QUERY_INFO_CACHE_SIZE)),
        }
    }

    pub fn add_query(&self, name: &Name) {
        let mut stat = self.query_stat.lock().unwrap();
        stat.add_query(name);
    }

    pub fn collect_query_statistic(&self) -> QueryInfo {
        let mut stat = self.query_stat.lock().unwrap();
        stat.sort_and_clear()
    }

    pub fn add_forward(&self, zone: Name, addr: SocketAddr) -> anyhow::Result<()> {
        let mut forwarders = self.forwarders.write().unwrap();
        forwarders.insert(zone, Some(addr));
        Ok(())
    }

    pub fn get_forward(&self, name: &Name) -> Option<SocketAddr> {
        let forwarders = self.forwarders.read().unwrap();
        let result = forwarders.find(name);
        if result.flag == FindResultFlag::ExacatMatch || result.flag == FindResultFlag::PartialMatch
        {
            if let Some(addr) = result.get_value() {
                return Some(addr.clone());
            }
        }
        None
    }

    pub fn gen_response(&self, req: &Request) -> Option<Response> {
        let mut cache = self.cache.lock().unwrap();
        cache.gen_response(req)
    }

    pub fn add_response(&self, resp: Response) {
        let mut cache = self.cache.lock().unwrap();
        cache.add_response(resp);
    }
}

impl Recursor {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RecursorInner::new()),
        }
    }

    pub fn add_forward(&self, zone: Name, addr: SocketAddr) {
        self.inner.add_forward(zone, addr);
    }

    pub async fn collect_query_statistic(&self, report_server: SocketAddr) {
        let mut interval = time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            let info = self.inner.collect_query_statistic();
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap()
                .post(format!("http://{}{}", report_server, REPORT_HTTP_PATH))
                .json(&info)
                .send()
                .await;
        }
    }
}

#[async_trait]
impl Handler for Recursor {
    async fn resolve(&mut self, req: Request) -> anyhow::Result<Response> {
        self.inner.add_query(&req.question.name);

        if let Some(resp) = self.inner.gen_response(&req) {
            return Ok(resp);
        }

        if let Some(addr) = self.inner.get_forward(&req.question.name) {
            let mut resp = roundtrip(&req, addr).await?;
            self.inner.add_response(resp.clone());
            let mut builder = ResponseBuilder::new(&mut resp);
            builder.id(req.header.id).done();
            Ok(resp)
        } else {
            let mut resp = Response::with_question(req.question.name.clone(), req.question.typ);
            let mut builder = ResponseBuilder::new(&mut resp);
            builder
                .id(req.header.id)
                .make_response()
                .rcode(Rcode::ServFail)
                .done();
            Ok(resp)
        }
    }
}
