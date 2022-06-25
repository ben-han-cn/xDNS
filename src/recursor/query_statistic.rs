use chrono::Local;
use lru::LruCache;
use r53::Name;
use serde::Serialize;
use std::mem::swap;

pub struct QueryStatistic {
    queries: LruCache<Name, u64>,
}

#[derive(Serialize, Debug, Default)]
pub struct QueryInfo {
    #[serde(rename = "key")]
    pub time: String,
    #[serde(rename = "value")]
    pub top: String,
}

impl QueryStatistic {
    pub fn new(cap: usize) -> Self {
        QueryStatistic {
            queries: LruCache::new(cap),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.queries.len()
    }

    pub fn add_query(&mut self, name: &Name) {
        if let Some(c) = self.queries.get_mut(name) {
            *c += 1
        } else {
            self.queries.put(name.clone(), 1);
        }
    }

    pub fn sort_and_clear(&mut self) -> QueryInfo {
        let mut new = LruCache::new(self.queries.cap());
        swap(&mut self.queries, &mut new);
        let mut info = new
            .into_iter()
            .map(|(n, c)| (n.to_string(), c))
            .collect::<Vec<(String, u64)>>();
        info.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        QueryInfo {
            time: Local::now().format("%Y-%m-%d-%H-%M-%S").to_string(),
            top: serde_json::to_string(&info).unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r53::Name;

    #[test]
    fn test_query_info() {
        let mut info = QueryStatistic::new(10);
        let a_com = Name::new("a.com.").unwrap();
        let b_com = Name::new("b.com.").unwrap();
        let c_com = Name::new("c.com.").unwrap();
        info.add_query(&a_com);
        info.add_query(&a_com);
        info.add_query(&a_com);
        info.add_query(&b_com);
        info.add_query(&b_com);
        info.add_query(&c_com);
        let v = info.sort_and_clear();
        assert_eq!(v.len(), 3);
        assert_eq!(v[0].1, 3);
        assert_eq!(v[1].1, 2);
        assert_eq!(v[2].1, 1);
    }
}
