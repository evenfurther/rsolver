use super::loader::Loader;
use crate::get_config;
use crate::model::*;
use crate::Config;
use failure::{Error, ResultExt};
use my::params;
use mysql as my;
use std::collections::HashMap;

pub struct MysqlLoader {
    pool: my::Pool,
    students: Vec<Student>,
    projects: Vec<Project>,
}

macro_rules! load {
    ($name:ident, $query:expr, $ty:ty, $pattern:pat, $value:expr) => {
        fn $name(&self) -> Result<Vec<$ty>, Error> {
            Ok(self.pool.prep_exec($query, ()).and_then(|result| {
                result
                    .map(|row| {
                        row.map(|row| {
                            let $pattern = my::from_row(row);
                            $value
                        })
                    })
                .collect::<Result<Vec<_>, _>>()
            })?)
        }
    };
}

impl MysqlLoader {
    pub fn new(config: &Config) -> Result<MysqlLoader, Error> {
        let host = get_config(config, "mysql", "host");
        let port = get_config(config, "mysql", "port")
            .map(|p| p.parse::<u16>().context("parsing mysql port"))
            .unwrap_or(Ok(3306))?;
        let user = get_config(config, "mysql", "user");
        let password = get_config(config, "mysql", "password");
        let database = get_config(config, "mysql", "database");
        let force_tcp = get_config(config, "mysql", "force-tcp")
            .map(|p| p.parse::<bool>().context("parsing force-tcp"))
            .unwrap_or(Ok(false))?;
        let mut opts = my::OptsBuilder::new();
        opts.ip_or_hostname(host)
            .tcp_port(port)
            .prefer_socket(!force_tcp)
            .user(user)
            .pass(password)
            .db_name(database);
        Ok(my::Pool::new(opts)
            .context("mysql connection")
            .map(|pool| MysqlLoader {
                pool,
                students: Vec::new(),
                projects: Vec::new(),
            })?)
    }
}

impl Loader for MysqlLoader {
    load!(
        load_projects,
        "SELECT id, intitule, quota_min, quota_max, occurrences FROM projets",
        Project,
        (id, name, min_students, max_students, max_occurrences),
        Project {
            id: ProjectId(id),
            name,
            min_students,
            max_students,
            max_occurrences,
        }
    );

    load!(
        load_students,
        "SELECT id, CONCAT(prenom, ' ', nom) FROM eleves",
        Student,
        (id, name),
        Student {
            id: StudentId(id),
            name,
            rankings: Vec::new(),
            bonuses: HashMap::new(),
        }
    );

    load!(
        load_bonuses,
        "SELECT eleve_id, projet_id, poids FROM pref_override",
        (StudentId, ProjectId, isize),
        (student_id, project_id, weight),
        (StudentId(student_id), ProjectId(project_id), weight)
    );

    load!(
        load_preferences,
        "SELECT eleve_id, projet_id, poids FROM preferences",
        (StudentId, ProjectId, isize),
        (student_id, project_id, weight),
        (StudentId(student_id), ProjectId(project_id), weight)
    );

    fn store_projects(&mut self, projects: &[Project]) {
        self.projects = projects.to_vec();
    }

    fn store_students(&mut self, students: &[Student]) {
        self.students = students.to_vec();
    }

    fn save_assignments(&self, assignments: &[(StudentId, ProjectId)]) -> Result<(), Error> {
        let mut stmt = self
            .pool
            .prepare("UPDATE eleves SET attribution=:attribution WHERE id=:id")
            .context("cannot prepare statement")?;
        for (s, p) in assignments {
            stmt.execute(params! {
                "id" => s.0,
                "attribution" => p.0,
            })
            .context("cannot save attributions")?;
        }
        Ok(())
    }
}
