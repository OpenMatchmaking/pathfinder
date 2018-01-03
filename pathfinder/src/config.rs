extern crate config;

use self::config::{Config, File};


// Creates the default configuration for an application with data,
// read from file.
pub fn get_config(file_path: &str) -> Box<Config> {
    let mut conf = Box::new(Config::default());

    if file_path != "" {
        conf.merge(File::with_name(file_path))
            .map_err(|err|
                println!("Error during reading file: {}. \
                          Changes won't applied.", err)
            )
            .and_then(|new_config| Ok(new_config))
            .is_ok();
    }

    conf
}


#[cfg(test)]
mod tests {
    use super::{get_config};

    #[test]
    fn test_get_config_returns_a_new_config_by_default() {
        let conf = get_config(&"");
        assert_eq!(format!("{}", conf.cache), "nil");
    }

    #[test]
    fn test_get_config_returns_a_new_config_with_values_from_file() {
        let conf = get_config(&"./tests/files/valid_file.yaml");

        let data = conf.cache.into_table();
        assert_eq!(data.is_ok(), true);

        let table = data.unwrap();
        assert_eq!(table.contains_key("foo"), true);

        let foo_array = table["foo"].clone().into_array().unwrap();
        assert_eq!(foo_array.len(), 1);
        assert_eq!(foo_array[0].clone().into_str().unwrap(), "bar");
    }
}
