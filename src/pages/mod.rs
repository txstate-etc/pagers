use std::io::Read;
use serde_json;
use serde_json::Value;
use failure::Error; //, err_msg};
use chrono::{DateTime, Local};

#[derive(Debug, PartialEq)]
pub struct Page {
    path: String,
    last_modified: Option<DateTime<Local>>,
}

pub type Pages = Vec<Page>;

pub fn new<R: Read>(data: R) -> Result<Pages, Error> {
    let node = Node::new(data)?;
    Ok(node.pages())
}

// curl --user '<usr:pwd>' -H 'Accept: application/json' '<url>/.rest/nodes/v1/website/<site>?depth=1&excludeNodeTypes=mgnl:resource,mgnl:content,mgnl:contentNode,mgnl:area,mgnl:component,mgnl:user,mgnl:group,mgnl:role,mgnl:folder,mgnl:metaData,mgnl:nodeData,mgnl:reserve&includeMetadata=true' | python -m json.tool

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
        //let node: Node = serde_json::from_reader(data).unwrap()); //or (Err(err_msg("Please implement")))?;
        Ok(serde_json::from_reader(data)?)
    }

    fn info(&self) -> Page {
        for property in &self.properties {
            if property["name"] == "mgnl:lastModified" {
                if let Value::Array(ref last_modifieds) = property["values"] {
                    match last_modifieds.last() {
                        Some(&Value::String(ref last_modified)) => {
                            // magnolia times are only in milliseconds so convert to nanoseconds
                            if let Ok(last_modified) = last_modified.parse::<DateTime<Local>>() {
                                return Page{ path: self.path.clone(), last_modified: Some(last_modified) };
                            }
                            ()
                        },
                        _ => (),
                    }
                }
            }
        }
        Page{ path: self.path.clone(), last_modified: None }
    }

    fn pages(&self) -> Pages {
        let mut pages = vec![self.info()];
        if let Some(ref nodes) = self.nodes {
            for node in nodes {
                pages.extend(node.pages());
            }
        }
        pages
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_deserialize() {
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
        let pages = new(data).unwrap();
        assert_eq!(pages, vec![
            Page{ path: "/gato".to_string(), last_modified: Some("2018-05-05T08:59:29.261-05:00".parse::<DateTime<Local>>().unwrap()) },
            Page{ path: "/gato/las-communications".to_string(), last_modified: Some("2018-02-20T17:30:14.383-06:00".parse::<DateTime<Local>>().unwrap()) },
        ]);
    }
}
