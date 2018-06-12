use nodes::PathInfo;
use percent_encoding::{utf8_percent_encode, PATH_SEGMENT_ENCODE_SET};

// get site from path
fn site(path: &str) -> &str {
    path.splitn(3, "/").skip(1).next().unwrap_or("")
}

// Turn PathInfo into /<repo>/<site>-<archive_extension>/percent_encoded(<repo>/<site/path>).xml
pub fn name(path: &PathInfo, archive_ext: &str, _previous_ext: &str) -> String {
    format!("/{}/{}-{}/{}.xml",
        path.repo_type,
        site(&path.path),
        archive_ext,
        utf8_percent_encode(&format!("{}{}", path.repo_type, &path.path), PATH_SEGMENT_ENCODE_SET))
}


#[cfg(test)]
mod tests {
    use super::*;
    use repos::RepoType;
    use chrono::{DateTime, Local};

    #[test]
    fn test_name() {
        let path = PathInfo{
            path: "/gato/subpage1/subpage2/file name.odf".to_string(),
            repo_type: RepoType::Website,
            last_modified: Some("2018-05-05T08:59:29.261-05:00".parse::<DateTime<Local>>().unwrap()),
        };
        assert_eq!(name(&path, "20180506", "20180505"), "/website/gato-20180506/website%2Fgato%2Fsubpage1%2Fsubpage2%2Ffile%20name.odf.xml");
    }
}
