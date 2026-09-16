//! 入口：解析 CLI → 加载 → 执行 → 保存 → 打印；业务规则都在 `store`。

use anyhow::Result;
use clap::Parser;

mod cli;
mod store;
mod task;

use cli::{Cli, Commands, join_desc};
use store::{Scope, Store};
use task::Task;

fn main() {
    if let Err(err) = run() {
        // {:#} 打印完整原因链，默认只输出最外层。
        eprintln!("error: {err:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let mut store = Store::load()?;

    match cli.command {
        Commands::Add { words } => {
            let task = store.add(&join_desc(&words));
            store.save()?;
            println!("Added #{}: {}", task.id, task.desc);
        }
        Commands::Edit { id, words } => {
            let task = store.edit(id, &join_desc(&words))?;
            store.save()?;
            println!("Edited #{}: {}", task.id, task.desc);
        }
        Commands::List(args) => {
            let scope = if args.done {
                Scope::Done
            } else if args.all {
                Scope::All
            } else {
                Scope::Pending
            };
            print_tasks(&store.list(scope), scope);
        }
        Commands::Done { id } => {
            let task = store.set_done(id, true)?;
            store.save()?;
            println!("Completed #{}: {}", task.id, task.desc);
        }
        Commands::Undone { id } => {
            let task = store.set_done(id, false)?;
            store.save()?;
            println!("Reopened #{}: {}", task.id, task.desc);
        }
        Commands::Rm { id } => {
            let task = store.remove(id)?;
            store.save()?;
            println!("Removed #{}: {}", task.id, task.desc);
        }
    }
    Ok(())
}

/// 打印任务列表；空列表按作用域给出提示。
fn print_tasks(tasks: &[&Task], scope: Scope) {
    if tasks.is_empty() {
        let hint = match scope {
            Scope::Pending => "No pending tasks. Use --all to see completed tasks.",
            Scope::Done => "No completed tasks.",
            Scope::All => "No tasks yet. Use `mini-todo add <desc>` to create one.",
        };
        println!("{hint}");
        return;
    }
    for task in tasks {
        let mark = if task.done { 'x' } else { ' ' };
        if task.created_at.is_empty() {
            println!("{} [{}] {}", task.id, mark, task.desc);
        } else {
            println!("{} [{}] {} ({})", task.id, mark, task.desc, task.created_at);
        }
    }
}
