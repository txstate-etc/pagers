#[macro_use] extern crate lazy_static;
#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate percent_encoding;
extern crate regex;
extern crate failure;
extern crate chrono;
extern crate crossbeam_channel;
extern crate reqwest;
extern crate hyper;

pub mod repos;
pub mod nodes;
pub mod fetch;
pub mod backup;

use std::thread;
use crossbeam_channel as channel;
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

/// PREVIOUS_EXT environment variable is generally used to
/// indicate the date used the backups the previous daily backup
/// like /repo/site.20180617/...
lazy_static!{
    static ref PREVIOUS_EXT: String = {
        match env::var("PREVIOUS_EXT") {
            Ok(ext) => ext,
            Err(_) => panic!("Require an archive extension"),
        }
    };
}

fn run(backup_urls: &Vec<String>, archive_ext: &'static str, previous_ext: &'static str) {
    let primary_url = backup_urls.first().unwrap().clone();
    let (s, r) = channel::unbounded();
    for (thread_n, url) in backup_urls.iter().enumerate() {
        let thread_magnolia = Fetch::new(url).unwrap();
        let thread_r = r.clone();
        thread::spawn(move || {
            while let Some(path) = thread_r.recv() {
                //thread_magnolia.export(&path).unwrap();
                match thread_magnolia.doc_size(&path) {
                    Ok(len) => println!("INFO[{}]: size={:?}: {}", thread_n, len, backup::name(&path, archive_ext, previous_ext)),
                    Err(e) => println!("ERROR[{}]: {}{}: {}", thread_n, path.repo_type, path.path, e),
                }
            }
        });
    }

    let magnolia = Fetch::new(&primary_url).unwrap();
    if let Ok(Some(sites)) = magnolia.sites(repos::RepoType::Dam) {
        for site in sites {
            //TODO: generate archive site folder
            //println!("----------------- {}{}", site.repo_type.to_string(), site.path);
            if let Ok(Some(paths)) = magnolia.paths(&site) {
                for path in paths {
                    println!("DEBUG: path: {}", path.path);
                    s.send(path);
                }
            }
        }
    }
}

fn main() {
    run(&*BACKUP_URLS, &ARCHIVE_EXT, &PREVIOUS_EXT);
    println!("Done");
}
