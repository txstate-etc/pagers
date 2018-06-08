use nodes::PathInfo;

// Turn PathInfo into /<repo>/<site>-<archive_extension>/<repo>.<site.path>.xml
pub fn name(site_info: &PathInfo, path_info: &PathInfo, archive_extension: &str) -> String {
    format!("/{}{}-{}/{}{}.xml", site_info.repo_type, site_info.path, archive_extension, path_info.repo_type, path_info.path.replace("/", "."))
}


#[cfg(test)]
mod tests {
    use super::*;
    use repos::RepoType;
    use chrono::{DateTime, Local};

    #[test]
    fn test_name() {
        let site = PathInfo{
            path: "/gato".to_string(),
            repo_type: RepoType::Website,
            last_modified: None,
        };
        let path = PathInfo{
            path: "/gato/subpage1/subpage2/file".to_string(),
            repo_type: RepoType::Website,
            last_modified: Some("2018-05-05T08:59:29.261-05:00".parse::<DateTime<Local>>().unwrap()),
        };
        assert_eq!(name(&site, &path, "20180506"), "/website/gato-20180506/website.gato.subpage1.subpage2.file.xml");
    }
}
