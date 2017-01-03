use ini::Ini;
use mysql as my;
use project::Project;
use student::Student;
use super::loader::Loader;

pub struct MysqlLoader;

fn pool(config: &Ini) -> Result<my::Pool, String> {
    let (host, port, user, password, database) = match config.section(Some("mysql".to_string())) {
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

fn load_projects(pool: &my::Pool) -> Result<Vec<Project>, String> {
    pool.prep_exec("SELECT id, intitule, quota_min, quota_max, occurrences FROM projets",
                   ())
        .map(|result| {
            result.map(|x| x.unwrap())
                .map(|row| {
                    let (id, name, min_students, max_students, max_occurrences) = my::from_row(row);
                    Project {
                        id: id,
                        name: name,
                        min_students: min_students,
                        max_students: max_students,
                        max_occurrences: max_occurrences,
                    }
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

fn load_bonuses(pool: &my::Pool) -> Result<Vec<(usize, usize, i32)>, String> {
    pool.prep_exec("SELECT eleve_id, project_id, poids FROM prefs_override", ())
        .map(|result| {
            result.map(|x| x.unwrap())
                .map(|row| {
                    let (student_id, project_id, weight) = my::from_row(row);
                    (student_id, project_id, weight)
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

fn load_preferences(pool: &my::Pool) -> Result<Vec<(usize, usize, i32)>, String> {
    pool.prep_exec("SELECT eleve_id, project_id, poids FROM preferences", ())
        .map(|result| {
            result.map(|x| x.unwrap())
                .map(|row| {
                    let (student_id, project_id, weight) = my::from_row(row);
                    (student_id, project_id, weight)
                })
                .collect()
        })
        .map_err(|e| e.to_string())
}

impl Loader for MysqlLoader {
    fn load(&self, config: &Ini) -> Result<(Vec<Student>, Vec<Project>), String> {
        let pool = pool(config)?;
        let projects = load_projects(&pool)?;
        Err("FIXME".to_string())
    }
}
