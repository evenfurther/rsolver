use crate::model::{Assignments, StudentId};
use crate::stats;

pub fn display_details(a: &Assignments, rename_lazy: bool) {
    let mut projects = a.projects.clone();
    projects.sort_by_key(|ref p| p.name.clone());
    for p in &projects {
        let mut lazy_index = 0;
        let mut students = a
            .students_for(p.id)
            .iter()
            .map(|&s| {
                (
                    if rename_lazy && a.student(s).is_lazy() {
                        lazy_index += 1;
                        format!("Zzz {}", lazy_index)
                    } else {
                        a.student(s).name.clone()
                    },
                    s,
                )
            })
            .collect::<Vec<_>>();
        students.sort_by_key(|(name, _)| name.clone());
        if !students.is_empty() {
            println!("{}:", p.name);
            for (name, s) in students {
                print!("  - {}", name);
                if let Some(rank) = a.rank_of(s, p.id) {
                    print!(" (rank {})", rank + 1);
                }
                if a.is_pinned_and_has_chosen(s, p.id) {
                    print!(" (pinned)");
                }
                println!();
            }
            println!();
        }
    }
}

pub fn display_stats(a: &Assignments, eliminated: usize) {
    let students = a.students.len();
    let lazy = (0..students)
        .filter(|&s| a.rankings(StudentId(s)).is_empty())
        .count();
    assert!(
        lazy == 0 || eliminated == 0,
        "cannot have lazy students if they have been eliminated"
    );
    let (unconsidered_str, unconsidered) = if eliminated > 0 {
        ("unconsidered", eliminated)
    } else {
        ("unregistered", lazy)
    };
    println!(
        "Students registered/{}/total: {}/{}/{}",
        unconsidered_str,
        students - lazy,
        unconsidered,
        students + eliminated,
    );
    let ranks = stats::statistics(a);
    let cumul = ranks.iter().scan(0, |s, &r| {
        *s += r;
        Some(*s)
    });
    let total: usize = ranks.iter().sum();
    println!("Final ranking:");
    for (rank, (n, c)) in ranks.iter().zip(cumul).enumerate() {
        if *n != 0 {
            println!(
                "  - rank {}: {} (cumulative {} - {:.2}%)",
                rank + 1,
                n,
                c,
                100.0 * c as f32 / total as f32
            );
        }
    }
}

pub fn display_empty(a: &Assignments) {
    let mut projects = a.filter_projects(|p| !a.is_open(p));
    projects.sort_by_key(|&p| a.project(p).name.clone());
    if !projects.is_empty() {
        println!("Empty projects:");
        for p in projects {
            println!("  - {}", a.project(p).name);
        }
    }
}

pub fn display_with_many_lazy(a: &Assignments) {
    let mut projects = a
        .filter_projects(|p| a.is_open(p))
        .iter()
        .filter_map(|&p| {
            let lazy = a.students_for(p).iter().filter(|&&s| a.is_lazy(s)).count();
            let regular = a.students_for(p).len() - lazy;
            if lazy >= regular {
                Some((p, regular, lazy))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    projects.sort_by_key(|&(p, _, _)| a.project(p).name.clone());
    if !projects.is_empty() {
        println!(
            "Projects with at least half the members being unregistered students (unregistered/total):"
        );
        for (p, regular, lazy) in projects {
            println!("  - {} ({}/{})", a.project(p).name, lazy, lazy + regular);
        }
    }
}

pub fn display_missed_bonuses(a: &Assignments) {
    let mut missed_bonuses = a
        .all_students()
        .into_iter()
        .flat_map(|s| {
            if let Some(p) = a.project_for(s) {
                if a.bonus(s, p).is_some() {
                    vec![]
                } else if let Some(r) = a.rank_of(s, p) {
                    let mut bonuses = a
                        .bonuses(s)
                        .iter()
                        .filter_map(|(&pp, &b)| {
                            a.rank_of(s, pp).and_then(|rr| {
                                if rr < r && b > 0 {
                                    Some((s, p, r, pp, rr, b))
                                } else {
                                    None
                                }
                            })
                        })
                        .collect::<Vec<_>>();
                    bonuses.sort_by_key(|&(_s, _p, _r, _pp, rr, _b)| rr);
                    bonuses
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        })
        .collect::<Vec<_>>();
    if !missed_bonuses.is_empty() {
        missed_bonuses.sort_by_key(|&(s, _p, _r, pp, _rr, b)| {
            (a.project(pp).name.clone(), -b, a.student(s).name.clone())
        });
        println!("Useless bonuses:");
        for (s, p, r, pp, rr, b) in missed_bonuses {
            println!(
                "  - {} was assigned to {} (rank {}) despite having a bonus of {} for {} (rank {})",
                a.student(s).name,
                a.project(p).name,
                r + 1,
                b,
                a.project(pp).name,
                rr + 1
            );
        }
    }
}
