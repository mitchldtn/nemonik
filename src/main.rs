mod assemble;
mod herdr;
mod launch;
mod remote;
mod review_view;
mod session;
mod trust;

use clap::{Parser, Subcommand};

/// Launch Claude Code or Codex with a kit: system prompt, skills, MCP servers,
/// hooks and workflow.
#[derive(Parser)]
#[command(name = "nemonik", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Log in to nemonik's server.
    Login,
    /// Set or print the trust policy: `strict` (default) reviews a new or changed
    /// kit before `run` starts it, `off` skips the review. Saved in
    /// ~/.nemonik/config.json.
    Trust {
        /// `strict` or `off`. Omit to print the current policy.
        mode: Option<String>,
    },
    /// Show a kit's exact content and pin its hash when you approve it.
    Verify {
        /// Full kit name (`owner/category:name`, e.g. `mitchldtn/code:review`), or a
        /// kit alias or local kit name from `nemonik.json`.
        kit: String,
        /// Also open the review, with its diff, in the browser.
        #[arg(long)]
        web: bool,
    },
    /// Start Claude Code or Codex in a tmux session with a kit loaded.
    Run {
        /// Full kit name (`owner/category:name`, e.g. `mitchldtn/code:review`), or a
        /// kit alias or local kit name from `nemonik.json`.
        kit: String,
        /// Project for `--remote` access. Defaults to `$NEMONIK_PROJECT`, then
        /// `nemonik.json`.
        #[arg(long)]
        project: Option<String>,
        /// Session name instead of a generated one: 1 to 32 characters, a lowercase
        /// letter first, then lowercase letters, digits, `_` or `-`.
        #[arg(long)]
        name: Option<String>,
        /// Agent to launch.
        #[arg(long, default_value = "claude")]
        agent: launch::Agent,
        /// Start the session in the background and print how to attach. In
        /// herdr, also open it in a background herdr tab.
        #[arg(long)]
        no_attach: bool,
        /// Open the session in a new terminal window, using `new_window_command`
        /// from ~/.nemonik/config.json. In herdr, open it in a new herdr tab
        /// instead.
        #[arg(long)]
        new: bool,
        /// Skip the agent's own permission prompts.
        #[arg(long)]
        yolo: bool,
        /// Let your account write to or watch the session from another device.
        /// Needs --project.
        #[arg(long)]
        remote: bool,
        /// Start a new or changed kit without review and without pinning, for
        /// this run only.
        #[arg(long)]
        yes: bool,
        /// Override the trust policy for this run only (`strict`/`off`).
        /// Precedence: this flag > NEMONIK_TRUST > ~/.nemonik/config.json > strict.
        #[arg(long)]
        trust: Option<String>,
        /// The agent's first task.
        #[arg(trailing_var_arg = true)]
        prompt: Vec<String>,
    },
    /// Remove a session's skills folder (`~/.nemonik/sessions/<session>/`). Runs
    /// on its own when a session closes; use it for a session that died first.
    CleanupSkills {
        /// Session name.
        session: String,
    },
    /// Attach this terminal to a session.
    Attach {
        /// Session name.
        session: String,
    },
    /// Paste text into a session's pane and submit it.
    Write {
        /// Session name.
        session: String,
        /// Text to send; it can be several words and start with `-`.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        text: Vec<String>,
    },
    /// Print a session's current pane text.
    Read {
        /// Session name.
        session: String,
        /// Scrollback lines to include above the visible pane.
        #[arg(long)]
        lines: Option<u64>,
    },
    /// Turn on remote control for a running session: one launched without
    /// `--remote`, or one whose `-daemon` session died.
    Remote {
        /// Session name.
        session: String,
        /// Project the session runs under; remote access is checked against it.
        #[arg(long)]
        project: String,
    },
    /// Watch your own session's live output.
    Watch {
        /// Session name.
        session: String,
        /// Project the session runs under; remote access is checked against it.
        #[arg(long)]
        project: String,
    },
    /// Internal: run by the tmux session to `exec` the agent from a launch spec.
    #[command(name = "__exec-agent", hide = true)]
    ExecAgent { spec: String },
    /// Internal: the remote-control daemon loop, run in its own tmux session.
    #[command(name = "__harness-daemon", hide = true)]
    HarnessDaemon {
        session: String,
        #[arg(long)]
        project: String,
    },
    /// List the example kits, or write one into a new folder. Examples are read
    /// from `examples/` on `main` of github.com/mitchldtn/nemonik. GITHUB_TOKEN
    /// is optional and raises GitHub's rate limit.
    Example {
        /// Example to write. Omit to list the examples.
        name: Option<String>,
        /// Folder to write it into: missing or empty. Defaults to `./<name>`.
        dir: Option<std::path::PathBuf>,
    },
}

/// Expands an alias for `verify`; no project is resolved, since trust pins are
/// project-independent.
fn expand_kit_alias(kit: String) -> anyhow::Result<String> {
    let (config, _) = nemonik_core::project_config::discover()?;
    // A local kit wins over an alias of the same name.
    if config.kits.contains_key(&kit) {
        eprintln!("nemonik: '{kit}' is a local kit (from nemonik.json)");
        return Ok(kit);
    }
    let expanded = nemonik_core::project_config::expand_alias(&kit, &config.aliases).to_string();
    if expanded != kit {
        eprintln!("nemonik: '{kit}' -> '{expanded}' (alias from nemonik.json)");
    }
    Ok(expanded)
}

/// Resolves the effective project and expands an alias for `run`. The project only
/// scopes `--remote`.
fn apply_project_config(
    flag_project: Option<String>,
    kit: String,
) -> anyhow::Result<(Option<String>, String)> {
    let (config, _) = nemonik_core::project_config::discover()?;
    let env_project = std::env::var("NEMONIK_PROJECT").ok();
    // A local kit wins over an alias, but the project is still resolved.
    if config.kits.contains_key(&kit) {
        eprintln!("nemonik: '{kit}' is a local kit (from nemonik.json)");
        let project = nemonik_core::project_config::resolve_project(
            flag_project.as_deref(),
            env_project.as_deref(),
            &config,
        );
        return Ok((project, kit));
    }
    let resolved = nemonik_core::project_config::resolve(
        flag_project.as_deref(),
        env_project.as_deref(),
        &kit,
        &config,
    );
    // The file may be untrusted: report on stderr whatever it changed.
    if resolved.project_from_config
        && let Some(project) = resolved.project.as_deref()
    {
        eprintln!("nemonik: project '{project}' (from nemonik.json)");
    }
    if resolved.expanded {
        eprintln!(
            "nemonik: '{kit}' -> '{}' (alias from nemonik.json)",
            resolved.kit
        );
    }
    Ok((resolved.project, resolved.kit))
}

async fn example(name: Option<String>, dir: Option<std::path::PathBuf>) -> anyhow::Result<()> {
    use nemonik_core::examples;
    let token = examples::token_from_env(std::env::var("GITHUB_TOKEN").ok());
    let all = examples::parse_archive(&examples::fetch_tarball(token.as_deref()).await?)?;
    let Some(name) = name else {
        let summaries = examples::summaries(&all);
        if summaries.is_empty() {
            eprintln!("nemonik: no examples are available");
        }
        let width = summaries.iter().map(|(n, _)| n.len()).max().unwrap_or(0);
        for (name, summary) in summaries {
            println!("{name:<width$}  {summary}");
        }
        return Ok(());
    };
    let dir = dir.unwrap_or_else(|| std::path::PathBuf::from(&name));
    let files = examples::select(all, &name)?;
    examples::install_example(&dir, &files)?;
    eprintln!(
        "nemonik: wrote example '{name}' to {}; start with its README.md",
        dir.display()
    );
    Ok(())
}

/// Resolves the deployment for the commands that talk to the server; read back via
/// `ApiConfig::current()`. The others never call it, so a bad `NEMONIK_API` can't stop them.
fn init_api() -> anyhow::Result<()> {
    let api = nemonik_core::config::ApiConfig::init()?;
    if !api.is_default() {
        eprintln!(
            "nemonik: using {} via NEMONIK_API (not the default nemonik.io)",
            api.domain()
        );
    }
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match Cli::parse().command {
        Command::Login => {
            init_api()?;
            nemonik_core::auth::login().await
        }
        Command::Trust { mode } => nemonik_core::config::trust_command(mode),
        Command::Verify { kit, web } => {
            init_api()?;
            let kit = expand_kit_alias(kit)?;
            trust::verify(&kit, web).await
        }
        Command::Run {
            kit,
            project,
            name,
            agent,
            no_attach,
            new,
            yolo,
            remote,
            yes,
            trust,
            prompt,
            ..
        } => {
            init_api()?;
            let (project, kit) = apply_project_config(project, kit)?;
            let prompt = (!prompt.is_empty()).then(|| prompt.join(" "));
            launch::run(
                project.as_deref(),
                &kit,
                agent,
                launch::RunFlags {
                    no_attach,
                    new,
                    yolo,
                    remote,
                    yes,
                    name,
                },
                prompt.as_deref(),
                trust.as_deref(),
            )
            .await
        }
        Command::CleanupSkills { session } => {
            // Removes the launch spec too, in case `__exec-agent` never consumed it.
            nemonik_core::launchspec::remove_launch_spec(&session);
            nemonik_core::skills::cleanup(&session)
        }
        Command::Example { name, dir } => example(name, dir).await,
        Command::Attach { session } => session::attach(&session),
        Command::Write { session, text } => session::write(&session, &text.join(" ")),
        Command::Read { session, lines } => session::read(&session, lines),
        Command::Remote { session, project } => {
            init_api()?;
            remote::spawn::daemon(&project, &session)
        }
        Command::Watch { session, project } => {
            init_api()?;
            remote::watch::watch(&project, &session).await
        }
        Command::ExecAgent { spec } => session::exec_agent(&spec),
        Command::HarnessDaemon { session, project } => {
            init_api()?;
            remote::daemon::run(&project, &session).await
        }
    }
}

/// Parses `nemonik <args>`, for tests that check argv the CLI builds for itself.
#[cfg(test)]
fn parse<S: AsRef<str>>(args: &[S]) -> Command {
    let argv = std::iter::once("nemonik").chain(args.iter().map(AsRef::as_ref));
    Cli::try_parse_from(argv).unwrap().command
}

#[cfg(test)]
mod tests {
    use super::Cli;

    fn run_prompt(args: &[&str]) -> Result<Vec<String>, clap::Error> {
        use clap::Parser;
        let argv = ["nemonik", "run", "mitchldtn/code:review"]
            .iter()
            .chain(args);
        match Cli::try_parse_from(argv)?.command {
            super::Command::Run { prompt, .. } => Ok(prompt),
            _ => unreachable!(),
        }
    }

    #[test]
    fn run_rejects_an_unknown_flag_after_the_kit() {
        assert!(run_prompt(&["--nwe", "--yolo", "do", "it"]).is_err());
    }

    #[test]
    fn run_prompt_keeps_flag_like_words_after_its_first_word() {
        let prompt = run_prompt(&["--new", "review", "the", "--no-attach", "flag"]).unwrap();
        assert_eq!(prompt, ["review", "the", "--no-attach", "flag"]);
    }

    #[test]
    fn run_prompt_can_start_with_a_dash_after_double_dash() {
        assert_eq!(run_prompt(&["--", "-x", "do"]).unwrap(), ["-x", "do"]);
    }

    #[test]
    fn write_takes_multi_word_text_that_may_start_with_a_dash() {
        match super::parse(&["write", "sess", "-x", "review", "--done"]) {
            super::Command::Write { session, text } => {
                assert_eq!(session, "sess");
                assert_eq!(text, ["-x", "review", "--done"]);
            }
            _ => unreachable!(),
        }
    }
}
