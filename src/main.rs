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

use std::env;
use fetch::Fetch;

lazy_static!{
    static ref BACKUP_URLS: Vec<String> = {
        match env::var("BACKUP_URLS") {
            Ok(urls) => urls.split(",").map(|s| s.into()).collect(),
            Err(_) => panic!("Require list of URLs"),
        }
    };
}

fn main() {
    let magnolia = Fetch::new(&*BACKUP_URLS[0]).unwrap();
    let repo_sites = magnolia.sites(repos::RepoType::Dam).unwrap();
    println!("{:?}", repo_sites);
}
