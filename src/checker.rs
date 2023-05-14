use std::collections::HashMap;

use indicatif::HumanDuration;
use tokio::time;

use crate::{judge::get_judges, utils::vec_of_strings};

pub struct Checker {
    pub verify_ssl: bool,

    req_proto: HashMap<String, bool>,
}

impl Checker {
    pub async fn check_judges(&mut self) {
        let stime = time::Instant::now();

        let mut judges = HashMap::new();
        let mut works = 0;
        for judge in get_judges(self.verify_ssl).await {
            if judge.is_working {
                let scheme = &judge.scheme;
                if !judges.contains_key(scheme) {
                    judges.insert(scheme.clone(), vec![]);
                }

                let v = judges.get_mut(scheme).unwrap();
                v.push(judge.clone());
                works += 1;
            }
        }
        log::debug!(
            "{} judges added, Runtime {}",
            works,
            HumanDuration(stime.elapsed())
        );

        let mut nojudges = vec![];
        let mut disable_protocols = vec![];

        for (scheme, proto) in [
            (
                "HTTP".to_string(),
                vec_of_strings!["HTTP", "CONNECT:80", "SOCKS4", "SOCKS5"],
            ),
            ("HTTPS".to_string(), vec_of_strings!["HTTPS"]),
            ("SMTP".to_string(), vec_of_strings!["SMTP"]),
        ] {
            if judges.get(&scheme).unwrap().is_empty() {
                nojudges.push(scheme.clone());
                disable_protocols.extend(proto);
                self.req_proto.insert(scheme.clone(), false);
            }
        }

        println!("{:#?}", self.req_proto);
        todo!("Implement negotiators")
    }
}

impl Default for Checker {
    fn default() -> Self {
        let mut req_proto = HashMap::new();
        req_proto.insert("HTTP".to_string(), true);
        req_proto.insert("HTTPS".to_string(), true);
        req_proto.insert("SMTP".to_string(), true);

        Checker {
            verify_ssl: false,
            req_proto,
        }
    }
}
