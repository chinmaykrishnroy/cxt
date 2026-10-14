#[allow(dead_code)]
mod common;
use common::{assert_success, run_cxt, temp_dir};

#[test]
fn export_skill_creates_a_portable_guide_without_scanning_or_overwriting() {
    let directory = temp_dir("skill-export");
    let result = run_cxt(&directory, &["--export-skill"]);
    assert_success(&result);
    let target = directory.join("cxt-skill/SKILL.md");
    let guide = std::fs::read_to_string(&target).unwrap();
    assert!(guide.starts_with("---\nname: cxt\n"));
    assert!(guide.contains("--query") && guide.contains("--mcp") && guide.contains("--tui"));
    assert!(!directory.join("cxt-output.md").exists());
    let repeated = run_cxt(&directory, &["--export-skill"]);
    assert!(!repeated.status.success());
    assert_eq!(std::fs::read_to_string(target).unwrap(), guide);
}

#[test]
fn agent_defaults_match_project_skill_conventions() {
    for (agent, path) in [
        ("codex", ".agents/skills/cxt/SKILL.md"),
        ("claude", ".claude/skills/cxt/SKILL.md"),
        ("antigravity", ".agents/skills/cxt/SKILL.md"),
        ("devin", ".agents/skills/cxt/SKILL.md"),
        ("kiro", ".kiro/skills/cxt/SKILL.md"),
    ] {
        let directory = temp_dir(agent);
        assert_success(&run_cxt(
            &directory,
            &["--export-skill", "--skill-agent", agent],
        ));
        assert_eq!(
            std::fs::read_to_string(directory.join(path)).unwrap(),
            cxt_core::skill::GUIDE
        );
    }
}

#[test]
fn custom_destination_wins_and_does_not_read_project_configuration() {
    let directory = temp_dir("skill-custom");
    std::fs::write(directory.join(".cxt.toml"), "this is not valid TOML").unwrap();
    let path = directory.join("custom guide 日本語/SKILL.md");
    assert_success(&run_cxt(
        &directory,
        &[
            "--export-skill",
            path.to_str().unwrap(),
            "--skill-agent",
            "claude",
            "--scan-max-entries",
            "1",
        ],
    ));
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        cxt_core::skill::GUIDE
    );
    assert!(!directory.join(".claude").exists());
}

#[test]
fn invalid_skill_invocations_do_not_write_or_pack() {
    let directory = temp_dir("skill-invalid");
    for arguments in [
        vec!["--skill-agent", "codex"],
        vec!["--export-skill", "--mcp"],
        vec!["--export-skill", "--tui"],
        vec!["--export-skill", "--stdout"],
        vec!["--export-skill", "--skill-agent", "unknown"],
    ] {
        let result = run_cxt(&directory, &arguments);
        assert_eq!(result.status.code(), Some(2), "{arguments:?}: {result:?}");
    }
    assert!(!directory.join("cxt-skill").exists());
    assert!(!directory.join("cxt-output.md").exists());
    let result = run_cxt(&directory, &["--export-skill", "."]);
    assert_eq!(result.status.code(), Some(3));
}

#[test]
fn skill_is_self_contained_and_covers_every_public_long_option() {
    use clap::CommandFactory;
    let guide = cxt_core::skill::GUIDE;
    for argument in cxt_core::cli::Args::command().get_arguments() {
        if let Some(long) = argument.get_long() {
            assert!(
                guide.contains(&format!("--{long}")),
                "skill missing --{long}"
            );
        }
    }
    assert!(!guide.contains("0.5.0-dev"));
    assert!(guide.contains("--secrets fail") && guide.contains("not authority"));
}
