#![allow(clippy::cast_sign_loss)]

use crate::model::{Project, ProjectId, Student, StudentId};
use anyhow::{Context, Error};
use sqlx::any::{AnyConnectOptions, AnyRow};
use sqlx::{AnyConnection, Connection, Row};
use std::collections::HashMap;
use std::str::FromStr;
use tracing::trace;

pub struct Loader {
    conn: AnyConnection,
}

impl Loader {
    pub async fn new(s: &str) -> Result<Self, Error> {
        Ok(Self {
            conn: AnyConnection::connect_with(&AnyConnectOptions::from_str(s)?).await?,
        })
    }

    pub async fn load(&mut self) -> Result<(Vec<Student>, Vec<Project>), Error> {
        let projects = self.load_projects().await.context("cannot load projects")?;
        let mut students = self.load_students().await.context("cannot load students")?;
        let preferences = self
            .load_preferences()
            .await
            .context("cannot load rankings")?;
        let bonuses = self.load_bonuses().await.context("cannot load bonuses")?;
        for student in &mut students {
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
            if !student.bonuses.is_empty() {
                trace!(
                    student = %student,
                    bonuses = ?student
                        .bonuses
                        .iter()
                        .map(|(p, w)| (&projects[p.0].name, w))
                        .collect::<Vec<_>>(),
                    "student has been assigned bonuses",
                );
            }
        }
        Ok((students, projects))
    }

    async fn load_projects(&mut self) -> Result<Vec<Project>, Error> {
        sqlx::query("SELECT id, intitule, quota_min, quota_max, occurrences FROM projets")
            .map(|row: AnyRow| {
                Ok(Project {
                    id: ProjectId(row.get::<i32, _>("id") as usize),
                    name: row.get("intitule"),
                    min_students: row.get::<i32, _>("quota_min") as u32,
                    max_students: row.get::<i32, _>("quota_max") as u32,
                    max_occurrences: row.get::<i32, _>("occurrences") as u32,
                })
            })
            .fetch_all(&mut self.conn)
            .await?
            .into_iter()
            .collect()
    }

    async fn load_students(&mut self) -> Result<Vec<Student>, Error> {
        sqlx::query("SELECT id, prenom, nom FROM eleves")
            .map(|row: AnyRow| {
                Ok(Student::new(
                    StudentId(row.get::<i32, _>("id") as usize),
                    row.get("prenom"),
                    row.get("nom"),
                    Vec::new(),
                    HashMap::new(),
                ))
            })
            .fetch_all(&mut self.conn)
            .await?
            .into_iter()
            .collect()
    }

    async fn load_bonuses(&mut self) -> Result<Vec<(StudentId, ProjectId, i64)>, Error> {
        sqlx::query("SELECT eleve_id, projet_id, poids FROM pref_override")
            .map(|row: AnyRow| {
                Ok((
                    StudentId(row.get::<i32, _>("eleve_id") as usize),
                    ProjectId(row.get::<i32, _>("projet_id") as usize),
                    row.get("poids"),
                ))
            })
            .fetch_all(&mut self.conn)
            .await?
            .into_iter()
            .collect()
    }

    async fn load_preferences(&mut self) -> Result<Vec<(StudentId, ProjectId, i64)>, Error> {
        sqlx::query("SELECT eleve_id, projet_id, poids FROM preferences")
            .map(|row: AnyRow| {
                Ok((
                    StudentId(row.get::<i32, _>("eleve_id") as usize),
                    ProjectId(row.get::<i32, _>("projet_id") as usize),
                    row.get("poids"),
                ))
            })
            .fetch_all(&mut self.conn)
            .await?
            .into_iter()
            .collect()
    }

    #[allow(clippy::cast_possible_wrap)]
    pub async fn save_assignments(
        &mut self,
        assignments: &[(StudentId, ProjectId)],
        unassigned: &[StudentId],
    ) -> Result<(), Error> {
        let mut trans = self.conn.begin().await?;
        for (s, p) in assignments {
            sqlx::query("UPDATE eleves SET attribution=? WHERE id=?")
                .bind(p.0 as i32)
                .bind(s.0 as i32)
                .execute(&mut trans)
                .await
                .context("cannot save attributions")?;
        }
        for s in unassigned {
            sqlx::query("UPDATE eleves SET attribution=NULL WHERE id=?")
                .bind(s.0 as i32)
                .execute(&mut trans)
                .await
                .context("cannot delete attribution for unassigned student")?;
        }
        trans
            .commit()
            .await
            .context("error when committing transaction")?;
        Ok(())
    }
}
