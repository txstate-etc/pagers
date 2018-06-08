#[macro_use] extern crate lazy_static;
#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate percent_encoding;
extern crate regex;
extern crate failure;
extern crate chrono;
extern crate reqwest;
extern crate hyper;

pub mod repos;
pub mod nodes;
pub mod fetch;
pub mod backup;

use nodes::PathInfo;
use std::env;
use fetch::Fetch;

/// BACKUP_URLS is a comma delimited list of the cluster
/// used to backup the data.
/// https://usr:pwd@host:port/path,https://usr:pwd@host:port/path,...
lazy_static!{
    static ref BACKUP_URLS: Vec<String> = {
        match env::var("BACKUP_URLS") {
            Ok(urls) => urls.split(",").map(|s| s.into()).collect(),
            Err(_) => panic!("Require list of URLs"),
        }
    };
}

/// ARCHIVE_EXT environment variable is generally used to
/// date the backups for that day like /repo/site.20180618/...
lazy_static!{
    static ref ARCHIVE_EXT: String = {
        match env::var("ARCHIVE_EXT") {
            Ok(ext) => ext,
            Err(_) => panic!("Require an archive extension"),
        }
    };
}

fn main() {
    let magnolia = Fetch::new(&*BACKUP_URLS[0]).unwrap();
    if let Ok(Some(sites)) = magnolia.sites(repos::RepoType::Dam) {
        for site in sites {
            println!("----------------- {}{}", site.repo_type.to_string(), site.path);
            if let Ok(Some(paths)) = magnolia.paths(&site) {
                for path in paths {
                    //magnolia.export(nodes::PathInfo{repo_type: repos::RepoType::Dam, path: "/banner-images/JavelinaStampBWlarge.jpg".to_string(), last_modified: None}).unwrap();
                    println!("    {}", backup::name(&site, &path, "20180607"));
                }
            }
        }
    }
}
