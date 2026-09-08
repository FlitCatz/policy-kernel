#![forbid(unsafe_code)]

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};
use std::{env, fs, io, path::PathBuf, time::SystemTime};

fn now() -> String {
    DateTime::<Utc>::from(SystemTime::now()).to_rfc3339_opts(SecondsFormat::AutoSi, true)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env::var("FAKE_GH_ROOT")?);
    let args: Vec<String> = env::args().skip(1).collect();
    let route = args.iter().find(|s| s.starts_with("/repos/")).ok_or("missing route")?;
    let response = if route.ends_with("/actions/runs/9001") {
        fs::read_to_string(root.join("workflow.json"))?
    } else if route.ends_with("/pulls/6") {
        fs::read_to_string(root.join("pr.json"))?
    } else if route.ends_with("/check-runs") && args.iter().any(|s| s == "POST") {
        let mut data: Value = serde_json::from_reader(io::stdin().lock())?;
        if env::var("MERGE_DURING_POST").as_deref() == Ok("1") {
            let mut pr: Value = serde_json::from_slice(&fs::read(root.join("pr.json"))?)?;
            pr["state"] = json!("closed");
            pr["merged"] = json!(true);
            pr["merged_at"] = json!(now());
            pr["merge_commit_sha"] = json!("d".repeat(40));
            pr["merged_by"] = json!({"login":"release-owner","type":"User"});
            fs::write(root.join("pr.json"), serde_json::to_vec(&pr)?)?;
        }
        data["id"] = json!(6001);
        data["check_suite"] = json!({"id":8001});
        data["app"] = json!({"id":15368,"slug":"github-actions"});
        if data.get("started_at").is_none() { data["started_at"] = json!(now()); }
        for key in ["started_at", "completed_at"] {
            let timestamp = DateTime::parse_from_rfc3339(data[key].as_str().ok_or("missing timestamp")?)?;
            data[key] = json!(timestamp.to_rfc3339_opts(SecondsFormat::Secs, true));
        }
        let response = serde_json::to_string(&data)?;
        fs::write(root.join("published.json"), &response)?;
        response
    } else if route.ends_with("/check-runs/6001") {
        fs::read_to_string(root.join("published.json"))?
    } else {
        return Err(format!("unexpected route: {args:?}").into());
    };
    println!("{response}");
    Ok(())
}
