use ini::Ini;
use mysql as my;
use project::Project;
use student::Student;
use super::loader::Loader;

pub struct MysqlLoader;

impl MysqlLoader {
    fn pool(config: &Ini) -> Result<my::Pool, String> {
        let (host, port, user, password, database) =
            match config.section(Some("mysql".to_string())) {
                Some(section) => {
                    let port = section.get("port")
                        .map(|p| p.parse::<u16>().map_err(|e| e.to_string()));
                    (section.get("host").cloned(),
                     port,
                     section.get("user").cloned(),
                     section.get("password").cloned(),
                     section.get("database").cloned())
                }
                None => (None, None, None, None, None),
            };
        let mut opts = my::OptsBuilder::new();
        opts.ip_or_hostname(host)
            .tcp_port(port.unwrap_or(Ok(3306))?)
            .user(user)
            .pass(password)
            .db_name(database.or(Some("solver".to_string())));
        my::Pool::new(opts).map_err(|e| e.to_string())
    }
}

impl Loader for MysqlLoader {
    fn load(&self, config: &Ini) -> Result<(Vec<Student>, Vec<Project>), String> {
        let pool = Self::pool(config)?;
        Err("FIXME".to_string())
    }
}
