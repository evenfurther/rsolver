#![cfg(feature = "sqlite")]

use super::loader::Loader;
use crate::model::{Project, ProjectId, Student, StudentId};
use crate::{get_config, Config};
use anyhow::{format_err, Context, Error};
use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions, SqliteRow};
use sqlx::{Pool, Row, Sqlite};
use std::collections::HashMap;

pub struct SqliteLoader {
    pool: Pool<Sqlite>,
}

impl SqliteLoader {
    pub async fn new(config: &Config) -> Result<SqliteLoader, Error> {
        let filename = get_config(config, "sqlite", "file")
            .ok_or_else(|| format_err!("cannot find sqlite file"))?;
        let options = SqliteConnectOptions::new().filename(filename);
        Ok(SqliteLoader {
            pool: SqlitePoolOptions::new().connect_with(options).await?,
        })
    }
}

#[async_trait]
impl Loader for SqliteLoader {
    async fn load_projects(&self) -> Result<Vec<Project>, Error> {
        sqlx::query("SELECT id, intitule, quota_min, quota_max, occurrences FROM projets")
            .map(|row: SqliteRow| {
                Ok(Project {
                    id: ProjectId(row.get::<u32, _>("id") as usize),
                    name: row.get("intitule"),
                    min_students: row.get("quota_min"),
                    max_students: row.get("quota_max"),
                    max_occurrences: row.get("ocurrences"),
                })
            })
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .collect()
    }

    async fn load_students(&self) -> Result<Vec<Student>, Error> {
        sqlx::query("SELECT id, prenom, nom FROM eleves")
            .map(|row: SqliteRow| {
                Ok(Student::new(
                    StudentId(row.get::<u32, _>("id") as usize),
                    row.get("prenom"),
                    row.get("nom"),
                    Vec::new(),
                    HashMap::new(),
                ))
            })
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .collect()
    }

    async fn load_bonuses(&self) -> Result<Vec<(StudentId, ProjectId, i64)>, Error> {
        sqlx::query("SELECT eleve_id, projet_id, poids FROM pref_override")
            .map(|row: SqliteRow| {
                Ok((
                    StudentId(row.get::<u32, _>("eleve_id") as usize),
                    ProjectId(row.get::<u32, _>("projet_id") as usize),
                    row.get("poids"),
                ))
            })
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .collect()
    }

    async fn load_preferences(&self) -> Result<Vec<(StudentId, ProjectId, i64)>, Error> {
        sqlx::query("SELECT eleve_id, projet_id, poids FROM preferences")
            .map(|row: SqliteRow| {
                Ok((
                    StudentId(row.get::<u32, _>("eleve_id") as usize),
                    ProjectId(row.get::<u32, _>("projet_id") as usize),
                    row.get("poids"),
                ))
            })
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .collect()
    }

    async fn save_assignments(
        &self,
        assignments: &[(StudentId, ProjectId)],
        unassigned: &[StudentId],
    ) -> Result<(), Error> {
        for (s, p) in assignments {
            sqlx::query("UPDATE eleves SET attribution=? WHERE id=?")
                .bind(p.0 as u32)
                .bind(s.0 as u32)
                .execute(&self.pool)
                .await
                .context("cannot save attributions")?;
        }
        for s in unassigned {
            sqlx::query("UPDATE eleves SET attribution=NULL WHERE id=?")
                .bind(s.0 as u32)
                .execute(&self.pool)
                .await
                .context("cannot delete attribution for unassigned student")?;
        }
        Ok(())
    }
}
