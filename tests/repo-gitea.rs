use eyre::{eyre, Result};
use rand::{distributions::Alphanumeric, Rng};
use std::path::Path;
use std::process::{Command, Stdio};
use std::{env, fs};

fn str_rnd(count: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(count)
        .map(char::from)
        .collect()
}

fn exec(cmd: &str, args: Vec<&str>, inherit: bool) -> Result<Vec<String>> {
    let command = Command::new(cmd)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(if inherit {
            Stdio::inherit()
        } else {
            Stdio::piped()
        })
        .stderr(if inherit {
            Stdio::inherit()
        } else {
            Stdio::piped()
        })
        .output()?;

    if command.status.success() {
        let output = String::from_utf8_lossy(&command.stdout).to_string();
        Ok(output
            .lines()
            .map(|s| s.to_string())
            .collect::<Vec<String>>())
    } else {
        Err(eyre!(
            "{} {:?} {} {}",
            cmd,
            args,
            String::from_utf8_lossy(&command.stdout).to_string(),
            String::from_utf8_lossy(&command.stderr).to_string()
        ))
    }
}

fn test_repo_result(repo: Vec<String>, key: &str) -> Result<()> {
    let mut repo_lines = repo.iter();
    assert!(repo_lines.next().unwrap().contains(&key));
    assert!(repo_lines.next().unwrap().contains("0 stars, 0 forks"));
    repo_lines.next();
    assert!(repo_lines.next().unwrap().contains(&key));

    Ok(())
}

fn test_repo() -> Result<()> {
    let key = str_rnd(12);

    let forked_url = env::var("GR_GITEA_FORKED_REPOSITORY_URL")
        .unwrap_or("git@codeberg.org:gr-bin-test2/gr-bin-forked.git".to_string());

    let base_dir = env::current_dir()?;
    let repositories_dir = base_dir.join("tests").join("repositories");
    let gr_path = base_dir.join("target/debug/gr");
    let gr = gr_path.to_str().unwrap();

    // Create new repo and clone it
    let first_repo = format!("{key}-1");
    env::set_current_dir(repositories_dir)?;
    let repo = exec(
        gr,
        vec![
            "repo",
            "new",
            &first_repo,
            "--clone",
            "--host=codeberg.org",
            "--init",
            "--default-branch=develop",
            "--gitignore=Rust",
            "--license=MIT",
        ],
        false,
    )?;
    test_repo_result(repo, &first_repo)?;
    env::set_current_dir(&first_repo)?;

    // Test if the cloned repo is correct
    let branch = exec("git", vec!["rev-parse", "--abbrev-ref", "HEAD"], false)?;
    assert_eq!(branch[0], "develop");
    assert!(Path::new("README.md").exists());
    assert!(Path::new(".gitignore").exists());
    assert!(Path::new("LICENSE").exists());

    // Test if the repo exists
    let repo = exec(gr, vec!["repo", "get"], false)?;
    test_repo_result(repo, &first_repo)?;

    // Delete the repo, and test if it is actually deleted
    exec(
        gr,
        vec!["repo", "delete", "--yes-delete-permanently"],
        false,
    )?;
    let no_repo = exec(gr, vec!["repo", "get"], false);
    assert!(no_repo.is_err());

    // Fork repo and clone it
    let second_repo = format!("{key}-2");
    env::set_current_dir("..")?;
    let repo = exec(
        gr,
        vec!["repo", "fork", &forked_url, &second_repo, "--clone"],
        false,
    )?;
    test_repo_result(repo, &second_repo)?;
    env::set_current_dir(&second_repo)?;

    // Test if the forked repo exists
    let repo = exec(gr, vec!["repo", "get"], false)?;
    test_repo_result(repo, &second_repo)?;

    // Delete the repo, and test if it is actually deleted
    exec(
        gr,
        vec!["repo", "delete", "--yes-delete-permanently"],
        false,
    )?;
    let no_repo = exec(gr, vec!["repo", "get"], false);
    assert!(no_repo.is_err());

    // Delete created repositories
    env::set_current_dir("..")?;
    fs::remove_dir_all(&first_repo)?;
    fs::remove_dir_all(&second_repo)?;
    env::set_current_dir(&base_dir)?;

    Ok(())
}

#[test]
fn test_gitea_repo() {
    match test_repo() {
        Ok(()) => (),
        Err(err) => assert_eq!(err.to_string(), "^^^"),
    }
}
