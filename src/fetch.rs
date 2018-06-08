use std::io;
use repos::RepoType;
use nodes::{self, Paths, PathInfo};
use failure::{Error, err_msg};
//use chrono;
use percent_encoding::percent_decode;
use regex::Regex;
use reqwest::Client;
use hyper::header::{Cookie, Referer, Accept, qitem};
use hyper::{Uri, mime};

lazy_static!{
    static ref RE_JSESSIONID: Regex = Regex::new(r"^JSESSIONID=([A-F0-9]{32})[; ]").unwrap();
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
                Ok((u.into(), p.into()))
            } else {
                Err(err_msg("Authority requires a user and password"))
            }
        } else {
            Err(err_msg("Authority requires a user, password, and domain"))
        }
    } else {
        Err(err_msg("Invalid Authority"))
    }
}

pub struct Fetch {
    url: String,
    user: String,
    password: String,
    session: Option<String>,
    client: Client,
}

impl Fetch {
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
         fetch.set_session()?;
         Ok(fetch)
    }

    fn set_session(&mut self) -> Result<(), Error> {
        // If are not managing cookies in our request and do not provide a valid path
        // we will get a redirect which generates an extra session that never gets
        // used. The last session from the final response returns the authenticated
        // session: /^Set-Cookie: JSESSIONID=([A-F0-9]{32}).*$/
        // Example: Set-Cookie: JSESSIONID=9BE61261AC5D7F7AED81F84963CE9430; Path=/; HttpOnly
        // NOTE: If the path changes the reqwest default is to follow the chain
        // of at least 10 redirects before returning an error; so we may never
        // notice that we need to update the default path to obtain a session.
        let url = format!("{}/.magnolia/admincentral", self.url.to_string());
        let resp = self.client.get(&url)
            .basic_auth(&*self.user, Some(&*self.password))
            .send()?;
        if !resp.status().is_success() {
            println!("Status: {}", resp.status());
            return Err(err_msg("Invalid response. Unable to retrieve a session."))
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
        return Err(err_msg("Unable to find a valid session header."))
    }

    /// Fetch list of sites within a repo:
    ///   NOTE: Exclude 'mgnl:resources' from magnolia RESTful json responses as they include binary data we do NOT require.
    ///   curl -s -H 'Accept: application/json' '<url>/.rest/nodes/v1/<repo>?depth=1&excludeNodeTypes=mgnl:resource'
    pub fn sites(&self, repo_type: RepoType) -> Result<Option<Paths>, Error> {
        let mut cookie_session = Cookie::new();
        cookie_session.append("JSESSIONID", self.session.as_ref().unwrap().to_string());
        let url = format!("{}/.rest/nodes/v1/{}?depth=1&excludeNodeTypes=mgnl:resource", self.url, repo_type);
        let resp = self.client.get(&url)
            .header(cookie_session)
            .header(Accept(vec![qitem(mime::APPLICATION_JSON)]))
            .send()?;
        if resp.status().is_success() {
            Ok(nodes::build_paths(resp, repo_type, true)?)
        } else {
            Err(err_msg("Unable to retrieve list of sites for repo"))
        }
    }

    /// Fetch list of node paths for all sites within a repo and include the associated mgnl:lastModified properties found in nodes Metadata:
    ///   NOTE: Exclude 'mgnl:resources' from magnolia RESTful json responses as they include binary data we do NOT require.
    /// For all paths within repo while NOT including mgnl:folders
    ///   curl -s -H 'Accept: application/json' '<url>/.rest/nodes/v1/<repo>/<site>?depth=999&excludeNodeTypes=mgnl:resource&includeMetadata=true'
    pub fn paths(&self, path_info: &PathInfo) -> Result<Option<Paths>, Error> {
        let mut cookie_session = Cookie::new();
        cookie_session.append("JSESSIONID", self.session.as_ref().unwrap().to_string());
        let url = format!("{}/.rest/nodes/v1/{}{}?depth=999&excludeNodeTypes=mgnl:resource&includeMetadata=true", self.url, path_info.repo_type, path_info.path);
        println!("  check: {}", url);
        let resp = self.client.get(&url)
            .header(cookie_session)
            .header(Accept(vec![qitem(mime::APPLICATION_JSON)]))
            .send()?;
        if resp.status().is_success() {
            Ok(nodes::build_paths(resp, path_info.repo_type, false)?)
        } else {
            Err(err_msg("Unable to retrieve list of sites for repo"))
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
    pub fn export(&self, path_info: &PathInfo) -> Result<(), Error> {
        let mut cookie_session = Cookie::new();
        cookie_session.append("JSESSIONID", self.session.as_ref().unwrap().to_string());
        let url = format!("{}/docroot/gato/export.jsp", &self.url);
        let mut resp = self.client.get(&url)
            .header(cookie_session)
            .header(Accept(vec![qitem(mime::TEXT_XML)]))
            .header(Referer::new(url))
            .query(&[
                ("repo", path_info.repo_type.to_string()),
                ("path", path_info.path.clone()),
            ])
            .send()?;
        if resp.status().is_success() {
            let mut stdout = io::stdout();
            match io::copy(&mut resp, &mut stdout) {
                Ok(_) => Ok(()),
                Err(_) => Err(err_msg("")),
            }
        } else {
            Err(err_msg(""))
        }
    }
}
