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
    let projects = a.filter_projects(|p| !a.is_open(p));
    if !projects.is_empty() {
        println!("Empty projects:");
        for p in projects {
            println!("  - {}", a.project(p).name);
        }
    }
}
