#![cfg(feature = "sqlite")]

use super::loader::Loader;
use crate::model::*;
use crate::{get_config, Config};
use failure::{format_err, Error, ResultExt};
use rusqlite::Connection;
use std::collections::HashMap;

pub struct SqliteLoader {
    conn: Connection,
}

macro_rules! load {
    ($name:ident, $query:expr, $ty:ty, $row:ident, $value:expr) => {
        fn $name(&self) -> Result<Vec<$ty>, Error> {
            let mut stmt = self.conn.prepare($query)?;
            let result = stmt
                .query_map([], |$row| Ok($value))?
                .collect::<Result<Vec<_>, _>>();
            Ok(result?)
        }
    };
}

impl SqliteLoader {
    pub fn new(config: &Config) -> Result<SqliteLoader, Error> {
        let filename = get_config(config, "sqlite", "file")
            .ok_or_else(|| format_err!("cannot find sqlite file"))?;
        Ok(SqliteLoader {
            conn: Connection::open(filename)?,
        })
    }
}

impl Loader for SqliteLoader {
    load!(
        load_projects,
        "SELECT id, intitule, quota_min, quota_max, occurrences FROM projets",
        Project,
        row,
        Project {
            id: ProjectId(row.get::<_, u32>(0)? as usize),
            name: row.get(1)?,
            min_students: row.get::<_, u32>(2)? as usize,
            max_students: row.get::<_, u32>(3)? as usize,
            max_occurrences: row.get::<_, u32>(4)? as usize,
        }
    );

    load!(
        load_students,
        "SELECT id, prenom, nom FROM eleves",
        Student,
        row,
        Student::new(
            StudentId(row.get::<_, u32>(0)? as usize),
            row.get(1)?,
            row.get(2)?,
            Vec::new(),
            HashMap::new()
        )
    );

    load!(
        load_bonuses,
        "SELECT eleve_id, projet_id, poids FROM pref_override",
        (StudentId, ProjectId, isize),
        row,
        (
            StudentId(row.get::<_, u32>(0)? as usize),
            ProjectId(row.get::<_, u32>(1)? as usize),
            row.get::<_, i32>(2)? as isize,
        )
    );

    load!(
        load_preferences,
        "SELECT eleve_id, projet_id, poids FROM preferences",
        (StudentId, ProjectId, isize),
        row,
        (
            StudentId(row.get::<_, u32>(0)? as usize),
            ProjectId(row.get::<_, u32>(1)? as usize),
            row.get::<_, i32>(2)? as isize,
        )
    );

    fn save_assignments(
        &self,
        assignments: &[(StudentId, ProjectId)],
        unassigned: &[StudentId],
    ) -> Result<(), Error> {
        for (s, p) in assignments {
            self.conn
                .execute(
                    "UPDATE eleves SET attribution=?1 WHERE id=?2",
                    [p.0 as u32, s.0 as u32],
                )
                .context("cannot save attributions")?;
        }
        for s in unassigned {
            self.conn
                .execute(
                    "UPDATE eleves SET attribution=NULL WHERE id=?1",
                    [s.0 as u32],
                )
                .context("cannot delete attribution for unassigned student")?;
        }
        Ok(())
    }
}
