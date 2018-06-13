use nodes::PathInfo;
use percent_encoding::{utf8_percent_encode, PATH_SEGMENT_ENCODE_SET};

/// get site from path
pub fn extract_site(path: &str) -> &str {
    path.splitn(3, "/").skip(1).next().unwrap_or("")
}

/// Turn archive dir, extension, and PathInfo into /<repo>/<site>/<archive_extension>
pub fn archive_path(dir: &str, ext: &str, path: &PathInfo) -> String {
    format!("{}/{}/{}/{}", dir.to_string(), &path.repo_type, extract_site(&path.path), ext)
}

/// Turn PathInfo into percent_encoded(<repo>/<site/path>).xml
pub fn backup_filename(path: &PathInfo) -> String {
    utf8_percent_encode(&format!("{}{}.xml", path.repo_type, &path.path), PATH_SEGMENT_ENCODE_SET).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use repos::RepoType;
    use chrono::{DateTime, Local};

    #[test]
    fn test_archive_path() {
        let archive_dir = "/mnt/nfs/archive";
        let archive_ext = "20180506";
        let path = PathInfo{
            path: "/gato/subpage1/subpage2/file name.odf".to_string(),
            repo_type: RepoType::Website,
            last_modified: Some("2018-05-05T08:59:29.261-05:00".parse::<DateTime<Local>>().unwrap()),
        };
        assert_eq!(archive_path(archive_dir, &archive_ext, &path), "/mnt/nfs/archive/website/gato/20180506");
    }

    #[test]
    fn test_backup_filename() {
        let path = PathInfo{
            path: "/gato/subpage1/subpage2/file name.odf".to_string(),
            repo_type: RepoType::Website,
            last_modified: Some("2018-05-05T08:59:29.261-05:00".parse::<DateTime<Local>>().unwrap()),
        };
        assert_eq!(backup_filename(&path), "website%2Fgato%2Fsubpage1%2Fsubpage2%2Ffile%20name.odf.xml");
    }
}
