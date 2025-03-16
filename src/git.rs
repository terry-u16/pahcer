use anyhow::Result;
use rand::{
    distr::{Alphanumeric, SampleString},
    rng,
};
use std::process::{Command, Output};

/// 現在の変更をコミットした上でタグ付けし、タグ名を返す
pub(super) fn commit(tag_name: Option<String>) -> Result<String> {
    git_add_all()?;
    let has_diff = git_diff()?;

    if has_diff {
        git_commit()?;
    }

    let tag_name = generate_tag_name(tag_name);
    git_tag(&tag_name)?;

    if has_diff {
        git_reset()?;
    }

    Ok(tag_name)
}

/// pahcer関連のブランチを削除する
pub(super) fn prune() -> Result<()> {
    let branches = list_branches("pahcer/*")?;
    let current_branch = get_current_branch_name()?;

    for branch in branches.iter().filter(|b| **b != current_branch) {
        check_return_code(
            Command::new("git")
                .args(&["branch", "-D", branch])
                .output()?,
        )?;

        println!("Deleted branch: {}", branch);
    }

    Ok(())
}

/// 現在のブランチ名を取得する
fn get_current_branch_name() -> Result<String> {
    let current_branch_name = read_stdout(
        Command::new("git")
            .args(&["rev-parse", "--abbrev-ref", "HEAD"])
            .output()?,
    )?;

    Ok(current_branch_name)
}

/// タグ名を生成する
fn generate_tag_name(tag_name: Option<String>) -> String {
    // a-z, A-Z, 0-9の62種類の文字を使って8文字の文字列をランダムに生成する場合、
    // 衝突確率が0.01%以上にするためには20万回の試行が必要となり、十分に安全
    // 衝突しているか判定しても良いのだが、めんどくさいのでやらない
    const NAME_LENGTH: usize = 8;
    format!(
        "pahcer/{}",
        tag_name.unwrap_or_else(|| Alphanumeric.sample_string(&mut rng(), NAME_LENGTH))
    )
}

/// タグを生成する
fn git_tag(tag_name: &str) -> Result<()> {
    check_return_code(
        Command::new("git")
            .args(&[
                "tag",
                "-a",
                tag_name,
                "-m",
                "automatically generated by pahcer",
            ])
            .output()?,
    )
}

/// 直前のコミットを取り消す
fn git_reset() -> Result<()> {
    check_return_code(
        Command::new("git")
            .args(&["reset", "--mixed", "HEAD^"])
            .output()?,
    )
}

/// 全てのファイルをステージングする
fn git_add_all() -> Result<()> {
    check_return_code(Command::new("git").args(&["add", "--all"]).output()?)?;
    Ok(())
}

/// 変更があるかどうかを判定する
fn git_diff() -> Result<bool> {
    let diffs = read_stdout(
        Command::new("git")
            .args(&["diff", "--cached", "--name-only"])
            .output()?,
    )?;

    Ok(!diffs.is_empty())
}

/// 変更をコミットする
fn git_commit() -> Result<()> {
    check_return_code(
        Command::new("git")
            .args(&["commit", "-m", "automatically generated by pahcer"])
            .output()?,
    )
}

/// 指定されたブランチに移動する
fn switch_branch(current_branch_name: String) -> Result<(), anyhow::Error> {
    check_return_code(
        Command::new("git")
            .args(&["switch", &current_branch_name])
            .output()?,
    )
}

/// カレントブランチに変更を反映する
fn restore_changes(new_branch_name: &String) -> Result<(), anyhow::Error> {
    check_return_code(
        Command::new("git")
            .args(&["restore", "--source", new_branch_name, "--worktree", ":/"])
            .output()?,
    )
}

/// ブランチ名のリストを取得する
fn list_branches(pattern: &str) -> Result<Vec<String>, anyhow::Error> {
    let branches = read_stdout(
        Command::new("git")
            .args(&["branch", "--list", pattern, "--format='%(refname:short)'"])
            .output()?,
    )?;

    Ok(branches.lines().map(|s| s.to_string()).collect())
}

/// コマンドの実行結果を文字列として取得する
fn read_stdout(output: Output) -> Result<String> {
    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(anyhow::anyhow!(stderr))
    }
}

/// コマンドが正常終了したかどうかをチェックする
fn check_return_code(output: Output) -> Result<()> {
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(anyhow::anyhow!(stderr))
    }
}
