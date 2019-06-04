#![cfg(feature = "sqlite")]

use super::loader::Loader;
use crate::get_config;
use crate::model::*;
use crate::Config;
use failure::{format_err, Error, ResultExt};
use rusqlite::{Connection, NO_PARAMS};
use std::collections::HashMap;

pub struct SqliteLoader {
    conn: Connection,
    students: Vec<Student>,
    projects: Vec<Project>,
}

impl SqliteLoader {
    pub fn new(config: &Config) -> Result<SqliteLoader, Error> {
        let filename = get_config(config, "sqlite", "file")
            .ok_or_else(|| format_err!("cannot find sqlite file"))?;
        Ok(SqliteLoader {
            conn: Connection::open(filename)?,
            students: Vec::new(),
            projects: Vec::new(),
        })
    }

    pub fn load_projects(&self) -> Result<Vec<Project>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, intitule, quota_min, quota_max, occurrences FROM projets")?;
        let result = stmt
            .query_map(NO_PARAMS, |row| {
                Ok(Project {
                    id: ProjectId(row.get::<_, u32>(0)? as usize),
                    name: row.get(1)?,
                    min_students: row.get::<_, u32>(2)? as usize,
                    max_students: row.get::<_, u32>(3)? as usize,
                    max_occurrences: row.get::<_, u32>(4)? as usize,
                })
            })?
            .collect();
        result
    }

    pub fn load_students(&self) -> Result<Vec<Student>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, prenom || ' ' || nom FROM eleves")?;
        let result = stmt
            .query_map(NO_PARAMS, |row| {
                Ok(Student {
                    id: StudentId(row.get::<_, u32>(0)? as usize),
                    name: row.get(1)?,
                    rankings: Vec::new(),
                    bonuses: HashMap::new(),
                })
            })?
            .collect();
        result
    }

    pub fn load_bonuses(&self) -> Result<Vec<(StudentId, ProjectId, isize)>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT eleve_id, projet_id, poids FROM pref_override")?;
        let result = stmt
            .query_map(NO_PARAMS, |row| {
                Ok((
                    StudentId(row.get::<_, u32>(0)? as usize),
                    ProjectId(row.get::<_, u32>(1)? as usize),
                    row.get::<_, i32>(2)? as isize,
                ))
            })?
            .collect();
        result
    }

    pub fn load_preferences(&self) -> Result<Vec<(StudentId, ProjectId, isize)>, rusqlite::Error> {
        let mut stmt = self
            .conn
            .prepare("SELECT eleve_id, projet_id, poids FROM preferences")?;
        let result = stmt
            .query_map(NO_PARAMS, |row| {
                Ok((
                    StudentId(row.get::<_, u32>(0)? as usize),
                    ProjectId(row.get::<_, u32>(1)? as usize),
                    row.get::<_, i32>(2)? as isize,
                ))
            })?
            .collect();
        result
    }
}

impl Loader for SqliteLoader {
    fn load(&mut self) -> Result<(Vec<Student>, Vec<Project>), Error> {
        self.projects = self.load_projects().context("cannot load projects")?;
        self.students = self.load_students().context("cannot load students")?;
        let preferences = self.load_preferences().context("cannot load rankings")?;
        let bonuses = self.load_bonuses().context("cannot load bonuses")?;
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

    fn save(&self, assignments: &Assignments) -> Result<(), Error> {
        for s in &assignments.students {
            self.conn
                .execute(
                    "UPDATE eleves SET attribution=?1 WHERE id=?2",
                    &[
                        self.projects[assignments.project_for(s.id).unwrap().0].id.0 as u32,
                        self.students[s.id.0].id.0 as u32,
                    ],
                )
                .context("cannot save attributions")?;
        }
        Ok(())
    }
}
