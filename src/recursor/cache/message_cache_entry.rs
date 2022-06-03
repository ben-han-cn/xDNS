use super::entry_key::EntryKey;
use r53::{
    header_flag::HeaderFlag, Name, RRTtl, RRType, RRset, Rcode, Request, Response, ResponseBuilder,
    SectionType,
};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct MessageEntry {
    rcode: Rcode,
    answer_rrset_count: u16,
    auth_rrset_count: u16, //for soa
    rrsets: Vec<RRset>,
    init_time: Instant,
    expire_time: Instant,
}

unsafe impl Send for MessageEntry {}

impl MessageEntry {
    pub fn new(resp: &mut Response) -> Self {
        let answer_rrset_count = resp
            .section(SectionType::Answer)
            .map_or(0, |rrsets| rrsets.len() as u16);

        let auth_rrset_count = if answer_rrset_count == 0 {
            resp.section(SectionType::Authority)
                .map_or(0, |rrsets| rrsets.len() as u16)
        } else {
            0
        };

        let now = Instant::now();
        let mut entry = MessageEntry {
            rcode: resp.header.rcode,
            answer_rrset_count,
            auth_rrset_count,
            rrsets: Vec::with_capacity((answer_rrset_count + auth_rrset_count) as usize),
            init_time: now,
            expire_time: now,
        };

        let mut min_ttl = RRTtl(u32::max_value());
        if answer_rrset_count > 0 {
            entry.add_section(resp, SectionType::Answer, &mut min_ttl);
        }
        if auth_rrset_count > 0 {
            entry.add_section(resp, SectionType::Authority, &mut min_ttl);
        }
        entry.expire_time = entry
            .expire_time
            .checked_add(Duration::from_secs(min_ttl.0 as u64))
            .unwrap();
        entry
    }

    fn add_section(&mut self, resp: &mut Response, section: SectionType, min_ttl: &mut RRTtl) {
        for rrset in resp.take_section(section).unwrap().into_iter() {
            if rrset.ttl.0 < min_ttl.0 {
                *min_ttl = rrset.ttl;
            }
            self.rrsets.push(rrset);
        }
    }

    #[inline]
    pub fn is_expired(&self) -> bool {
        self.expire_time <= Instant::now()
    }

    pub fn gen_response(&self, req: &Request) -> Option<Response> {
        let now = Instant::now();
        if self.expire_time < now {
            return None;
        }

        let elapsed = self.init_time.elapsed().as_secs();
        let mut resp = Response::with_question(req.question.name.clone(), req.question.typ);
        let mut builder = ResponseBuilder::new(&mut resp);
        builder
            .id(req.header.id)
            .make_response()
            .set_flag(HeaderFlag::RecursionAvailable)
            .rcode(self.rcode);
        let mut iter = self.rrsets.clone().into_iter();

        let decrease_ttl = |rrset: &mut RRset| {
            let mut new_ttl = rrset.ttl.0.checked_sub(elapsed as u32).unwrap();
            if new_ttl == 0 {
                new_ttl = 1;
            }
            rrset.ttl = RRTtl(new_ttl);
        };

        for _ in 0..self.answer_rrset_count {
            let mut rrset = iter.next().unwrap();
            decrease_ttl(&mut rrset);
            builder.add_rrset(SectionType::Answer, rrset);
        }

        for _ in 0..self.auth_rrset_count {
            let mut rrset = iter.next().unwrap();
            decrease_ttl(&mut rrset);
            builder.add_rrset(SectionType::Authority, rrset);
        }
        builder.done();
        Some(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r53::{build, Rcode};

    fn build_positive_response() -> Response {
        let mut resp = build(
            "test.example.com.",
            RRType::A,
            vec![vec![
                "test.example.com. 3600 IN A 192.0.2.2",
                "test.example.com. 3600 IN A 192.0.2.1",
            ]],
            vec![vec!["example.com. 10 IN NS ns1.example.com."]],
            vec![vec!["ns1.example.com. 3600 IN A 2.2.2.2"]],
            Some(4096),
        )
        .unwrap();

        let mut builder = ResponseBuilder::new(&mut resp);
        builder
            .id(1200)
            .rcode(Rcode::NoError)
            .set_flag(HeaderFlag::RecursionDesired)
            .done();
        resp
    }

    fn build_negative_response() -> Response {
        let mut resp= build(
            "test.example.com.",
            RRType::A,
            vec![],
            vec![vec!["example.com. 30 IN SOA a.gtld-servers.net. nstld.verisign-grs.com. 1563935574 1800 900 604800 86400"]],
            vec![],
            Some(4096),
        )
        .unwrap();

        let mut builder = ResponseBuilder::new(&mut resp);
        builder
            .id(1200)
            .rcode(Rcode::NXDomain)
            .set_flag(HeaderFlag::RecursionDesired)
            .done();
        resp
    }
    #[test]
    fn test_positive_message() {
        let resp = build_positive_response();
        let entry = MessageEntry::new(&mut resp.clone());
        assert_eq!(entry.answer_rrset_count, 1);
        assert_eq!(entry.auth_rrset_count, 0);
        assert_eq!(entry.rrsets.len(), 1);
        assert!(
            entry.expire_time
                <= Instant::now()
                    .checked_add(Duration::from_secs(3600))
                    .unwrap()
        );

        let req = Request::new(Name::new("test.example.com.").unwrap(), RRType::A);
        let response = entry.gen_response(&req).unwrap();
        assert_eq!(response.header.qd_count, resp.header.qd_count);

        let gen_message_sections = response.section(SectionType::Answer).unwrap();
        for (i, rrset) in resp
            .section(SectionType::Answer)
            .unwrap()
            .iter()
            .enumerate()
        {
            assert_eq!(rrset.typ, gen_message_sections[i].typ);
            assert_eq!(rrset.rdatas, gen_message_sections[i].rdatas);
            assert_eq!(rrset.name, gen_message_sections[i].name);
            assert!(rrset.ttl.0 > gen_message_sections[i].ttl.0);
        }
    }

    #[test]
    fn test_negative_message() {
        let resp = build_negative_response();
        let entry = MessageEntry::new(&mut resp.clone());
        assert_eq!(entry.answer_rrset_count, 0);
        assert_eq!(entry.auth_rrset_count, 1);
        assert_eq!(entry.rrsets.len(), 1);
        assert!(entry.expire_time < Instant::now().checked_add(Duration::from_secs(30)).unwrap());
        assert!(entry.expire_time > Instant::now().checked_add(Duration::from_secs(20)).unwrap());

        let req = Request::new(Name::new("test.example.com.").unwrap(), RRType::A);
        let response = entry.gen_response(&req).unwrap();
        assert_eq!(response.header.qd_count, resp.header.qd_count);
        assert_eq!(response.header.an_count, resp.header.an_count);
        assert_eq!(response.header.ns_count, resp.header.ns_count);

        for section in vec![SectionType::Authority] {
            let gen_message_sections = response.section(section).unwrap();
            for (i, rrset) in resp.section(section).unwrap().iter().enumerate() {
                assert_eq!(rrset.typ, gen_message_sections[i].typ);
                assert_eq!(rrset.rdatas, gen_message_sections[i].rdatas);
                assert_eq!(rrset.name, gen_message_sections[i].name);
                assert!(rrset.ttl.0 >= gen_message_sections[i].ttl.0);
            }
        }
    }
}
