#[macro_use] extern crate lazy_static;
#[macro_use] extern crate serde_derive;
extern crate serde;
#[macro_use] extern crate failure_derive;
extern crate failure;
extern crate serde_json;
extern crate percent_encoding;
extern crate regex;
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
use std::time::Duration;
use crossbeam_channel as channel;
use std::env;
use std::io;
use fetch::{Fetch, FetchError};
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
    let (s, r) = channel::bounded(backup_urls.len());
    for (thread_n, url) in backup_urls.iter().enumerate() {
        let thread_r = r.clone();
        let thread_url = url.clone();
        thread::spawn(move || {
            let mut magnolia = Fetch::new(&thread_url).unwrap();
            for path in thread_r {
                // if previous file exists and has matching modified times then hard link, else create a new entry
                let previous_file = format!("{}/{}", backup::archive_path(archive_dir, previous_ext, &path), backup::backup_filename(&path));
                let archive_file = format!("{}/{}", backup::archive_path(archive_dir, archive_ext, &path), backup::backup_filename(&path));
                if let Ok(p_meta) = fs::metadata(&previous_file) {
                    if let Ok(p_modified) = p_meta.modified() {
                        if Some(DateTime::from(p_modified)) == path.last_modified {
                            if let Err(e) = fs::hard_link(&previous_file, &archive_file) {
                                println!("ERROR[{}]: {}, {}", thread_n, &path.path, e);
                            }
                            let timestamp = FileTime::from_unix_time(path.last_modified.unwrap().timestamp(), path.last_modified.unwrap().timestamp_subsec_nanos());
                            if let Err(e) = set_file_times(&archive_file, timestamp, timestamp) {
                                println!("ERROR[{}]: {}, {}", thread_n, &path.path, e);
                            }
                            continue;
                        }
                    }
                }
                if let Ok(mut file) = File::create(&archive_file) {
                    loop {
                        match magnolia.export(&path) {
                            Ok(mut export) => {
                                match io::copy(&mut export, &mut file) {
                                    Ok(size) => {
                                        let timestamp = FileTime::from_unix_time(path.last_modified.unwrap().timestamp(), path.last_modified.unwrap().timestamp_subsec_nanos());
                                        if let Err(e) = set_file_times(&archive_file, timestamp, timestamp) {
                                            println!("ERROR[{}]: {}, {}", thread_n, &path.path, e);
                                        } else {
                                            println!("INFO[{}]: Exported {} bytes {}", thread_n, size, &path.path);
                                        }
                                    },
                                    // TODO: Remove file if bad copy.
                                    Err(e) => println!("ERROR[{}]: Export failed {}, {}", thread_n, &path.path, e),
                                }
                                break;
                            },
                            Err(FetchError::LostSession{error: e}) => {
                                println!("WARN[{}]: {}, session: {:?}, {}", thread_n, &path.path, magnolia.session, e);
                                if let Err(e) = magnolia.new_client() {
                                    println!("ERROR[{}]: {}, {}", thread_n, &path.path, e);
                                    return;
                                }
                            },
                            Err(FetchError::BackOff{error: e}) => {
                                println!("WARN[{}]: {}, session: {:?} {}", thread_n, &path.path, magnolia.session, e);
                                thread::sleep(Duration::new(15, 0));
                                // Reset connection and renew session as magnolia cannot
                                // recover a persistent connection after a server error
                                // Also it looks like this specific request if retried
                                // will keep generating 500's so skipping after a pause
                                // rather then only backing off and retrying.
                                // TODO: at some point if we find other issues that are
                                // recoverable with a retry then we may want to retry
                                // one more time.
                                if let Err(e) = magnolia.new_client() {
                                    println!("ERROR[{}]: {}, {}", thread_n, &path.path, e);
                                    return;
                                }
                                break;
                            },
                            Err(FetchError::Skip{error: e}) => {
                                println!("ERROR[{}]: {}, {}", thread_n, &path.path, e);
                                break;
                            },
                            Err(FetchError::Blocking{error: e}) => {
                                println!("ERROR[{}]: {}, {}", thread_n, &path.path, e);
                                return;
                            },
                        }
                    }
                }
            }
        });
    }

    let mut magnolia = Fetch::new(&primary_url).unwrap();
    if let Ok(Some(sites)) = magnolia.sites(repos::RepoType::Dam) {
        for site in sites {
            let archive_path = backup::archive_path(archive_dir, archive_ext, &site);
            match DirBuilder::new().recursive(true).create(&archive_path) {
                Ok(()) => {
                    loop {
                        match magnolia.paths(&site) {
                            Ok(Some(paths)) => {
                                for path in paths {
                                    //println!("DEBUG[m]: dam: {} path: {}", path.repo_type, path.path);
                                    s.send(path);
                                }
                                break;
                            },
                            Ok(None) => {
                                println!("INFO[m]: No paths for site {}", &site.path);
                                break;
                            },
                            Err(FetchError::LostSession{error: e}) => {
                                println!("WARN[m]: {}, session: {:?}, {}", &site.path, magnolia.session, e);
                                if let Err(e) = magnolia.new_client() {
                                    println!("ERROR[m]: {}, {}", &site.path, e);
                                    return;
                                }
                            },
                            Err(FetchError::BackOff{error: e}) => {
                                // TODO: Exponential backoff currently waits 15 seconds.
                                // NOTE: Retry behavior basically removed as
                                // some requests will always generate 500's,
                                // basically turning this into a skip after delay.
                                println!("WARN[m]: {}, session: {:?}, {}", &site.path, magnolia.session, e);
                                thread::sleep(Duration::new(15, 0));
                                // Reset connection and renew session as magnolia cannot
                                // recover a persistent connection after a server error
                                if let Err(e) = magnolia.new_client() {
                                    println!("ERROR[m]: {}, {}", &site.path, e);
                                    return;
                                }
                                break;
                            },
                            Err(FetchError::Skip{error: e}) => {
                                println!("ERROR[m]: {}, {}", &site.path, e);
                                break;
                            },
                            Err(FetchError::Blocking{error: e}) => {
                                println!("ERROR[m]: {}, {}", &site.path, e);
                                return;
                            },
                        }
                    }
                },
                Err(e) => println!("ERROR[m]: NOT able to create archive directory: {}, {}", archive_path, e),
            }
        }
    } else {
        println!("ERROR[m]: Unable to retrieve sites for repo {}", repos::RepoType::Dam);
    }
    drop(s);
    // wait up to 5 minutes to end main running thread
    let mut force = true;
    for _ in 0..10 {
        if r.is_empty() {
            force = false;
            break;
        }
        thread::sleep(Duration::new(30, 0))
    }
    if force {
        println!("ERROR[m]: Forced to terminate main thread with outstanding requests");
    }
}

fn main() {
    run(&*BACKUP_URLS, &ARCHIVE_DIR, &ARCHIVE_EXT, &PREVIOUS_EXT);
    println!("Done");
}
