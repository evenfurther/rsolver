use errors::*;
use ini::Ini;
use mysql as my;
use std::collections::HashMap;
use super::loader::Loader;
use types::*;

pub struct MysqlLoader;

fn pool(config: &Ini) -> Result<my::Pool> {
    let (host, port, user, password, database) = match config.section(Some("mysql".to_string())) {
        Some(section) => {
            let port = section.get("port")
                .map(|p| p.parse::<u16>().chain_err(|| "parsing mysql port"));
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
        .db_name(database.or_else(|| Some("solver".to_string())));
    my::Pool::new(opts).chain_err(|| "mysql connection")
}

macro_rules! load {
    ($name:ident, $query:expr, $ty:ty, $pattern:pat, $value:expr) => {
        fn $name(pool: &my::Pool) -> my::Result<Vec<$ty>> {
            pool.prep_exec($query, ())
                .and_then(|result| {
                    result.map(|row| {
                        row.map(|row| {
                            let $pattern =
                                my::from_row(row);
                            $value
                        })
                    })
                    .collect()
                })
        }
    }
}

load!(load_projects, "SELECT id, intitule, quota_min, quota_max, occurrences FROM projets",
      Project, (id, name, min_students, max_students, max_occurrences),
      Project { id: ProjectId(id), name: name, min_students: min_students,
      max_students: max_students, max_occurrences: max_occurrences });

load!(load_students, "SELECT id, CONCAT(prenom, ' ', nom) FROM eleves",
      Student, (id, name), Student { id: StudentId(id), name: name,
      rankings: Vec::new(), bonuses: HashMap::new() });

load!(load_bonuses, "SELECT eleve_id, projet_id, poids FROM pref_override",
      (StudentId, ProjectId, i32), (student_id, project_id, weight),
      (StudentId(student_id), ProjectId(project_id), weight));

load!(load_preferences, "SELECT eleve_id, projet_id, poids FROM preferences",
      (StudentId, ProjectId, i32), (student_id, project_id, weight),
      (StudentId(student_id), ProjectId(project_id), weight));

impl Loader for MysqlLoader {
    fn load(&self, config: &Ini) -> Result<(Vec<Student>, Vec<Project>)> {
        let pool = pool(config)?;
        let mut projects = load_projects(&pool).chain_err(|| "cannot load projects")?;
        let mut students = load_students(&pool).chain_err(|| "cannot load students")?;
        let preferences = load_preferences(&pool).chain_err(|| "cannot load rankings")?;
        let bonuses = load_bonuses(&pool).chain_err(|| "cannot load bonuses")?;
        for student in &mut students {
            let mut preferences = preferences.iter()
                .filter_map(|&(s, p, w)| if s == student.id { Some((p, w)) } else { None })
                .collect::<Vec<_>>();
            preferences.sort_by_key(|&(_, w)| w);
            student.rankings = preferences.into_iter().map(|(p, _)| p).collect();
            student.bonuses = bonuses.iter()
                .filter_map(|&(s, p, w)| if s == student.id { Some((p, -w)) } else { None })
                .collect();
        }
        super::remap(&mut students, &mut projects);
        Ok((students, projects))
    }
}
