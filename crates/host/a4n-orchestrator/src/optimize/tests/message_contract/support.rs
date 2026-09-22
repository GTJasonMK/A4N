use super::*;

fn optimize_test_permissions() -> OptimizationAgentPermissions {
    OptimizationAgentPermissions {
        read_project_detail: true,
        read_blueprint: true,
        read_outlines: true,
        read_character_profiles: true,
        read_foreshadowing: true,
        automatic_writeback: false,
    }
}
