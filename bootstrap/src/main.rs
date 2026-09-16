#[cfg(unix)]
mod cli {
    use clap::{Args, Parser, Subcommand};
    use policy_bootstrap::{Installation, Operation, Plan, Request, UserProfile};
    use std::{collections::BTreeMap, path::PathBuf};

    #[derive(Parser)]
    #[command(
        about = "Install pinned local verification configuration; no policy authority",
        version
    )]
    pub struct Cli {
        #[command(subcommand)]
        command: Command,
    }

    #[derive(Subcommand)]
    enum Command {
        /// Read-only project inspection with file-level additions and conflicts.
        Inspect(Selection),
        /// Emit a complete JSON plan to stdout. Save it outside the target directory.
        Plan(Selection),
        /// Revalidate and atomically stage/publish a saved plan, with rollback journal.
        Apply {
            #[arg(long)]
            plan: PathBuf,
        },
        /// Restore an interrupted transaction or finish committed transaction cleanup.
        Recover {
            #[arg(long)]
            target: PathBuf,
        },
    }

    #[derive(Args)]
    struct Selection {
        #[arg(long)]
        target: PathBuf,
        #[arg(long, value_parser = ["react-typescript", "dotnet", "fullstack"], required_unless_present = "profile_file", conflicts_with = "profile_file")]
        profile: Option<String>,
        /// Existing checkout-relative user-owned profile; release 1.1.0 only.
        #[arg(long, conflicts_with = "components")]
        profile_file: Option<String>,
        #[arg(long)]
        release: String,
        /// Repeat for frontend=apps/web, backend=apps/api, shared=tests/shared.
        #[arg(long = "component", value_name = "ID=PATH")]
        components: Vec<String>,
        #[arg(long)]
        uninstall: bool,
    }

    impl Selection {
        fn request(self) -> Result<Request, Box<dyn std::error::Error>> {
            let mut components = BTreeMap::new();
            for value in self.components {
                let (id, path) = value.split_once('=').ok_or("component must be ID=PATH")?;
                if components.insert(id.into(), path.into()).is_some() {
                    return Err("duplicate component selection".into());
                }
            }
            Ok(Request {
                target: self.target,
                installation: Installation {
                    profile: self.profile.unwrap_or_default(),
                    release: self.release,
                    components,
                    user_profile: self.profile_file.map(|path| UserProfile {
                        path,
                        digest: String::new(),
                    }),
                },
                operation: if self.uninstall {
                    Operation::Uninstall
                } else {
                    Operation::Install
                },
            })
        }
    }

    pub fn execute() -> Result<bool, Box<dyn std::error::Error>> {
        match Cli::parse().command {
            Command::Inspect(selection) | Command::Plan(selection) => {
                let plan = policy_bootstrap::plan(selection.request()?)?;
                println!("{}", serde_json::to_string_pretty(&plan)?);
                Ok(plan.conflicts.is_empty())
            }
            Command::Apply { plan } => {
                use std::io::Read;
                let mut bytes = Vec::new();
                std::fs::File::open(plan)?
                    .take(16 * 1024 * 1024 + 1)
                    .read_to_end(&mut bytes)?;
                if bytes.len() > 16 * 1024 * 1024 {
                    return Err("plan exceeds 16 MiB".into());
                }
                let plan: Plan = serde_json::from_slice(&bytes)?;
                policy_bootstrap::apply(&plan)?;
                println!("{{\"status\":\"applied\"}}");
                Ok(true)
            }
            Command::Recover { target } => {
                policy_bootstrap::recover(&target)?;
                println!("{{\"status\":\"recovered\"}}");
                Ok(true)
            }
        }
    }
}

fn main() -> std::process::ExitCode {
    #[cfg(unix)]
    match cli::execute() {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::from(2),
        Err(error) => {
            eprintln!("policy-bootstrap: {error}");
            std::process::ExitCode::from(64)
        }
    }
    #[cfg(not(unix))]
    {
        eprintln!("policy-bootstrap: descriptor-bounded installation requires Unix");
        std::process::ExitCode::from(64)
    }
}
