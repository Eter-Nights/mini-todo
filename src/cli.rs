//! CLI 定义：只负责把参数变成类型安全的指令，执行在 `main.rs`。

use clap::{Args, Parser, Subcommand};

/// A tiny command-line todo manager.
#[derive(Debug, Parser)]
#[command(name = "mini-todo", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// 新增一条任务（多个单词自动以空格拼接，无需引号）
    Add {
        /// 任务描述，例如：add buy some milk
        #[arg(required = true, num_args = 1..)]
        words: Vec<String>,
    },
    /// 修改未完成任务的描述（已完成或不存在会报错）
    Edit {
        /// 任务 ID
        id: u64,
        /// 新的任务描述
        #[arg(required = true, num_args = 1..)]
        words: Vec<String>,
    },
    /// 列出任务，默认只显示未完成
    List(ListArgs),
    /// 标记任务为已完成
    Done {
        /// 任务 ID
        id: u64,
    },
    /// 取消完成标记
    Undone {
        /// 任务 ID
        id: u64,
    },
    /// 删除任务（ID 不会被后续任务复用）
    Rm {
        /// 任务 ID
        id: u64,
    },
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// 显示全部任务（含已完成）
    #[arg(long, conflicts_with = "done")]
    pub all: bool,
    /// 只显示已完成任务
    #[arg(long)]
    pub done: bool,
}

/// 把 `add` 的多个单词拼成完整描述。
pub fn join_desc(words: &[String]) -> String {
    words.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn clap_layout_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn add_joins_multiple_words() {
        let cli = Cli::try_parse_from(["mini-todo", "add", "buy", "some", "milk"]).unwrap();

        let Commands::Add { words } = cli.command else {
            panic!("expected add");
        };
        assert_eq!(join_desc(&words), "buy some milk");
    }

    #[test]
    fn add_requires_at_least_one_word() {
        assert!(Cli::try_parse_from(["mini-todo", "add"]).is_err());
    }

    #[test]
    fn edit_parses_id_and_words() {
        let cli = Cli::try_parse_from(["mini-todo", "edit", "3", "fix", "typo"]).unwrap();

        let Commands::Edit { id, words } = cli.command else {
            panic!("expected edit");
        };
        assert_eq!(id, 3);
        assert_eq!(join_desc(&words), "fix typo");

        // edit 必须带 id 与至少一个词
        assert!(Cli::try_parse_from(["mini-todo", "edit"]).is_err());
        assert!(Cli::try_parse_from(["mini-todo", "edit", "3"]).is_err());
    }

    #[test]
    fn list_flags_are_mutually_exclusive() {
        assert!(Cli::try_parse_from(["mini-todo", "list", "--all", "--done"]).is_err());
        assert!(Cli::try_parse_from(["mini-todo", "list", "--all"]).is_ok());
        assert!(Cli::try_parse_from(["mini-todo", "list"]).is_ok());
    }

    #[test]
    fn id_commands_parse_number() {
        let cli = Cli::try_parse_from(["mini-todo", "done", "42"]).unwrap();
        assert!(matches!(cli.command, Commands::Done { id: 42 }));

        let cli = Cli::try_parse_from(["mini-todo", "rm", "7"]).unwrap();
        assert!(matches!(cli.command, Commands::Rm { id: 7 }));

        // 非数字应被 clap 拒绝
        assert!(Cli::try_parse_from(["mini-todo", "done", "abc"]).is_err());
    }
}
