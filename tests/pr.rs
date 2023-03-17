use eyre::{eyre, Result};
use rand::{distributions::Alphanumeric, Rng};
use std::process::{Command, Stdio};
use std::{env, fs};

fn str_rnd(count: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(count)
        .map(char::from)
        .collect()
}

fn exec(cmd: &str, args: Vec<&str>) -> Result<Vec<String>> {
    let command = Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    if command.status.success() {
        let output = String::from_utf8_lossy(&command.stdout).to_string();
        Ok(output
            .lines()
            .map(|s| s.to_string())
            .collect::<Vec<String>>())
    } else {
        Err(eyre!(String::from_utf8_lossy(&command.stderr).to_string()))
    }
}

fn test_pr_result(pr: Vec<String>, key: &str) -> Result<()> {
    let pr_branch = format!("{key}-pr");
    let pr_msg = format!("pr: {key}");
    // let commit_msg = format!("commit: {key}");

    let mut created_pr_lines = pr.iter();
    assert!(created_pr_lines.next().unwrap().contains(&pr_msg));
    created_pr_lines.next();
    assert!(created_pr_lines.next().unwrap().contains(&pr_branch));
    // TODO: message is not getting into
    // assert!(created_pr_lines.any(|line| line.contains(&commit_msg)));

    Ok(())
}

#[test]
fn test_pr() -> Result<()> {
    let key = str_rnd(12);

    let url = env::var("GR_REPOSITORY_URL")?;

    let base_dir = env::current_dir()?;
    let repositories_dir = base_dir.join("tests").join("repositories");
    let gr_path = base_dir.join("target/debug/gr");
    let gr = gr_path.to_str().unwrap();

    // Setup authentication and clone the repo
    env::set_current_dir(repositories_dir)?;
    exec("git", vec!["clone", &url, &key])?;
    env::set_current_dir(&key)?;

    // Checkout on new base branch, so we don't merge anything to any important
    let base_branch = format!("{key}-base");
    exec("git", vec!["checkout", "-b", &base_branch])?;
    exec("git", vec!["push", "-u", "origin", &base_branch])?;

    // Checkout to new child branch and create new commit
    let pr_branch = format!("{key}-pr");
    exec("git", vec!["checkout", "-b", &pr_branch])?;
    fs::File::create(&key)?;
    exec("git", vec!["add", "-A"])?;
    let commit_msg = format!("commit: {key}");
    exec("git", vec!["commit", "-m", &commit_msg])?;
    exec("git", vec!["push", "-u", "origin", &pr_branch])?;

    // Create the PR
    let pr_msg = format!("pr: {key}");
    let created_pr = exec(gr, vec!["pr", "create", "-m", &pr_msg])?;
    test_pr_result(created_pr, &key)?;

    // Get the PR
    let got_pr = exec(gr, vec!["pr", "get"])?;
    test_pr_result(got_pr, &key)?;

    // List the PRs
    // let listed_prs = exec(gr, vec!["pr", "list"])?;
    // listed_prs.iter().any(|pr| pr.contains(&commit_msg));

    // Approve the PR
    let approved_prs = exec(gr, vec!["pr", "approve"])?;
    test_pr_result(approved_prs, &key)?;

    // Close the PR
    let closed_pr = exec(gr, vec!["pr", "close"])?;
    test_pr_result(closed_pr, &key)?;

    // Reopen and merge the PR, test that we change the branch to the base branch
    exec(gr, vec!["pr", "create", "-m", &pr_msg, "-t", &base_branch])?;
    let merged_pr = exec(gr, vec!["pr", "merge"])?;
    test_pr_result(merged_pr, &key)?;
    let current_branch = exec("git", vec!["branch", "--show-current"])?;
    assert_eq!(current_branch.iter().next().unwrap().trim(), &base_branch);

    // Delete created repositories
    env::set_current_dir("..")?;
    fs::remove_dir_all(&key)?;
    env::set_current_dir(&base_dir)?;

    Ok(())
}

