extern crate pest;
#[macro_use]
extern crate pest_derive;

use std::collections::HashMap;
use anyhow::Error;
use log::error;
use pest::Parser;

#[derive(Parser)]
#[grammar = "label_grammar.pest"]
struct LabelsParser;


/// Parse prometheus labels
/// "" -> []
/// "{}" -> []
/// "{a=\"b\"}" -> [("a", "b")]
/// "{a=\"b\", c=\"d\"}" -> [("a", "b"), ("c", "d")]
pub fn parse_labels(string: String) -> Result<Vec<(String, String)>, Error> {
    if string.len() == 0 {
        return Ok(vec![]);
    }
    if string == "{}".to_string() {
        return Ok(vec![]);
    }
    let labels_parser = LabelsParser::parse(Rule::labels, &string);
    match labels_parser {
        Ok(mut labels_parser) => {
            Ok(labels_parser
                .next()
                .unwrap()
                .into_inner()
                .map(|pair| {
                    let mut iter = pair.into_inner();
                    let key = iter.next().unwrap().as_str().to_string();
                    let value = iter.next().unwrap().as_str().to_string();
                    (key, value)
                })
                .collect())
        }
        Err(e) => {
            error!("{}", e);
            Err(e.into())
        }
    }
}

pub fn parse_labels_into_map(string: String) -> Result<HashMap<String, String>, Error> {
    let labels = parse_labels(string)?;
    Ok(labels.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_labels_empty_string() {
        let labels = parse_labels("".to_string()).unwrap();
        assert_eq!(labels, vec![]);
    }

    #[test]
    fn test_parse_labels_empty_object() {
        let labels = parse_labels("{}".to_string()).unwrap();
        assert_eq!(labels, vec![]);
    }

    #[test]
    fn test_parse_labels_one_label() {
        let labels = parse_labels("{foo=\"bar\"}".to_string()).unwrap();
        assert_eq!(labels, vec![("foo".to_string(), "bar".to_string())]);
    }

    #[test]
    fn test_parse_labels_two_labels() {
        let labels = parse_labels("{foo=\"bar\",baz=\"qux\"}".to_string()).unwrap();
        assert_eq!(labels, vec![("foo".to_string(), "bar".to_string()), ("baz".to_string(), "qux".to_string())]);
    }

    #[test]
    fn test_parse_labels_two_labels_with_spaces() {
        let labels = parse_labels("{foo = \"bar\", baz = \"qux\"}".to_string()).unwrap();
        assert_eq!(labels, vec![("foo".to_string(), "bar".to_string()), ("baz".to_string(), "qux".to_string())]);
    }

    #[test]
    fn test_parse_labels_throws_when_missing_quotes() {
        let labels = parse_labels("{foo=bar,baz=qux}".to_string()).unwrap_err();
        assert_eq!(labels.to_string(), " --> 1:2\n  |\n1 | {foo=bar,baz=qux}\n  |  ^---\n  |\n  = expected label".to_string());
    }

    #[test]
    fn test_parse_labels_throws_when_missing_key() {
        let labels = parse_labels("{=bar}".to_string()).unwrap_err();
        assert_eq!(labels.to_string(), " --> 1:2\n  |\n1 | {=bar}\n  |  ^---\n  |\n  = expected labelKey".to_string());
    }

    #[test]
    fn test_parse_labels_throws_when_missing_value() {
        let labels = parse_labels("{foo=}".to_string()).unwrap_err();
        assert_eq!(labels.to_string(), " --> 1:2\n  |\n1 | {foo=}\n  |  ^---\n  |\n  = expected label".to_string());
    }

    #[test]
    fn test_parse_labels_throws_when_missing_key_value_pair() {
        let labels = parse_labels("{,baz=qux}".to_string()).unwrap_err();
        assert_eq!(labels.to_string(), " --> 1:2\n  |\n1 | {,baz=qux}\n  |  ^---\n  |\n  = expected labelKey".to_string());
    }

    #[test]
    fn test_parse_labels_throws_when_missing_key_value_pair_with_spaces() {
        let labels = parse_labels("{ ,baz=qux}".to_string()).unwrap_err();
        assert_eq!(labels.to_string(), " --> 1:3\n  |\n1 | { ,baz=qux}\n  |   ^---\n  |\n  = expected labelKey".to_string());
    }

    #[test]
    fn test_parse_labels_into_map_one_label() {
        let labels = parse_labels_into_map("{foo=\"bar\"}".to_string()).unwrap();
        assert_eq!(labels, HashMap::from([("foo".to_string(), "bar".to_string())]));
    }
}