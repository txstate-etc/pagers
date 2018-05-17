extern crate serde;
#[macro_use] extern crate serde_derive;
//#[macro_use]
extern crate serde_json;
extern crate failure;
extern crate chrono;

mod pages;
//use serde_json;

fn main() {
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
    let ps = pages::new(data);
    println!("{:?}", ps);
}
