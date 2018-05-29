use std::str::FromStr;
use std::fmt::{self, Display};
use failure::{Error, err_msg};

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

        trait NodeType {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_string() {
        assert_eq!(RepoType::Website.to_string(), "website");
    }

    #[test]
    fn test_to_enum() {
        assert_eq!("website".parse::<RepoType>().unwrap(), RepoType::Website);
    }

    #[test]
    fn test_node_type() {
        assert_eq!(RepoType::Website.node_type(), "mgnl:page");
    }
}
