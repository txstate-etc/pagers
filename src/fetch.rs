use std::fmt::Display;
use repos::RepoType;
use nodes::{self, Paths, PathInfo};
use failure::{Error, err_msg};
//use chrono;
use percent_encoding::percent_decode;
use regex::Regex;
use reqwest::{Client, RedirectPolicy, StatusCode};
use hyper::header::{Cookie, Referer, Accept, qitem, ContentLength};
use hyper::{Uri, mime};
use std::io::Read;

lazy_static!{
    static ref RE_JSESSIONID: Regex = Regex::new(r"^JSESSIONID=([A-F0-9]{32})[; ]").unwrap();
}

#[derive(Debug, Fail)]
pub enum FetchError {
    // 302 redirect (300-399): Immediate Retry to Reset Session
    //   a few times before this Blocks future requests
    #[fail(display = "Lost session: {}", error)]
    LostSession {
        error: String,
    },

    // 400-499 Client Errors
    #[fail(display = "Blocking error type: {}", error)]
    Blocking {
        error: String,
    },

    // 500-599 Server Error: Backoff a few times before this Blocks future requests
    #[fail(display = "Backoff error type: {}", error)]
    BackOff {
        error: String,
    },

// NOTE: All other errors should be logged and the request skipped.
//   i.e. No immediate retry, backoff, or Blocking all future requests.
//  Skip Request which gave reqwest::ClientBuilder::send()? request failure
//  Skip Request which gave nodes::build_paths()? parsing failure
    #[fail(display = "Skip error type: {}", error)]
    Skip {
        error: String,
    },
}

fn new_fetch_error<D: Display, T>(status_code: Option<StatusCode>, text: D) -> Result<T, FetchError> {
    if let Some(status_code) = status_code {
        if status_code.is_redirection() {
            Err(FetchError::LostSession{error: format!("{}; {}", status_code, text)})
        } else if status_code.is_client_error() {
            Err(FetchError::Blocking{error: format!("{}; {}", status_code, text)})
        } else if status_code.is_server_error() {
            Err(FetchError::BackOff{error: format!("{}; {}", status_code, text)})
        } else {
            Err(FetchError::Skip{error: format!("{}; {}", status_code, text)})
        }
    } else {
        Err(FetchError::Skip{error: text.to_string()})
    }
}

fn new_fetch_error_skip<D: Display, T>(text: D) -> Result<T, FetchError> {
    new_fetch_error(None, text)
}

// authority is in the form Option<"user:password@host">
fn split_authority(authority: Option<&str>) -> Result<(String, String), Error> {
    if let Some(auth) = authority {
        if let Some(at) = auth.find("@") {
            let (split_base, _host) = auth.split_at(at);
            let mut up = split_base.splitn(2, ":");
            let up = (up.next(), up.next());
            if let (Some(u), Some(p)) = up {
                let u = percent_decode(u.as_bytes()).decode_utf8().unwrap();
                let p = percent_decode(p.as_bytes()).decode_utf8().unwrap();
                return Ok((u.into(), p.into()));
            }
        }
    }
    Err(err_msg("Invalid Authority Segment"))
}

pub struct Fetch {
    url: String,
    user: String,
    password: String,
    session: Option<String>,
    client: Client,
}

impl Fetch {
    fn new_session(&mut self) -> Result<(), Error> {
        // If are not managing cookies in our request and do not provide a valid path
        // we will get a redirect which generates an extra session that never gets
        // used. The last session from the final response returns the authenticated
        // session: /^Set-Cookie: JSESSIONID=([A-F0-9]{32}).*$/
        // Example: Set-Cookie: JSESSIONID=9BE61261AC5D7F7AED81F84963CE9430; Path=/; HttpOnly
        // NOTE: Turned off Redirection as Reqwest default is to follow the chain
        // of at least 10 redirects before returning an error.
        let url = format!("{}/.magnolia/admincentral", self.url.to_string());
        let resp = self.client.get(&url)
            .basic_auth(&*self.user, Some(&*self.password))
            .send()?;
        if !resp.status().is_success() {
            return Err(err_msg("Unable to retrieve a session. Invalid status."))
        }
        for header_view in resp.headers().iter() {
            if header_view.name() == "Set-Cookie" {
                let header_value = header_view.value_string();
                if let Some(session) = RE_JSESSIONID.find(&header_value) {
                    let (_, session) = session.as_str().split_at(11);
                    self.session = Some(session.to_string());
                    return Ok(());
                }
            }
        }
        return Err(err_msg("Unable to retrieve a session. No Session in header."))
    }

    pub fn new_client(&mut self) -> Result<(), Error> {
        if let Ok(client) = Client::builder().redirect(RedirectPolicy::none()).build() {
            self.client = client;
            self.new_session()?;
            Ok(())
        } else {
            Err(err_msg("Unable to build http client."))
        }
    }

    pub fn new(url: &str) -> Result<Fetch, Error> {
        let uri = url.parse::<Uri>().unwrap();
        // Extract user and password as we only want to
        // use those to initialy generate a session.
        let url = format!("{}://{}:{}{}", uri.scheme().unwrap(), uri.host().unwrap(), uri.port().unwrap(), uri.path().trim_right_matches("/"));
        let (user, password) = split_authority(uri.authority())?;
        let mut fetch = Fetch{
            url: url.to_string(),
            user: user.to_string(),
            password: password.to_string(),
            session: None,
            client: Client::new(),
         };
         fetch.new_client()?;
         Ok(fetch)
    }

    /// Fetch list of sites within a repo:
    ///   NOTE: Exclude 'mgnl:resources' from magnolia RESTful json responses as they include binary data we do NOT require.
    ///   curl -s -H 'Accept: application/json' '<url>/.rest/nodes/v1/<repo>?depth=1&excludeNodeTypes=mgnl:resource'
    pub fn sites(&self, repo_type: RepoType) -> Result<Option<Paths>, FetchError> {
        let mut cookie_session = Cookie::new();
        cookie_session.append("JSESSIONID", self.session.as_ref().unwrap().to_string());
        let url = format!("{}/.rest/nodes/v1/{}?depth=1&excludeNodeTypes=mgnl:resource", self.url, repo_type);
        let resp = self.client.get(&url)
            .header(cookie_session)
            .header(Accept(vec![qitem(mime::APPLICATION_JSON)]))
            .send()
            .or_else(new_fetch_error_skip)?;
        if resp.status().is_success() {
            Ok(nodes::build_paths(resp, repo_type, true).or_else(new_fetch_error_skip)?)
        } else {
            new_fetch_error(Some(resp.status()), "Unable to retrive list of sites")
        }
    }

    /// Fetch list of node paths for all sites within a repo and include the associated mgnl:lastModified properties found in nodes Metadata:
    ///   NOTE: Exclude 'mgnl:resources' from magnolia RESTful json responses as they include binary data we do NOT require.
    /// For all paths within repo while NOT including mgnl:folders
    ///   curl -s -H 'Accept: application/json' '<url>/.rest/nodes/v1/<repo>/<site/path>?depth=999&excludeNodeTypes=mgnl:resource&includeMetadata=true'
    pub fn paths(&self, path_info: &PathInfo) -> Result<Option<Paths>, FetchError> {
        let mut cookie_session = Cookie::new();
        cookie_session.append("JSESSIONID", self.session.as_ref().unwrap().to_string());
        let url = format!("{}/.rest/nodes/v1/{}{}?depth=999&excludeNodeTypes=mgnl:resource&includeMetadata=true", self.url, path_info.repo_type, path_info.path);
        let resp = self.client.get(&url)
            .header(cookie_session)
            .header(Accept(vec![qitem(mime::APPLICATION_JSON)]))
            .send()
            .or_else(new_fetch_error_skip)?;
        if resp.status().is_success() {
            Ok(nodes::build_paths(resp, path_info.repo_type, false).or_else(new_fetch_error_skip)?)
        } else {
            new_fetch_error(Some(resp.status()), "Unable to retrieve list of paths")
        }
    }

    /// Was going to have magnolia return back Content-Length of path, however,
    /// both export.jsp and restful interfaces only return back chunked responses
    /// which does NOT contain a content length header. Instead we will request
    /// the actual document. WARN: This may only work for dam.
    pub fn doc_size(&self, path_info: &PathInfo) -> Result<Option<u64>, FetchError> {
        let mut cookie_session = Cookie::new();
        cookie_session.append("JSESSIONID", self.session.as_ref().unwrap().to_string());
        let url = format!("{}/{}{}", self.url, path_info.repo_type, path_info.path);
        let resp = self.client.head(&url)
            .header(cookie_session)
            .send()
            .or_else(new_fetch_error_skip)?;
        if resp.status().is_success() {
            Ok(resp.headers().get::<ContentLength>().map(|ct_len| **ct_len))
        } else {
            new_fetch_error(Some(resp.status()), "Unable to retrieve document")
        }
    }

    // Fetch site/node export and save to file under <repo>/<site_from_path>-YYYYMMDD/<repo>.<path>.xml with modified time of file updated with :
    //   NOTE: Exports should only receive a 200 status as a redirect to CAS login implies that the session was lost.
    //     Apparently if the export is large enough it takes longer to generate the XML file then the tomcat default
    //     30 minute session inactive timeout period. This is a regression as we fixed this before in our version of
    //     Magnolia where we stream the raw unfiltered XML file out without first saving it to disk which Magnolia
    //     does by default, even though versions do NOT exist within a site that would require filtering. The
    //     'mgnlKeepVersions=true' option is left incase we fix this again.
    //   NOTE: Exports may also return a 500 status. If the site is large enough it can fill up the temporary
    //     directory causing other export requests running in parallel to fail. Once they fail Magnolia will not
    //     clear out the associated temporary files which can build up.
    //   NOTE: Magnolia CMS gzip responses have a 2GB limit so do not accept gzip content to avoid this issue.
    //     The curl example by default will not request the reponse to be compressed unless we add the following
    //     header: -H 'Accept-Encoding: gzip,deflate'
    //   NOTE: Export requests require a 'referer' header as a security feature.
    //   NOTE: An export jsp was added as Magnolia put their export features behind an interactive Vaadin framework.
    //   curl -s --fail --cookie '<SessionID>' \
    //     '<URL>/docroot/gato/export.jsp?repo=<repo>&path=</path>'
    pub fn export(&self, path_info: &PathInfo) -> Result<Box<Read>, FetchError> {
        let mut cookie_session = Cookie::new();
        cookie_session.append("JSESSIONID", self.session.as_ref().unwrap().to_string());
        let url = format!("{}/docroot/gato/export.jsp", &self.url);
        let resp = self.client.get(&url)
            .header(cookie_session)
            .header(Accept(vec![qitem(mime::TEXT_XML)]))
            .header(Referer::new(url))
            .query(&[
                ("repo", path_info.repo_type.to_string()),
                ("path", path_info.path.clone()),
            ])
            .send()
            .or_else(new_fetch_error_skip)?;
        if resp.status().is_success() {
            Ok(Box::new(resp))
        } else {
            new_fetch_error(Some(resp.status()), "Unable to retrieve export")
        }
    }
}
