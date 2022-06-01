use std::fmt::Display;
use std::sync::{Arc, RwLock};

use anyhow::{self, bail};
use async_trait::async_trait;
use r53::{
    DomainTree, FindResultFlag, Name, RRType, RRset, Rcode, Request, Response, ResponseBuilder,
    SectionType,
};

use super::zone::{FindMode, FindResult, MemoryZone};
use super::zone_content_generator::{default_ns_and_glue, default_soa};
use crate::server::Handler;

#[derive(Clone)]
pub struct Auth {
    zones: Arc<RwLock<DomainTree<MemoryZone>>>,
}

impl Auth {
    pub fn new() -> Self {
        Self {
            zones: Arc::new(RwLock::new(DomainTree::new())),
        }
    }

    pub fn add_zone<T: AsRef<str> + Display>(
        &self,
        name: Name,
        ip_addrs: &Vec<T>,
    ) -> anyhow::Result<()> {
        let mut zones = self.zones.write().unwrap();
        let result = zones.find(&name);
        if result.flag != FindResultFlag::ExacatMatch {
            let mut zone = MemoryZone::new(name.clone());
            zone.add_rrset(default_soa(&name));
            let (ns, glue) = default_ns_and_glue(&name, ip_addrs);
            zone.add_rrset(ns);
            zone.add_rrset(glue);
            zones.insert(name, Some(zone));
            Ok(())
        } else {
            bail!("add duplicate zone");
        }
    }

    pub fn add_rrset(&self, zone: &Name, rrset: RRset) -> anyhow::Result<()> {
        let mut zones = self.zones.write().unwrap();
        let result = zones.find(zone);
        if result.flag == FindResultFlag::ExacatMatch {
            if let Some(zone) = result.get_value_mut() {
                zone.add_rrset(rrset);
                return Ok(());
            }
        }

        bail!("unknown zone");
    }
}

#[async_trait]
impl Handler for Auth {
    async fn resolve(&mut self, req: Request) -> anyhow::Result<Response> {
        let zones = self.zones.read().unwrap();
        let result = zones.find(&req.question.name);
        let mut resp = Response::with_question(req.question.name.clone(), req.question.typ);

        let mut builder = ResponseBuilder::new(&mut resp);
        builder.id(req.header.id).make_response();
        builder.rcode(Rcode::Refused).done();
        if result.flag == FindResultFlag::ExacatMatch || result.flag == FindResultFlag::PartialMatch
        {
            if let Some(zone) = result.get_value() {
                match zone.find(&req.question.name, req.question.typ, FindMode::DefaultFind) {
                    FindResult::Success(rrset) => {
                        builder
                            .rcode(Rcode::NoError)
                            .add_rrset(SectionType::Answer, rrset)
                            .done();
                    }
                    FindResult::Delegation(rrset) => {
                        if let Some(glues) = zone.get_glue_for_ns(&rrset) {
                            for rrset in glues {
                                builder.add_rrset(SectionType::Additional, rrset);
                            }
                        }

                        builder
                            .rcode(Rcode::NoError)
                            .add_rrset(SectionType::Authority, rrset)
                            .done();
                    }
                    FindResult::NXDomain => {
                        let soa = zone.get_apex_rrset(RRType::SOA).unwrap();
                        builder
                            .rcode(Rcode::NXDomain)
                            .add_rrset(SectionType::Authority, soa)
                            .done();
                    }
                    FindResult::NXRRset => {
                        builder.rcode(Rcode::NoError).done();
                    }
                }
            }
        }
        Ok(resp)
    }
}
