use r53::{Name, RRset};
use std::fmt::Display;
use std::str::FromStr;

pub fn default_soa(name: &Name) -> RRset {
    let soa = format!(
        "{} 3600 IN SOA hd.fuxi. root.fuxi. 1 600 300 2419200 600",
        name
    );
    RRset::from_str(soa.as_ref()).unwrap()
}

pub fn default_ns_and_glue<T: AsRef<str> + Display>(
    name: &Name,
    ip_addrs: &Vec<T>,
) -> (RRset, RRset) {
    let ns = format!("{} 3600 IN NS ns.{}", name, name);

    let mut glue = Vec::with_capacity(ip_addrs.len());
    for ip in ip_addrs {
        glue.push(format!("ns.{} 3600 IN A {}", name, ip));
    }

    (
        RRset::from_str(ns.as_ref()).unwrap(),
        RRset::from_strs(&glue).unwrap(),
    )
}

#[cfg(test)]
mod test {
    use super::*;
    use r53::Name;

    #[test]
    fn test_default_soa() {
        let name = Name::new("com").unwrap();
        let soa = default_soa(&name);
        assert_eq!(
            soa.to_string(),
            "com.	3600	IN	SOA	hd.fuxi. root.fuxi. 1 600 300 2419200 600\n"
        );
        let (ns, glue) = default_ns_and_glue(&name, &vec!["1.1.1.1", "2.2.2.2"]);
        assert_eq!(ns.to_string(), "com.	3600	IN	NS	ns.com.\n");
        assert_eq!(glue.rr_count(), 2);
        assert_eq!(
            glue.to_string(),
            "ns.com.	3600	IN	A	1.1.1.1\nns.com.	3600	IN	A	2.2.2.2\n"
        );
    }
}
