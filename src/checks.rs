use crate::model::Assignments;
use failure::{bail, Error};

pub fn check_pinned_consistency(a: &Assignments) {
    for s in a.all_students() {
        if let Some(p) = a.rankings(s).get(0) {
            if a.is_pinned_for(s, *p) && a.project_for(s) != Some(*p) {
                warn!(
                    "WARNING: student {} did not get pinned project {}",
                    a.student(s).name,
                    a.project(*p).name
                );
            }
        }
    }
}

pub fn ensure_acceptable(a: &Assignments) -> Result<(), Error> {
    if let Some(unacceptable) = a
        .all_projects()
        .iter()
        .find(|&&p| a.is_open(p) && !a.is_acceptable(p))
    {
        bail!(
            "project {} has an unacceptable number of students",
            a.project(*unacceptable).name
        );
    }
    Ok(())
}
