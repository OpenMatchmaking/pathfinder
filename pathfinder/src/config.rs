extern crate yaml_rust;

use std::error::Error;
use std::fs::File;
use std::io::Read;
use self::yaml_rust::{Yaml, YamlLoader};

// TODO: Add error processing more Rust idiomatic way (via returning Result object)


fn read_file(filepath: &str) -> String {
    let mut file = match File::open(filepath) {
        Ok(file) => file,
        Err(why) => panic!("Couldn't open file: {}", why.description())
    };

    let mut data = String::new();
    match file.read_to_string(&mut data) {
        Ok(_) => {},
        Err(why) => panic!("Couldn't read file: {}", why.description())
    };

    data
}


pub fn load_config(filepath: &str) -> Yaml {
    let data = match filepath {
        "" => "---".to_string(),
        _ => read_file(filepath)
    };

    let docs = YamlLoader::load_from_str(data.as_str()).unwrap();
    docs[0].clone()
}


#[cfg(test)]
mod tests {
    use super::{load_config, read_file};
    use super::Yaml;

    #[test]
    #[should_panic]
    fn test_read_file_will_panic_for_invalid_filepath() {
        read_file(&"invalid_path.yaml");
    }

    #[test]
    fn test_read_file_returns_yaml_data() {
        let data = read_file(&"./tests/files/valid_file.yaml");
        assert_eq!(data, "foo:\n  - bar\n");
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