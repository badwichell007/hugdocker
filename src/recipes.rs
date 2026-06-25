use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Recipe {
    pub name: &'static str,
    pub description: &'static str,
    pub command: &'static str,
    pub risk: &'static str,
}

pub fn builtin_recipes() -> Vec<Recipe> {
    vec![
        Recipe {
            name: "panic-stop",
            description: "停止项目活动容器，适合临时释放端口、CPU 或快速止血。",
            command: "hugdocker stop <project> --dry-run",
            risk: "medium",
        },
        Recipe {
            name: "rescue-unhealthy",
            description: "对 unhealthy/restarting 项目生成恢复重启预案。",
            command: "hugdocker rescue <project> --dry-run",
            risk: "low",
        },
        Recipe {
            name: "preflight-delete",
            description: "删除前预演容器、网络、卷、镜像影响范围。",
            command: "hugdocker plan purge <project>",
            risk: "high",
        },
        Recipe {
            name: "safe-cleanup",
            description: "清理 stopped containers、unused networks 和 dangling images，默认排除 volumes。",
            command: "hugdocker safe-prune --dry-run",
            risk: "medium",
        },
    ]
}
