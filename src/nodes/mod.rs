use std::io::Read;
use serde_json;
use serde_json::Value;
use failure::Error;
use chrono::{DateTime, Local};

#[derive(Debug, PartialEq)]
pub struct Info {
    path: String,
    last_modified: Option<DateTime<Local>>,
}

/// Information list of Nodes
pub type Nodes = Vec<Info>;


/// Create Information list of node types from data stream
pub fn new<R: Read>(data: R, node_type: &str) -> Result<Option<Nodes>, Error> {
    let node = Node::new(data)?;
    Ok(node.nodes(node_type))
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

    fn info(&self, node_type: &str) -> Option<Info> {
        if &self.node_type == node_type {
            for property in &self.properties {
                if property["name"] == "mgnl:lastModified" {
                    if let Value::Array(ref last_modifieds) = property["values"] {
                        match last_modifieds.last() {
                            Some(&Value::String(ref last_modified)) => {
                                if let Ok(last_modified) = last_modified.parse::<DateTime<Local>>() {
                                    return Some(Info{ path: self.path.clone(), last_modified: Some(last_modified) });
                                }
                                ()
                            },
                            _ => (),
                        }
                    }
                }
            }
            return Some(Info{ path: self.path.clone(), last_modified: None })
        }
        None
    }

    fn nodes(&self, node_type: &str) -> Option<Nodes> {
        let mut infos = Vec::new();
        if let Some(info) = self.info(node_type) {
            infos.push(info);
        }
        if let Some(ref nodes) = self.nodes {
            for node in nodes {
                if let Some(sub_nodes) = node.nodes(node_type) {
                    infos.extend(sub_nodes);
                }
            }
        }
        if infos.len() > 0 {
            Some(infos)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // curl --user '<usr:pwd>' -H 'Accept: application/json' '<url>/.rest/nodes/v1/website/<site>?depth=999&excludeNodeTypes=mgnl:resource,mgnl:content,mgnl:contentNode,mgnl:area,mgnl:component,mgnl:user,mgnl:group,mgnl:role,mgnl:folder,mgnl:metaData,mgnl:nodeData,mgnl:reserve&includeMetadata=true' | python -m json.tool
    #[test]
    fn test_nodes_type_page() {
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
        let nodes = new(data, "mgnl:page").unwrap();
        assert_eq!(nodes, Some(
            vec![
                Info{ path: "/gato".to_string(), last_modified: Some("2018-05-05T08:59:29.261-05:00".parse::<DateTime<Local>>().unwrap()) },
                Info{ path: "/gato/las-communications".to_string(), last_modified: Some("2018-02-20T17:30:14.383-06:00".parse::<DateTime<Local>>().unwrap()) },
       ]));
    }

    // curl -s --user '<usr>:<pwd>' -H 'Accept: application/json' '<url>/.rest/nodes/v1/dam/<site>?depth=999&excludeNodeTypes=mgnl:resource&includeMetadata=true' | python -m json.tool
    // NOTE: we do NOT want folders, only leaf nodes types "mgnl:assets"
    #[test]
    fn test_nodes_type_asset() {
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
        let nodes = new(data, "mgnl:asset").unwrap();
        assert_eq!(nodes, Some(
            vec![
                Info{ path: "/gato/subpage/basilisk.gif".to_string(), last_modified: Some("2016-06-30T12:17:18.324-05:00".parse::<DateTime<Local>>().unwrap()) },
                Info{ path: "/gato/rssfeed.png".to_string(), last_modified: Some("2018-05-18T09:53:36.380-05:00".parse::<DateTime<Local>>().unwrap()) },
        ]));
    }

    #[test]
    fn test_nodes_type_asset_empty() {
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
        let nodes = new(data, "mgnl:asset").unwrap();
        assert_eq!(nodes, None)
    }
}
