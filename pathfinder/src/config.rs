extern crate yaml_rust;

use std::error::Error;
use std::fs::File;
use std::io::Read;
use self::yaml_rust::{Yaml, YamlLoader};


fn read_file(filepath: &str) -> String {
    let mut file = match File::open(filepath) {
        Ok(file) => file,
        Err(why) => panic!("Couldn't open file: {}", why.description()),
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
