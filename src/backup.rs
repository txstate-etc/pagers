use nodes::PathInfo;
use percent_encoding::{utf8_percent_encode, PATH_SEGMENT_ENCODE_SET};

// Turn PathInfo into /<repo>/<site>-<archive_extension>/percent_encoded(<repo>/<site/path>).xml
pub fn name(site: &PathInfo, path: &PathInfo, archive_extension: &str) -> String {
    format!("/{}{}-{}/{}.xml",
        site.repo_type,
        site.path,
        archive_extension,
        utf8_percent_encode(&format!("{}{}", path.repo_type, path.path), PATH_SEGMENT_ENCODE_SET))
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
            path: "/gato/subpage1/subpage2/file name.odf".to_string(),
            repo_type: RepoType::Website,
            last_modified: Some("2018-05-05T08:59:29.261-05:00".parse::<DateTime<Local>>().unwrap()),
        };
        assert_eq!(name(&site, &path, "20180506"), "/website/gato-20180506/website%2Fgato%2Fsubpage1%2Fsubpage2%2Ffile%20name.odf.xml");
    }
}
