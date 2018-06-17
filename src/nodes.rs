use std::io::Read;
use serde_json;
use serde_json::Value;
use failure::Error;
use chrono::{DateTime, Local};
use repos::{RepoType, NodeType, FOLDER_NODE_TYPE};

/// PathInfo structure holds the associated repo type,
/// the absolute path of the node with regards to the repo,
/// and the last time the node was modified.
#[derive(Debug, PartialEq)]
pub struct PathInfo {
    pub repo_type: RepoType,
    pub path: String,
    pub last_modified: Option<DateTime<Local>>,
}

/// Information list of Nodes
pub type Paths = Vec<PathInfo>;


/// Create Information list of node types from data stream.
/// Depending on the data fed into this function, this is used to find the
/// sites that exist for a repo, or used to find the leaf nodes within a site.
pub fn build_paths<R: Read>(data: R, repo_type: RepoType, folders: bool) -> Result<Option<Paths>, Error> {
    let node = Node::new(data)?;
    Ok(node.flat_paths(repo_type, folders))
}

/// Reduce the number of nodes by triming the tree down to the level specified.
/// Level 0 is the root node, and level 1 would be the nodes just off the root
/// node, and so on. The remaining nodes will be assigned the max last_modified
/// timestamp of this parent and its associated children.
pub fn reduce_paths<R: Read>(data: R, repo_type: RepoType, level: usize) -> Result<Option<Paths>, Error> {
    let node = Node::new(data)?;
    Ok(node.reduce_paths(repo_type, level))
}

// Properties of Nodes. The only object we care about at the moment is the lastModified property.
// ISSUE: Was not able to ignore other Property types. As this list of Property types is unfixed,
// we cannot define all the variants, nor would we want to.
//#[derive(Deserialize, Debug)]
//#[serde(tag = "name")]
//pub struct Properties {
//    #[serde(rename="mgnl:lastModified")]
//    LastModified {
//        values: Vec<String>
//    },
//}

// If we need to request more than just page type nodes then we will need to turn this into an enum.
#[derive(Deserialize, Debug)]
struct Node {
    path: String,
    properties: Vec<Value>,
    nodes: Option<Vec<Node>>,
    #[serde(rename="type")]
    node_type: String,
}

impl Node {
    fn new<R: Read>(data: R) -> Result<Node, Error> {
        Ok(serde_json::from_reader(data)?)
    }

    // consumes Node to generate PathInfo
    fn path_info(&self, repo_type: RepoType, folders: bool) -> Option<PathInfo> {
        if &self.node_type == repo_type.node_type() || (folders && &self.node_type == FOLDER_NODE_TYPE) {
            for property in &self.properties {
                if property["name"] == "mgnl:lastModified" {
                    if let Value::Array(ref last_modifieds) = property["values"] {
                        match last_modifieds.last() {
                            Some(&Value::String(ref last_modified)) => {
                                if let Ok(last_modified) = last_modified.parse::<DateTime<Local>>() {
                                    return Some(PathInfo{ repo_type: repo_type, path: self.path.to_owned(), last_modified: Some(last_modified) });
                                }
                                ()
                            },
                            _ => (),
                        }
                    }
                }
            }
            return Some(PathInfo{ repo_type: repo_type, path: self.path.clone(), last_modified: None })
        }
        None
    }

    fn flat_paths(&self, repo_type: RepoType, folders: bool) -> Option<Paths> {
        if self.path.ends_with("]") {
            return None;
        }
        let mut paths = Vec::new();
        if let Some(path_info) = self.path_info(repo_type, folders) {
            paths.push(path_info);
        }
        if let Some(ref nodes) = self.nodes {
            for node in nodes {
                if let Some(sub_nodes) = node.flat_paths(repo_type, folders) {
                    paths.extend(sub_nodes);
                }
            }
        }
        if paths.len() > 0 {
            Some(paths)
        } else {
            None
        }
    }

    fn max_last_modified(&self, repo_type: RepoType) -> Option<DateTime<Local>> {
        if self.path.ends_with("]") {
            return None;
        }
        let mut last_modified = None;
        if let Some(path_info) = self.path_info(repo_type, true) {
            last_modified = path_info.last_modified;
        }
        if let Some(ref nodes) = self.nodes {
            for node in nodes {
                if let Some(sub_last_modified) = node.max_last_modified(repo_type) {
                    if let Some(cur_last_modified) = last_modified {
                        if cur_last_modified < sub_last_modified {
                            last_modified = Some(sub_last_modified);
                        }
                    } else {
                        last_modified = Some(sub_last_modified);
                    }
                }
            }
        }
        last_modified
    }

    fn reduce_paths(&self, repo_type: RepoType, mut level: usize) -> Option<Paths> {
        if self.path.ends_with("]") {
            return None;
        }
        let mut paths = Vec::new();
        // if node has no children (no matter level)
        //   1) Append this node to paths
        //   2) Return
        if self.nodes.is_none() {
            if let Some(path_info) = self.path_info(repo_type, true) {
                paths.push(path_info);
                return Some(paths);
            }
            return None;
        }

        // if level > 0 and node has children
        //   1) Do NOT append current node to paths
        //   2) Append results of reduce_paths called on each child w/ --level
        //   3) return
        if level > 0 {
            level -= 1;
            if let Some(ref nodes) = self.nodes {
                for node in nodes {
                    if let Some(sub_nodes) = node.reduce_paths(repo_type, level) {
                        paths.extend(sub_nodes);
                    }
                }
            }
            if paths.len() > 0 {
                return Some(paths);
            } else {
                return None;
            }
        }

        // if level 0 and node has children
        //   1) call max_last_modified on self and thus children
        //   2) Append this node's path_info with results of max_last_modified
        if let Some(mut path_info) = self.path_info(repo_type, true) {
            path_info.last_modified = self.max_last_modified(repo_type);
            paths.push(path_info);
            return Some(paths);
        } else {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // curl -s -H 'Accept: application/json' '<url>/.rest/nodes/v1/website/<site>?depth=999&excludeNodeTypes=mgnl:resource&includeMetadata=true' | python -m json.tool
    #[test]
    fn test_page_nodes_for_tree_structure_of_website_repo() {
        let data = r#"{
        "identifier": "8697faaa-00bc-4c43-94fa-1a9fe2e10a49", 
        "name": "gato", 
        "nodes": [
            {
                "identifier": "584e2528-9070-433b-9cea-af9f0b4d8755", 
                "name": "las-communications", 
                "nodes": null, 
                "path": "/gato/las-communications", 
                "properties": [
                    {
                        "multiple": false, 
                        "name": "mgnl:lastModified", 
                        "type": "Date", 
                        "values": [
                            "2018-02-20T17:30:14.383-06:00"
                        ]
                    }
                ], 
                "type": "mgnl:page"
            }
        ],
        "path": "/gato", 
        "properties": [
             {
                 "multiple": false, 
                 "name": "jcr:uuid", 
                 "type": "String", 
                 "values": [
                     "8697faaa-00bc-4c43-94fa-1a9fe2e10a49"
                 ]
             }, 
             {
                "multiple": true, 
                "name": "jcr:mixinTypes", 
                "type": "Name", 
                "values": [
                    "mix:lockable", 
                    "mgnl:hasVersion"
                ]
            },
            {
                "multiple": false, 
                "name": "mgnl:lastModified", 
                "type": "Date", 
                "values": [
                    "2018-05-05T08:59:29.261-05:00"
                ]
            }
        ],
        "type": "mgnl:page"
        }"#.as_bytes();
        let paths = build_paths(data, RepoType::Website, false).unwrap();
        assert_eq!(paths, Some(
            vec![
                PathInfo{ repo_type: RepoType::Website, path: "/gato".to_string(), last_modified: Some("2018-05-05T08:59:29.261-05:00".parse::<DateTime<Local>>().unwrap()) },
                PathInfo{ repo_type: RepoType::Website, path: "/gato/las-communications".to_string(), last_modified: Some("2018-02-20T17:30:14.383-06:00".parse::<DateTime<Local>>().unwrap()) },
       ]));
    }

    // curl -s -H 'Accept: application/json' '<url>/.rest/nodes/v1/dam/<site>?depth=999&excludeNodeTypes=mgnl:resource&includeMetadata=true' | python -m json.tool
    // We do NOT want folders, only leaf nodes types "mgnl:assets"
    // We also do not want ambiguous paths such as /gato[2] as 
    // such duplicate sites are not visible in magnolia, yet are
    // allowed in JCR. Filter duplicates and associated sub nodes.
    #[test]
    fn test_asset_nodes_for_leaf_nodes_in_dam_repo() {
        let data = r#"{
            "identifier": "7c31a9de-1cb5-41ce-940e-f6716d6cf7ca",
            "name": "gato",
            "nodes": [
                {
                    "identifier": "ed9f2988-93c2-455d-b35b-1a188a006031",
                    "name": "subpage",
                    "nodes": [
                        {
                            "identifier": "079ef347-3808-4d95-806b-a195fde75e2e",
                            "name": "basilisk.gif",
                            "nodes": null,
                            "path": "/gato/subpage/basilisk.gif",
                            "properties": [
                                {
                                    "multiple": false,
                                    "name": "mgnl:lastModified",
                                    "type": "Date",
                                    "values": [
                                        "2016-06-30T12:17:18.324-05:00"
                                    ]
                                }
                            ],
                            "type": "mgnl:asset"
                        }
                    ],
                    "path": "/gato/subpage",
                    "properties": [
                        {
                            "multiple": false,
                            "name": "mgnl:lastModified",
                            "type": "Date",
                            "values": [
                                "2016-06-28T12:17:20.486-05:00"
                            ]
                        }
                    ],
                    "type": "mgnl:folder"
                },
                {
                    "identifier": "ed9f2988-93c2-455d-b35b-1a188a006031",
                    "name": "subpage",
                    "nodes": [
                        {
                            "identifier": "079ef347-3808-4d95-806b-a195fde75e2e",
                            "name": "basilisk.gif",
                            "nodes": null,
                            "path": "/gato/subpage[2]/basilisk.gif",
                            "properties": [
                                {
                                    "multiple": false,
                                    "name": "mgnl:lastModified",
                                    "type": "Date",
                                    "values": [
                                        "2016-06-30T12:17:18.324-05:00"
                                    ]
                                }
                            ],
                            "type": "mgnl:asset"
                        }
                    ],
                    "path": "/gato/subpage[2]",
                    "properties": [
                        {
                            "multiple": false,
                            "name": "mgnl:lastModified",
                            "type": "Date",
                            "values": [
                                "2016-06-28T12:17:20.486-05:00"
                            ]
                        }
                    ],
                    "type": "mgnl:folder"
                },
                {
                    "identifier": "29355f9c-82cb-4397-9cea-bbd7fb96eea7",
                    "name": "rssfeed.png",
                    "nodes": null,
                    "path": "/gato/rssfeed.png",
                    "properties": [
                        {
                            "multiple": false,
                            "name": "mgnl:lastModified",
                            "type": "Date",
                            "values": [
                                "2018-05-18T09:53:36.380-05:00"
                            ]
                        }
                    ],
                    "type": "mgnl:asset"
                }
            ],
            "path": "/gato",
            "properties": [
                {
                    "multiple": false,
                    "name": "mgnl:lastModified",
                    "type": "Date",
                    "values": [
                        "2018-05-18T09:53:36.314-05:00"
                    ]
                }
            ],
            "type": "mgnl:folder"
        }"#.as_bytes();
        let paths = build_paths(data, RepoType::Dam, false).unwrap();
        assert_eq!(paths, Some(
            vec![
                PathInfo{ repo_type: RepoType::Dam, path: "/gato/subpage/basilisk.gif".to_string(), last_modified: Some("2016-06-30T12:17:18.324-05:00".parse::<DateTime<Local>>().unwrap()) },
                PathInfo{ repo_type: RepoType::Dam, path: "/gato/rssfeed.png".to_string(), last_modified: Some("2018-05-18T09:53:36.380-05:00".parse::<DateTime<Local>>().unwrap()) },
        ]));
    }

    #[test]
    fn test_asset_nodes_for_empty_site_in_dam_repo() {
        let data = r#"{
            "identifier": "7c31a9de-1cb5-41ce-940e-f6716d6cf7ca",
            "name": "gato",
            "nodes": null,
            "path": "/gato",
            "properties": [
                {
                    "multiple": false,
                    "name": "mgnl:lastModified",
                    "type": "Date",
                    "values": [
                        "2018-05-18T09:53:36.314-05:00"
                    ]
                }
            ],
            "type": "mgnl:folder"
        }"#.as_bytes();
        let nodes = build_paths(data, RepoType::Dam, false).unwrap();
        assert_eq!(nodes, None)
    }

    // Given data from repo with depth of 1, return back a list of sites under that repo
    // curl -s -H 'Accept: application/json' '<url>/.rest/nodes/v1/<repo>/?depth=1&excludeNodeTypes=mgnl:resource' | python -m json.tool
    #[test]
    fn test_folder_nodes_for_sites_in_dam_repo() {
        let data = r#"{
            "identifier": "cafebabe-cafe-babe-cafe-babecafebabe",
            "name": "",
            "nodes": [
                {
                    "identifier": "deadbeef-cafe-babe-cafe-babecafebabe",
                    "name": "jcr:system",
                    "nodes": null,
                    "path": "/jcr:system",
                    "properties": [],
                    "type": "rep:system"
                },
                {
                    "identifier": "7c31a9de-1cb5-41ce-940e-f6716d6cf7ca",
                    "name": "gato",
                    "nodes": null,
                    "path": "/gato",
                    "properties": [
                        {
                            "multiple": false,
                            "name": "title",
                            "type": "String",
                            "values": [
                                "gato"
                            ]
                        }
                    ],
                    "type": "mgnl:folder"
                },
                {
                    "identifier": "7c31a9de-1cb5-41ce-940e-f6716d6cf7ca",
                    "name": "gato",
                    "nodes": null,
                    "path": "/gato[2]",
                    "properties": [
                        {
                            "multiple": false,
                            "name": "title",
                            "type": "String",
                            "values": [
                                "gato"
                            ]
                        }
                    ],
                    "type": "mgnl:folder"
                },
                {
                    "identifier": "9c5a2747-c439-4c1c-bc0a-ac04f171c1d6",
                    "name": "Asset.zip",
                    "nodes": null,
                    "path": "/Asset.zip",
                    "properties": [
                        {
                            "multiple": false,
                            "name": "gato_activated_on_creation",
                            "type": "Boolean",
                            "values": [
                                "true"
                            ]
                        },
                        {
                            "multiple": false,
                            "name": "name",
                            "type": "String",
                            "values": [
                                "Asset Zip File"
                            ]
                        }
                    ],
                    "type": "mgnl:asset"
                }
            ],
            "path": "/",
            "properties": [],
            "type": "rep:root"
        }"#.as_bytes();
        let paths = build_paths(data, RepoType::Dam, true).unwrap();
        assert_eq!(paths, Some(
            vec![
                PathInfo{ repo_type: RepoType::Dam, path: "/gato".to_string(), last_modified: None },
                PathInfo{ repo_type: RepoType::Dam, path: "/Asset.zip".to_string(), last_modified: None },
        ]));
    }

    // Given data from repo with depth of 999, return back a list of sites under that repo and
    // their max last modified time stamps
    // curl -s -H 'Accept: application/json' '<url>/.rest/nodes/v1/<repo>/?depth=999&excludeNodeTypes=mgnl:resource&includeMetadata=true' | python -m json.tool
    #[test]
    fn test_reduce_pages_for_user_repo() {
        let data = r#"{
            "identifier": "cafebabe-cafe-babe-cafe-babecafebabe",
            "name": "",
            "nodes": [
                {
                    "identifier": "deadbeef-cafe-babe-cafe-babecafebabe",
                    "name": "jcr:system",
                    "nodes": null,
                    "path": "/jcr:system",
                    "properties": [],
                    "type": "rep:system"
                },
                {
                    "identifier": "7c31a9de-1cb5-41ce-940e-f6716d6cf7ca",
                    "name": "gato",
                    "nodes": null,
                    "path": "/gato",
                    "properties": [
                        {
                            "multiple": false,
                            "name": "title",
                            "type": "String",
                            "values": [
                                "gato"
                            ]
                        }
                    ],
                    "type": "mgnl:folder"
                },
                {
                    "identifier": "7c31a9de-1cb5-41ce-940e-f6716d6cf7ca",
                    "name": "gato",
                    "nodes": null,
                    "path": "/gato[2]",
                    "properties": [
                        {
                            "multiple": false,
                            "name": "title",
                            "type": "String",
                            "values": [
                                "gato"
                            ]
                        }
                    ],
                    "type": "mgnl:folder"
                },
                {
                    "identifier": "9c5a2747-c439-4c1c-bc0a-ac04f171c1d6",
                    "name": "Asset.zip",
                    "nodes": null,
                    "path": "/Asset.zip",
                    "properties": [
                        {
                            "multiple": false,
                            "name": "gato_activated_on_creation",
                            "type": "Boolean",
                            "values": [
                                "true"
                            ]
                        },
                        {
                            "multiple": false,
                            "name": "name",
                            "type": "String",
                            "values": [
                                "Asset Zip File"
                            ]
                        }
                    ],
                    "type": "mgnl:asset"
                }
            ],
            "path": "/",
            "properties": [],
            "type": "rep:root"
        }"#.as_bytes();
        let paths = reduce_paths(data, RepoType::Dam, 1).unwrap();
        assert_eq!(paths, Some(
            vec![
                PathInfo{ repo_type: RepoType::Dam, path: "/gato".to_string(), last_modified: None },
                PathInfo{ repo_type: RepoType::Dam, path: "/Asset.zip".to_string(), last_modified: None },
        ]));
    }
}
