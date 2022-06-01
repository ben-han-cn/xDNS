use r53::{DomainTree, FindResultFlag, Name, NodeChain, NodePtr, RRType, RRset};
use std::sync::RwLock;

#[derive(Debug)]
pub(crate) enum FindMode {
    DefaultFind,
    GlueOkFind,
}

#[derive(Debug)]
pub(crate) enum FindResult {
    Success(RRset),
    Delegation(RRset),
    NXDomain,
    NXRRset,
}

type RRsets = Vec<RRset>;

pub(crate) struct MemoryZone {
    origin: Name,
    domains: RwLock<DomainTree<RRsets>>,
}

impl MemoryZone {
    pub fn new(name: Name) -> Self {
        Self {
            origin: name,
            domains: RwLock::new(DomainTree::new()),
        }
    }

    pub fn add_rrset(&mut self, rrset: RRset) {
        let mut tree = self.domains.write().unwrap();
        let mut result = tree.find(&rrset.name);
        let is_delegation = !rrset.name.eq(&self.origin) && rrset.typ == r53::RRType::NS;
        if result.flag != FindResultFlag::ExacatMatch {
            let node = tree.insert(rrset.name.clone(), Some(vec![rrset])).0;
            if is_delegation {
                node.set_callback(true);
            }
        } else {
            let add_or_replace_rrset = |rrsets: &mut RRsets, rrset: RRset| {
                for (i, o) in rrsets.iter().enumerate() {
                    if o.typ == rrset.typ {
                        rrsets[i] = rrset;
                        return;
                    }
                }
                rrsets.push(rrset);
            };
            let old = result.node.get_value_mut().get_or_insert(vec![]);
            add_or_replace_rrset(old, rrset);
            if is_delegation {
                result.node.set_callback(true);
            }
        };
    }

    pub fn find(&self, name: &Name, typ: RRType, find_mode: FindMode) -> FindResult {
        let tree = self.domains.read().unwrap();
        let mut node_chain = NodeChain::new(&*tree);
        let mut result = FindResult::NXDomain;

        let mut callback = match find_mode {
            FindMode::DefaultFind => Some(|n: NodePtr<Vec<RRset>>, _, result: &mut FindResult| {
                for rrset in n.get_value().as_ref().unwrap().iter() {
                    if rrset.typ == RRType::NS {
                        *result = FindResult::Delegation(rrset.clone());
                        return true;
                    }
                }
                false
            }),
            FindMode::GlueOkFind => None,
        };
        let find_result = tree.find_node_ext(name, &mut node_chain, &mut callback, &mut result);
        match result {
            FindResult::Delegation(_) => {
                return result;
            }
            _ => match find_result.flag {
                FindResultFlag::ExacatMatch => {
                    if let Some(rrsets) = find_result.get_value() {
                        for rrset in rrsets {
                            if rrset.typ == typ {
                                return FindResult::Success(rrset.clone());
                            }
                        }
                    }
                    return FindResult::NXRRset;
                }
                _ => {
                    return FindResult::NXDomain;
                }
            },
        }
    }

    pub fn get_apex_rrset(&self, typ: RRType) -> Option<RRset> {
        let tree = self.domains.read().unwrap();
        let result = tree.find(&self.origin);
        if result.flag == FindResultFlag::ExacatMatch {
            if let Some(rrsets) = result.get_value() {
                return rrsets
                    .iter()
                    .find(|rrset| rrset.typ == typ)
                    .map(|rrset| rrset.clone());
            }
        }
        return None;
    }

    pub fn get_glue_for_ns(&self, ns: &RRset) -> Option<Vec<RRset>> {
        let mut glues = Vec::with_capacity(ns.rr_count());
        for rdata in &ns.rdatas {
            match rdata {
                r53::RData::NS(ref ns) => {
                    if ns.name.is_subdomain(&self.origin) {
                        let result = self.find(&ns.name, RRType::A, FindMode::GlueOkFind);
                        match result {
                            FindResult::Success(rrset) => glues.push(rrset),
                            _ => {}
                        }
                    }
                }
                _ => {
                    unreachable!("ns record isn't ns type");
                }
            }
        }
        Some(glues)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_zone_find() {
        let mut zone = MemoryZone::new(Name::from_str("com").unwrap());
        zone.add_rrset(RRset::from_str("com. 900 IN SOA a.gtld-servers.net. nstld.verisign-grs.com. 1653038001 1800 900 604800 86400").unwrap());

        let a_com_rrset = RRset::from_str("a.com. 900 IN A 1.1.1.1").unwrap();
        zone.add_rrset(a_com_rrset.clone());
        let b_com_ns = RRset::from_str("b.com. 900 IN NS ns1.b.com.").unwrap();
        zone.add_rrset(b_com_ns.clone());

        let a_com = Name::from_str("a.com").unwrap();
        let result = zone.find(&a_com, RRType::A);
        assert!(matches!(result, FindResult::Success(rrset) if rrset.eq(&a_com_rrset)));

        let a_b_com = Name::from_str("a.b.com").unwrap();
        let result = zone.find(&a_b_com, RRType::A);
        assert!(matches!(result, FindResult::Delegation(rrset) if rrset.eq(&b_com_ns)));
    }
}
