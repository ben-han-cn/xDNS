use super::{entry_key::EntryKey, message_cache_entry::MessageEntry};
use lru::LruCache;
use r53::{Request, Response};

const DEFAULT_MESSAGE_CACHE_SIZE: usize = 10000;

pub struct MessageLruCache {
    responses: LruCache<EntryKey, MessageEntry>,
}

impl MessageLruCache {
    pub fn new(cap: usize) -> Self {
        MessageLruCache {
            responses: LruCache::new(if cap == 0 {
                DEFAULT_MESSAGE_CACHE_SIZE
            } else {
                cap
            }),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.responses.len()
    }

    pub fn gen_response(&mut self, req: &Request) -> Option<Response> {
        let key = EntryKey::from_question(&req.question);
        if let Some(entry) = self.responses.get(&key) {
            entry.gen_response(req)
        } else {
            None
        }
    }

    pub fn add_response(&mut self, mut resp: Response) {
        let key = EntryKey::from_question(&resp.question);
        if let Some(entry) = self.responses.get(&key) {
            if !entry.is_expired() {
                return;
            }
        }

        let entry = MessageEntry::new(&mut resp);
        let key = EntryKey::new(resp.question.name, resp.question.typ);
        self.responses.put(key, entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r53::{build, header_flag, Name, RRType, Rcode, ResponseBuilder, SectionType};

    fn build_positive_response() -> Response {
        let mut resp = build(
            "test.example.com.",
            RRType::A,
            vec![vec![
                "test.example.com. 3600 IN A 192.0.2.2",
                "test.example.com. 3600 IN A 192.0.2.1",
            ]],
            vec![vec!["example.com. 100 IN NS ns1.example.com."]],
            vec![vec!["ns1.example.com. 3600 IN A 2.2.2.2"]],
            Some(4096),
        )
        .unwrap();
        let mut builder = ResponseBuilder::new(&mut resp);
        builder
            .id(1200)
            .rcode(Rcode::NoError)
            .set_flag(header_flag::HeaderFlag::RecursionDesired)
            .done();
        resp
    }

    #[test]
    fn test_message_cache() {
        let mut cache = MessageLruCache::new(100);
        let req = Request::new(Name::new("test.example.com.").unwrap(), RRType::A);
        assert!(cache.gen_response(&req).is_none());
        cache.add_response(build_positive_response());
        let response = cache.gen_response(&req).unwrap();
        assert_eq!(response.header.rcode, Rcode::NoError);
        assert!(header_flag::is_flag_set(
            response.header.flag,
            header_flag::HeaderFlag::QueryRespone
        ));
        assert!(!header_flag::is_flag_set(
            response.header.flag,
            header_flag::HeaderFlag::AuthenticData
        ));
        assert_eq!(response.header.an_count, 2);
        let answers = response.section(SectionType::Answer).unwrap();
        assert_eq!(answers.len(), 1);
        assert_eq!(answers[0].rdatas[0].to_string(), "192.0.2.2");

        let req = Request::new(Name::new("example.com.").unwrap(), RRType::NS);
        assert!(cache.gen_response(&req).is_none());
    }
}
