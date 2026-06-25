use serde::{Deserialize, Serialize};

use crate::domain::{DockerSnapshot, Project};
use crate::health::project_fingerprints;
use crate::resources::ResourcePanelData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InboxSeverity {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboxItem {
    pub severity: InboxSeverity,
    pub category: String,
    pub project: Option<String>,
    pub title: String,
    pub detail: String,
    pub command: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpsInbox {
    pub items: Vec<InboxItem>,
}

pub fn build_ops_inbox(
    snapshot: &DockerSnapshot,
    resource_data: Option<&ResourcePanelData>,
) -> OpsInbox {
    let mut items = Vec::new();

    for project in &snapshot.projects {
        if project.unhealthy > 0 || project.restarting > 0 {
            items.push(project_risk_item(project, InboxSeverity::Critical));
        } else if project.paused > 0 {
            items.push(project_risk_item(project, InboxSeverity::Warning));
        }
    }

    for fingerprint in project_fingerprints(snapshot)
        .into_iter()
        .filter(|item| item.risk_score > 0)
        .take(3)
    {
        items.push(InboxItem {
            severity: if fingerprint.risk_score >= 40 {
                InboxSeverity::Critical
            } else {
                InboxSeverity::Warning
            },
            category: "Risk Fingerprint".to_string(),
            project: Some(fingerprint.project.clone()),
            title: format!(
                "{} risk score {}",
                fingerprint.project, fingerprint.risk_score
            ),
            detail: fingerprint.signals.join(", "),
            command: fingerprint.suggested_command,
        });
    }

    if let Some(data) = resource_data.filter(|data| !data.loading) {
        for row in data.rows.iter().filter(|row| row.error.is_none()) {
            if row.cpu_percent >= 80.0 {
                items.push(InboxItem {
                    severity: InboxSeverity::Warning,
                    category: "Resource Pressure".to_string(),
                    project: Some(data.project.clone()),
                    title: format!("High CPU: {}", row.container_name),
                    detail: format!("{:.1}% CPU in {}", row.cpu_percent, data.project),
                    command: format!("hugdocker stats {} --json", row.container_id),
                });
            }
            if row.memory_percent >= 85.0 {
                items.push(InboxItem {
                    severity: InboxSeverity::Warning,
                    category: "Resource Pressure".to_string(),
                    project: Some(data.project.clone()),
                    title: format!("High memory: {}", row.container_name),
                    detail: format!("{:.1}% memory in {}", row.memory_percent, data.project),
                    command: format!("hugdocker stats {} --json", row.container_id),
                });
            }
        }
        for row in data.rows.iter().filter(|row| row.error.is_some()) {
            items.push(InboxItem {
                severity: InboxSeverity::Warning,
                category: "Resource Pressure".to_string(),
                project: Some(data.project.clone()),
                title: format!("Stats error: {}", row.container_name),
                detail: row
                    .error
                    .clone()
                    .unwrap_or_else(|| "stats failed".to_string()),
                command: format!("hugdocker stats {}", row.container_id),
            });
        }
    }

    let stopped = snapshot
        .projects
        .iter()
        .map(|project| project.stopped)
        .sum::<usize>();
    if stopped > 0 {
        items.push(InboxItem {
            severity: InboxSeverity::Info,
            category: "Cleanup".to_string(),
            project: None,
            title: format!("{stopped} stopped containers can be reviewed"),
            detail: "Safe prune excludes volumes by default.".to_string(),
            command: "hugdocker safe-prune --dry-run".to_string(),
        });
    }

    if let Some(project) = snapshot
        .projects
        .iter()
        .find(|project| project.unhealthy > 0 || project.restarting > 0)
    {
        items.push(InboxItem {
            severity: InboxSeverity::Info,
            category: "Next Action".to_string(),
            project: Some(project.name.clone()),
            title: format!("Run rescue preflight for {}", project.name),
            detail: "Review restart impact before touching containers.".to_string(),
            command: format!("hugdocker rescue {} --dry-run", project.name),
        });
    } else if let Some(project) = snapshot
        .projects
        .iter()
        .find(|project| project.active() > 0)
    {
        items.push(InboxItem {
            severity: InboxSeverity::Info,
            category: "Next Action".to_string(),
            project: Some(project.name.clone()),
            title: format!("Inspect {}", project.name),
            detail: "No critical project found; inspect active workload if needed.".to_string(),
            command: format!("hugdocker inspect {}", project.name),
        });
    }

    sort_inbox_items(&mut items);
    if items.is_empty() {
        items.push(InboxItem {
            severity: InboxSeverity::Info,
            category: "Next Action".to_string(),
            project: None,
            title: "No urgent action".to_string(),
            detail: "No projects or resource pressure found in the current snapshot.".to_string(),
            command: "hugdocker list".to_string(),
        });
    }

    OpsInbox { items }
}

fn project_risk_item(project: &Project, severity: InboxSeverity) -> InboxItem {
    let mut signals = Vec::new();
    if project.unhealthy > 0 {
        signals.push(format!("{} unhealthy", project.unhealthy));
    }
    if project.restarting > 0 {
        signals.push(format!("{} restarting", project.restarting));
    }
    if project.paused > 0 {
        signals.push(format!("{} paused", project.paused));
    }
    InboxItem {
        severity,
        category: "Critical".to_string(),
        project: Some(project.name.clone()),
        title: format!("{} needs attention", project.name),
        detail: signals.join(", "),
        command: format!(
            "hugdocker doctor --json | jq '.projects[] | select(.project==\"{}\")'",
            project.name
        ),
    }
}

fn sort_inbox_items(items: &mut [InboxItem]) {
    items.sort_by(|a, b| {
        severity_rank(a.severity)
            .cmp(&severity_rank(b.severity))
            .then_with(|| category_rank(&a.category).cmp(&category_rank(&b.category)))
            .then_with(|| a.project.cmp(&b.project))
            .then_with(|| a.title.cmp(&b.title))
    });
}

fn severity_rank(severity: InboxSeverity) -> usize {
    match severity {
        InboxSeverity::Critical => 0,
        InboxSeverity::Warning => 1,
        InboxSeverity::Info => 2,
    }
}

fn category_rank(category: &str) -> usize {
    match category {
        "Critical" => 0,
        "Risk Fingerprint" => 1,
        "Resource Pressure" => 2,
        "Cleanup" => 3,
        "Next Action" => 4,
        _ => 5,
    }
}
