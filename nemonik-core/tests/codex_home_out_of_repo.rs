//! End-to-end: under the Codex layout, skills land in a per-session `CODEX_HOME`
//! seeded from the real `~/.codex`, which is never modified. One test only: it
//! mutates process-wide `HOME`/`CODEX_HOME` and the cwd.

use nemonik_core::mcp::SkillSpec;
use nemonik_core::skills::{SkillLayout, cleanup, codex_home_dir, materialize, session_base};

#[test]
fn codex_materialize_seeds_a_per_session_home_and_never_touches_the_real_one() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let home = root.join("home");
    let real_codex = root.join("real-codex");
    std::fs::create_dir_all(real_codex.join("skills/.system/imagegen")).unwrap();
    std::fs::write(real_codex.join("auth.json"), "REAL TOKEN").unwrap();
    std::fs::write(real_codex.join("config.toml"), "model = \"gpt-5.6-sol\"").unwrap();
    std::fs::create_dir_all(real_codex.join("skills/my-own")).unwrap();

    // SAFETY: single-threaded (one test in this binary); see module note.
    unsafe {
        std::env::set_var("HOME", &home);
        std::env::set_var("CODEX_HOME", &real_codex);
    }

    let session = "nemonik-proj-tmpl-codexhome";
    let skills = vec![SkillSpec {
        name: "nemonik".to_string(),
        description: "d".to_string(),
        body: "KIT SKILL".to_string(),
    }];

    materialize(&skills, session, SkillLayout::Codex).expect("codex materialize must succeed");

    let ch = codex_home_dir(session).unwrap();
    let out = ch.join("skills").join("nemonik").join("SKILL.md");
    let contents = std::fs::read_to_string(&out).unwrap();
    assert!(
        contents.starts_with("---\nname: \"nemonik\""),
        "must begin with name frontmatter: {contents:?}"
    );
    assert!(
        contents.contains("description: \"d\""),
        "must carry the skill description"
    );
    assert!(contents.contains("KIT SKILL"), "must contain the kit body");
    assert!(
        ch.starts_with(&home),
        "the codex home must live under ~/.nemonik, not the repo or real ~/.codex"
    );
    // auth.json is a symlink and config.toml a copy; both carry the real content.
    assert_eq!(
        std::fs::read_to_string(ch.join("auth.json")).unwrap(),
        "REAL TOKEN"
    );
    assert_eq!(
        std::fs::read_to_string(ch.join("config.toml")).unwrap(),
        "model = \"gpt-5.6-sol\""
    );
    assert!(
        std::fs::symlink_metadata(ch.join("auth.json"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(ch.join("skills").join(".system").join("imagegen").exists());

    let real_skill_names: Vec<String> = std::fs::read_dir(real_codex.join("skills"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(real_skill_names.contains(&".system".to_string()));
    assert!(real_skill_names.contains(&"my-own".to_string()));
    assert!(
        !real_skill_names.contains(&"nemonik".to_string()),
        "kit skill must not leak into the real codex home"
    );

    let reserved = vec![SkillSpec {
        name: ".system".to_string(),
        description: "d".to_string(),
        body: "EVIL".to_string(),
    }];
    assert!(
        materialize(&reserved, "nemonik-proj-tmpl-reserved", SkillLayout::Codex).is_err(),
        "a kit skill named .system must be rejected on the codex layout"
    );

    // A traversal skill name is refused before anything is written, Codex seeding included.
    let traversal = vec![
        SkillSpec {
            name: "ok".to_string(),
            description: "d".to_string(),
            body: "fine".to_string(),
        },
        SkillSpec {
            name: "../../canary".to_string(),
            description: "d".to_string(),
            body: "EVIL".to_string(),
        },
    ];
    let traversal_session = "nemonik-proj-tmpl-traversal";
    assert!(materialize(&traversal, traversal_session, SkillLayout::Codex).is_err());
    assert!(
        !session_base(traversal_session).unwrap().exists(),
        "a rejected kit must leave no session folder (no seeded codex home, no canary)"
    );

    cleanup(session).unwrap();
    assert!(
        !ch.exists(),
        "cleanup must remove the per-session codex home"
    );
    // remove_dir_all must not follow the symlinked `.system` directory into the real home.
    assert!(real_codex.join("auth.json").exists());
    assert!(real_codex.join("config.toml").exists());
    assert!(
        real_codex.join("skills/.system/imagegen").exists(),
        "the real .system built-ins must survive cleanup"
    );
    assert!(real_codex.join("skills/my-own").exists());
}
