use super::loader::Loader;
use crate::model::{Project, ProjectId, Student, StudentId};
use crate::{get_config, Config};
use anyhow::{Context, Error};
use async_trait::async_trait;
use sqlx::mysql::{MySqlConnectOptions, MySqlPoolOptions};
use sqlx::{MySql, Pool};
use std::collections::HashMap;

pub struct MysqlLoader {
    pool: Pool<MySql>,
}

impl MysqlLoader {
    pub async fn new(config: &Config) -> Result<MysqlLoader, Error> {
        let host = get_config(config, "mysql", "host").unwrap_or_else(|| String::from("localhost"));
        let port = get_config(config, "mysql", "port")
            .map_or(Ok(3306), |p| p.parse::<u16>().context("parsing mysql port"))?;
        let user = get_config(config, "mysql", "user").expect("missing user");
        let password = get_config(config, "mysql", "password");
        let database = get_config(config, "mysql", "database").expect("missing database");
        let mut options = MySqlConnectOptions::new()
            .database(&database)
            .username(&user)
            .host(&host)
            .port(port);
        if let Some(password) = password {
            options = options.password(&password);
        }
        Ok(MysqlLoader {
            pool: MySqlPoolOptions::new().connect_with(options).await?,
        })
    }
}

#[async_trait]
impl Loader for MysqlLoader {
    async fn load_projects(&self) -> Result<Vec<Project>, Error> {
        sqlx::query!("SELECT id, intitule, quota_min, quota_max, occurrences FROM projets")
            .map(|row| {
                Ok(Project {
                    id: ProjectId(row.id as usize),
                    name: row.intitule,
                    min_students: row.quota_min as u32,
                    max_students: row.quota_max as u32,
                    max_occurrences: row.occurrences as u32,
                })
            })
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .collect()
    }

    async fn load_students(&self) -> Result<Vec<Student>, Error> {
        sqlx::query!("SELECT id, prenom, nom FROM eleves")
            .map(|row| {
                Ok(Student::new(
                    StudentId(row.id as usize),
                    row.prenom,
                    row.nom,
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
        sqlx::query!("SELECT eleve_id, projet_id, poids FROM pref_override")
            .map(|row| {
                Ok((
                    StudentId(row.eleve_id as usize),
                    ProjectId(row.projet_id as usize),
                    row.poids as i64,
                ))
            })
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .collect()
    }

    async fn load_preferences(&self) -> Result<Vec<(StudentId, ProjectId, i64)>, Error> {
        sqlx::query!("SELECT eleve_id, projet_id, poids FROM preferences")
            .map(|row| {
                Ok((
                    StudentId(row.eleve_id as usize),
                    ProjectId(row.projet_id as usize),
                    row.poids as i64,
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
            sqlx::query!(
                "UPDATE eleves SET attribution=? WHERE id=?",
                p.0 as i32,
                s.0 as u32
            )
            .execute(&self.pool)
            .await
            .context("cannot save attributions")?;
        }
        for s in unassigned {
            sqlx::query!("UPDATE eleves SET attribution=NULL WHERE id=?", s.0 as u32)
                .execute(&self.pool)
                .await
                .context("cannot delete attribution for unassigned student")?;
        }
        Ok(())
    }
}
