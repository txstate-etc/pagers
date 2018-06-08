use nodes::PathInfo;

// Turn PathInfo into /<repo>/<site>-<archive_extension>/<repo>.<site.path>.xml
pub fn name(site_info: &PathInfo, path_info: &PathInfo, archive_extension: &str) -> String {
    format!("/{}{}-{}/{}{}.xml", site_info.repo_type, site_info.path, archive_extension, path_info.repo_type, path_info.path.replace("/", "."))
}
