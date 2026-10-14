//! End-to-end: the HOME-based `materialize`/`cleanup` write skills outside the repo,
//! never touch the repo's own `.claude/skills`, and remove the whole session base.
//! One test only: it mutates process-wide `HOME` and the cwd.

use nemonik_core::mcp::SkillSpec;
use nemonik_core::skills::{SkillLayout, cleanup, materialize, session_base};

#[test]
fn materialize_writes_out_of_repo_and_cleanup_removes_the_whole_base() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().to_path_buf();
    let repo = home.join("repo");
    let committed = repo.join(".claude").join("skills").join("gh");
    std::fs::create_dir_all(&committed).unwrap();
    std::fs::write(committed.join("SKILL.md"), "OLD COMMITTED CONTENT").unwrap();

    // SAFETY: single-threaded (one test in this binary); see module note.
    unsafe { std::env::set_var("HOME", &home) };
    std::env::set_current_dir(&repo).unwrap();

    let session = "nemonik-proj-tmpl-skillsbase";
    let skills = vec![SkillSpec {
        name: "gh".to_string(),
        description: "d".to_string(),
        body: "NEW KIT CONTENT".to_string(),
    }];

    // A fresh out-of-repo base, so no collision with the repo's committed `.claude/skills/gh`.
    materialize(&skills, session, SkillLayout::Claude).expect(
        "materialize must launch cleanly from a repo with a conflicting committed skill dir",
    );

    let base = session_base(session).unwrap();
    let out = base
        .join(".claude")
        .join("skills")
        .join("gh")
        .join("SKILL.md");
    assert!(
        out.exists(),
        "skill must be materialized under the out-of-repo session base"
    );
    let contents = std::fs::read_to_string(&out).unwrap();
    assert!(
        contents.starts_with("---\nname: \"gh\""),
        "must begin with name frontmatter: {contents:?}"
    );
    assert!(
        contents.contains("NEW KIT CONTENT"),
        "must contain the kit body"
    );
    assert!(
        base.starts_with(&home),
        "the base must live under ~/.nemonik, not the repo"
    );

    assert_eq!(
        std::fs::read_to_string(committed.join("SKILL.md")).unwrap(),
        "OLD COMMITTED CONTENT",
        "the launch cwd's repo tree must be untouched"
    );

    cleanup(session).unwrap();
    assert!(
        !base.exists(),
        "cleanup must remove the entire session base"
    );
}
