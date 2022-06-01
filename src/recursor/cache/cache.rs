use super::message_cache::MessageLruCache;
use r53::{Request, Response};

pub struct MessageCache {
    positive_cache: MessageLruCache,
    negative_cache: MessageLruCache,
}

impl MessageCache {
    pub fn new(cap: usize) -> Self {
        debug_assert!(cap > 0);
        MessageCache {
            positive_cache: MessageLruCache::new(cap),
            negative_cache: MessageLruCache::new(cap),
        }
    }

    pub fn len(&self) -> usize {
        self.positive_cache.len() + self.negative_cache.len()
    }

    pub fn gen_response(&mut self, req: &Request) -> Option<Response> {
        let response = self.positive_cache.gen_response(req);
        if response.is_none() {
            self.negative_cache.gen_response(req)
        } else {
            response
        }
    }

    pub fn add_response(&mut self, resp: Response) {
        if resp.header.an_count > 0 {
            self.positive_cache.add_response(resp);
        } else {
            self.negative_cache.add_response(resp);
        }
    }
}
