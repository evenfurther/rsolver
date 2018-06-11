use super::loader::Loader;
use errors::*;
use get_config;
use mysql as my;
use std::collections::HashMap;
use types::*;
use Config;

pub struct MysqlLoader {
    pool: my::Pool,
    students: Vec<Student>,
    projects: Vec<Project>,
}

macro_rules! load {
    ($name:ident, $query:expr, $ty:ty, $pattern:pat, $value:expr) => {
        fn $name(&self) -> my::Result<Vec<$ty>> {
            self.pool.prep_exec($query, ()).and_then(|result| {
                result
                    .map(|row| {
                        row.map(|row| {
                            let $pattern = my::from_row(row);
                            $value
                        })
                    })
                .collect()
            })
        }
    };
}

impl MysqlLoader {
    pub fn new(config: &Config) -> Result<MysqlLoader> {
        let host = get_config(config, "mysql", "host");
        let port = get_config(config, "mysql", "port")
            .map(|p| p.parse::<u16>().chain_err(|| "parsing mysql port"))
            .unwrap_or(Ok(3306))?;
        let user = get_config(config, "mysql", "user");
        let password = get_config(config, "mysql", "password");
        let database = get_config(config, "mysql", "database");
        let force_tcp = get_config(config, "mysql", "force-tcp")
            .map(|p| p.parse::<bool>().chain_err(|| "parsing force-tcp"))
            .unwrap_or(Ok(false))?;
        let mut opts = my::OptsBuilder::new();
        opts.ip_or_hostname(host)
            .tcp_port(port)
            .prefer_socket(!force_tcp)
            .user(user)
            .pass(password)
            .db_name(database);
        my::Pool::new(opts)
            .chain_err(|| "mysql connection")
            .map(|pool| MysqlLoader {
                pool,
                students: Vec::new(),
                projects: Vec::new(),
            })
    }

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
}

impl Loader for MysqlLoader {
    fn load(&mut self) -> Result<(Vec<Student>, Vec<Project>)> {
        self.projects = self.load_projects().chain_err(|| "cannot load projects")?;
        self.students = self.load_students().chain_err(|| "cannot load students")?;
        let preferences = self
            .load_preferences()
            .chain_err(|| "cannot load rankings")?;
        let bonuses = self.load_bonuses().chain_err(|| "cannot load bonuses")?;
        for student in &mut self.students {
            let mut preferences = preferences
                .iter()
                .filter_map(|&(s, p, w)| if s == student.id { Some((p, w)) } else { None })
                .collect::<Vec<_>>();
            preferences.sort_by_key(|&(_, w)| w);
            student.rankings = preferences.into_iter().map(|(p, _)| p).collect();
            student.bonuses = bonuses
                .iter()
                .filter_map(|&(s, p, w)| if s == student.id { Some((p, -w)) } else { None })
                .collect();
        }
        let mut students = self.students.clone();
        let mut projects = self.projects.clone();
        super::remap(&mut students, &mut projects);
        Ok((students, projects))
    }

    fn save(&self, assignments: &Assignments) -> Result<()> {
        let mut stmt = self
            .pool
            .prepare("UPDATE eleves SET attribution=:attribution WHERE id=:id")
            .chain_err(|| "cannot prepare statement")?;
        for s in &assignments.students {
            stmt.execute(params!{
                "id" => self.students[s.id.0].id.0,
                "attribution" => self.projects[assignments.project_for(s.id).unwrap().0].id.0
            }).chain_err(|| "cannot save attributions")?;
        }
        Ok(())
    }
}
