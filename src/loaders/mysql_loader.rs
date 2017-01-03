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
        .db_name(database.or(Some("solver".to_string())));
    my::Pool::new(opts).chain_err(|| "mysql connection")
}

fn load_projects(pool: &my::Pool) -> Result<Vec<Project>> {
    pool.prep_exec("SELECT id, intitule, quota_min, quota_max, occurrences FROM projets",
                   ())
        .map(|result| {
            result.map(|x| x.unwrap())
                .map(|row| {
                    let (id, name, min_students, max_students, max_occurrences) = my::from_row(row);
                    Project {
                        id: ProjectId(id),
                        name: name,
                        min_students: min_students,
                        max_students: max_students,
                        max_occurrences: max_occurrences,
                    }
                })
                .collect()
        })
        .chain_err(|| "loading projects")
}

fn load_students(pool: &my::Pool) -> Result<Vec<Student>> {
    pool.prep_exec("SELECT id, nom, prenom FROM eleves", ())
        .map(|result| {
            result.map(|x| x.unwrap())
                .map(|row| {
                    let (id, last_name, first_name): (usize, String, String) = my::from_row(row);
                    Student {
                        id: StudentId(id),
                        name: format!("{} {}", first_name, last_name),
                        rankings: Vec::new(),
                        bonuses: HashMap::new(),
                    }
                })
                .collect()
        })
        .chain_err(|| "loading students")
}

fn load_bonuses(pool: &my::Pool) -> Result<Vec<(StudentId, ProjectId, i32)>> {
    pool.prep_exec("SELECT eleve_id, projet_id, poids FROM pref_override", ())
        .map(|result| {
            result.map(|x| x.unwrap())
                .map(|row| {
                    let (student_id, project_id, weight) = my::from_row(row);
                    (StudentId(student_id), ProjectId(project_id), weight)
                })
                .collect()
        })
        .chain_err(|| "loading bonuses")
}

fn load_preferences(pool: &my::Pool) -> Result<Vec<(StudentId, ProjectId, i32)>> {
    pool.prep_exec("SELECT eleve_id, projet_id, poids FROM preferences", ())
        .map(|result| {
            result.map(|x| x.unwrap())
                .map(|row| {
                    let (student_id, project_id, weight) = my::from_row(row);
                    (StudentId(student_id), ProjectId(project_id), weight)
                })
                .collect()
        })
        .chain_err(|| "loading preferences")
}

impl Loader for MysqlLoader {
    fn load(&self, config: &Ini) -> Result<(Vec<Student>, Vec<Project>)> {
        let pool = pool(config)?;
        let mut projects = load_projects(&pool)?;
        let mut students = load_students(&pool)?;
        let preferences = load_preferences(&pool)?;
        let bonuses = load_bonuses(&pool)?;
        for student in students.iter_mut() {
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
