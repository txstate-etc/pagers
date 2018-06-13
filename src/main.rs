#[macro_use] extern crate lazy_static;
#[macro_use] extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate percent_encoding;
extern crate regex;
extern crate failure;
extern crate chrono;
extern crate filetime;
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
use std::io;
use fetch::Fetch;
use std::fs::{self, DirBuilder, File};
use chrono::DateTime;
use filetime::{set_file_times, FileTime};

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

/// ARCHIVE_DIR envirnomen variable holds the backup location
/// in the filesystem.
lazy_static!{
    static ref ARCHIVE_DIR: String = {
        match env::var("ARCHIVE_DIR") {
            Ok(dir) => dir,
            Err(_) => panic!("Require an archive directory"),
        }
    };
}

fn run(backup_urls: &Vec<String>, archive_dir: &'static str, archive_ext: &'static str, previous_ext: &'static str) {
    let primary_url = backup_urls.first().unwrap().clone();
    let (s, r) = channel::unbounded();
    for (thread_n, url) in backup_urls.iter().enumerate() {
        let thread_magnolia = Fetch::new(url).unwrap();
        let thread_r = r.clone();
        thread::spawn(move || {
            for path in thread_r {
                // if previous file exists and has matching modified times then hard link, else create a new entry
                let previous_file = format!("{}/{}", backup::archive_path(archive_dir, previous_ext, &path), backup::backup_filename(&path));
                let archive_file = format!("{}/{}", backup::archive_path(archive_dir, archive_ext, &path), backup::backup_filename(&path));
                if let Ok(p_meta) = fs::metadata(&previous_file) {
                     if let Ok(p_modified) = p_meta.modified() {
                         if Some(DateTime::from(p_modified)) == path.last_modified {
                             if let Err(e) = fs::hard_link(&previous_file, &archive_file) {
                                 println!("ERROR: {}, {}", &path.path, e);
                             }
                             let timestamp = FileTime::from_unix_time(path.last_modified.unwrap().timestamp(), path.last_modified.unwrap().timestamp_subsec_nanos());
                             if let Err(e) = set_file_times(&archive_file, timestamp, timestamp) {
                                 println!("ERROR: {}, {}", &path.path, e);
                             }
                             continue;
                         }
                     }
                }
                if let Ok(mut file) = File::create(archive_file) {
                    if let Ok(mut export) = thread_magnolia.export(&path) {
                        match io::copy(&mut export, &mut file) {
                            Ok(size) => println!("INFO[{}] exported {} bytes {}", thread_n, size, &path.path), 
                            Err(e) => println!("ERROR[{}] failed {}, {}", thread_n, &path.path, e),
                        }
                    }
                }
            }
        });
    }

    let magnolia = Fetch::new(&primary_url).unwrap();
    if let Ok(Some(sites)) = magnolia.sites(repos::RepoType::Dam) {
        for site in sites {
            //TODO: generate archive site folder
            let archive_path = backup::archive_path(archive_dir, archive_ext, &site);
            match DirBuilder::new().recursive(true).create(archive_path) {
                Ok(()) => if let Ok(Some(paths)) = magnolia.paths(&site) {
                        for path in paths {
                            //println!("DEBUG: dam: {} path: {}", path.repo_type, path.path);
                            s.send(path);
                        }
                    } else {
                        println!("ERROR: NOT able to retrieve paths for repo");
                    },
                Err(_) => println!("ERROR NOT able to create archive directory"),
            }
        }
    }
}

fn main() {
    run(&*BACKUP_URLS, &ARCHIVE_DIR, &ARCHIVE_EXT, &PREVIOUS_EXT);
    println!("Done");
}
