use failure::{Error, ResultExt};
use ini::Ini;

pub struct Config {
    conf: Ini,
}

impl Config {
    pub fn load(file_name: &str) -> Result<Config, Error> {
        Ok(Config {
            conf: Ini::load_from_file(file_name).context("cannot load configuration file")?,
        })
    }
}

pub fn get_config(config: &Config, section: &str, key: &str) -> Option<String> {
    config
        .conf
        .section(Some(section.to_owned()))
        .and_then(|s| s.get(key))
        .map(String::from)
}
