use std::str::FromStr;
use std::fmt::{self, Display};
use serde_json::{self, Value};
use failure::{Error, err_msg};

pub const FOLDER_NODE_TYPE: &'static str = "mgnl:folder";

macro_rules! lowercase_enum {
    (name $name: ident,
     $($str_name: expr => ($variant: ident, $str_node_type: expr),)+
    ) => {
        #[derive(Debug, Copy, Clone, PartialEq)]
        pub enum $name {
            $($variant),+
        }

        impl FromStr for $name {
            type Err = Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($str_name => Ok($name::$variant),)+
                    _ => Err(err_msg("Invalid value")),
                }
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                match self {
                    $(&$name::$variant => write!(f, "{}", $str_name),)+
                }
            }
        }

        pub trait NodeType {
            fn node_type(&self) -> &str;
        }

        impl NodeType for $name {
            fn node_type(&self) -> &str {
                match self {
                    $(&$name::$variant => $str_node_type,)+
                }
            }
        }
    };
}

lowercase_enum!{
    name RepoType,
    "dam" => (Dam, "mgnl:asset"), //"mgnl:folder"
    "website" => (Website, "mgnl:page"),
    "config" => (Config, "mgnl:content"), //NOTE: mgnl:contentNode is used parts of a page that need to be activated with the parent
    "gatoapps" => (Gatoapps, "mgnl:content"),
    "resources" => (Resources, "mgnl:content"), //"mgnl:folder"
    "usergroups" => (Usergroups, "mgnl:group"),
    "userroles" => (Userroles, "mgnl:role"),
    "users" => (Users, "mgnl:user"), //"mgnl:folder"
}

pub type Site = String;

pub type Sites = Vec<Site>;

#[derive(Debug, PartialEq)]
pub struct Repo {
    repo_type: RepoType,
    sites: Option<Sites>,
}

pub type Repos = Vec<Repo>;

pub fn new(data: &str) -> Result<Repos, Error> {
    let mut repos: Repos = Vec::new();
    let json = serde_json::from_str(data)?;
    if let Value::Array(repo_list) = json {
        for repo_json in repo_list {
            match repo_json {
                Value::String(r) => repos.push(Repo{ repo_type: r.parse()?, sites: None }),
                Value::Object(o) => {
                    for (repo, ss) in o {
                        let repo: RepoType = repo.parse()?;
                        let mut sites: Sites = Vec::new();
                        if let Value::Array(ss) = ss {
                            for site in ss {
                                if let Value::String(site) = site {
                                     sites.push(site);
                                } else {
                                    return Err(err_msg("Malformed repo site entry"));
                                }
                            }
                        } else {
                            return Err(err_msg("Malformed repo site entry"));
                        }
                        repos.push(Repo{ repo_type: repo, sites: Some(sites) });
                    }
                },
                _ => return Err(err_msg("Invalid repo list entry type")),
            }
        }
        Ok(repos)
    } else {
        Err(err_msg("Invalid repo list"))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enum_to_string() {
        assert_eq!(RepoType::Website.to_string(), "website");
    }

    #[test]
    fn test_string_to_enum() {
        assert_eq!("website".parse::<RepoType>().unwrap(), RepoType::Website);
    }

    #[test]
    fn test_enum_to_node_type() {
        assert_eq!(RepoType::Website.node_type(), "mgnl:page");
    }

    #[test]
    fn test_string_to_repo() {
        let json = r#"["dam","website"]"#;
        let repos: Repos = new(json).unwrap();
        assert_eq!(repos, vec![
            Repo{ repo_type: RepoType::Dam, sites: None },
            Repo{ repo_type: RepoType::Website, sites: None },
        ]);
    }

    #[test]
    fn test_object_to_repo() {
        let json = r#"[{"dam": ["dam1","dam2"]}, {"website": ["website1"]}]"#;
        let repos: Repos = new(json).unwrap();
        assert_eq!(repos, vec![
            Repo{ repo_type: RepoType::Dam, sites: Some(vec!["dam1".to_string(), "dam2".to_string()]) },
            Repo{ repo_type: RepoType::Website, sites: Some(vec!["website1".to_string()]) },
        ]);
    }

    #[test]
    fn test_object_and_string_to_repo() {
        let json = r#"[{"dam": ["dam1","dam2"]}, "website"]"#;
        let repos: Repos = new(json).unwrap();
        assert_eq!(repos, vec![
            Repo{ repo_type: RepoType::Dam, sites: Some(vec!["dam1".to_string(), "dam2".to_string()]) },
            Repo{ repo_type: RepoType::Website, sites: None },
        ]);
    }

}
