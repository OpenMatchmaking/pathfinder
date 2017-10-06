extern crate yaml_rust;

use std::fs::File;
use std::io::Read;

use error::{Result};
use self::yaml_rust::{Yaml, YamlLoader};


fn read_file(file_path: &str) -> Result<String> {
    let mut file = try!(File::open(file_path));
    let mut data = String::new();
    try!(file.read_to_string(&mut data));
    Ok(data)
}


pub fn load_config(file_path: &str) -> Yaml {
    let data = match file_path {
        "" => "---".to_string(),
        _ => match read_file(file_path) {
            Ok(content) => content,
            Err(why) => {
                println!("{}", why);
                "---".to_string()
            }
        }
    };

    let docs = YamlLoader::load_from_str(data.as_str()).unwrap();
    docs[0].clone()
}


#[cfg(test)]
mod tests {
    use super::{load_config, read_file};
    use super::Yaml;

    #[test]
    fn test_read_file_returns_an_error_for_invalid_filepath() {
        let result = read_file(&"invalid_path.yaml");
        assert_eq!(result.is_err(), true);
        assert_eq!(
            format!("{}", result.unwrap_err()),
            "IO error: No such file or directory (os error 2)"
        );
    }

    #[test]
    fn test_read_file_returns_yaml_data() {
        let result = read_file(&"./tests/files/valid_file.yaml");
        assert_eq!(result.is_ok(), true);
        assert_eq!(result.unwrap(), "foo:\n  - bar\n");
    }

    #[test]
    fn test_load_config_returns_an_empty_yaml_by_default() {
        let doc = load_config(&"");
        assert_eq!(doc, Yaml::Null);
    }

    #[test]
    fn test_load_config_returns_an_valid_yaml_document() {
        let doc = load_config(&"./tests/files/valid_file.yaml");
        assert_eq!(doc["foo"][0].as_str().unwrap(), "bar");
    }
}