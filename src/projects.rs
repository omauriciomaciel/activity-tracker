use anyhow::Result;
use std::collections::{HashMap, HashSet};

use crate::summarizer::{LogEntry, read_log_entries};

pub struct ProjectStat {
    pub name: String,
    pub _path: String,
    pub commits: usize,
    pub days_active: usize,
    pub pct: f64,
}

pub fn load_stats(days: u32) -> Result<Vec<ProjectStat>> {
    let log_dir = crate::config::log_dir();
    let today = chrono::Local::now().date_naive();

    // repo_path -> (unique commit hashes/messages, days with any commit)
    let mut repo_data: HashMap<String, (HashSet<String>, usize)> = HashMap::new();

    for d in 0..days {
        let date = today - chrono::Duration::days(d as i64);
        let path = log_dir.join(format!("{}.jsonl", date.format("%Y-%m-%d")));

        let mut day_repos: HashSet<String> = HashSet::new();
        for entry in read_log_entries(&path) {
            if let LogEntry::Context { data } = entry {
                for r in data.git_repos {
                    let commits: Vec<String> = if !r.commits.is_empty() {
                        r.commits
                    } else if !r.last_commit.is_empty() {
                        vec![r.last_commit]
                    } else {
                        continue;
                    };

                    let entry = repo_data.entry(r.repo.clone()).or_default();
                    for c in commits {
                        entry.0.insert(c);
                    }
                    if day_repos.insert(r.repo.clone()) {
                        entry.1 += 1;
                    }
                }
            }
        }
    }

    let total_commits: usize = repo_data.values().map(|(cs, _)| cs.len()).sum();

    let mut stats: Vec<ProjectStat> = repo_data
        .into_iter()
        .map(|(path, (commits, days_active))| {
            let name = std::path::Path::new(&path)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&path)
                .to_string();
            let commit_count = commits.len();
            let pct = if total_commits > 0 {
                (commit_count as f64 / total_commits as f64) * 100.0
            } else {
                0.0
            };
            ProjectStat {
                name,
                _path: path,
                commits: commit_count,
                days_active,
                pct,
            }
        })
        .collect();

    stats.sort_by(|a, b| b.commits.cmp(&a.commits).then(a.name.cmp(&b.name)));
    Ok(stats)
}

pub fn format_context(stats: &[ProjectStat], days: u32) -> String {
    let mut out = format!("=== DISTRIBUIÇÃO DE PROJETOS (últimos {days} dias) ===\n");
    for s in stats.iter().take(10) {
        out.push_str(&format!(
            "  {}: {:.0}% ({} commits, {} dias ativos)\n",
            s.name, s.pct, s.commits, s.days_active
        ));
    }
    out.push('\n');
    out
}
