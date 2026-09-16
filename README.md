# mini-todo

一个简单的命令行 Todo 管理器，使用 Rust 编写，任务数据保存在当前目录的 `todos.json`。

## 构建

```bash
cargo build --release
```

## 使用

```bash
# 新增任务
cargo run -- add buy some milk

# 查看未完成任务
cargo run -- list

# 查看全部任务或仅已完成任务
cargo run -- list --all
cargo run -- list --done

# 修改、完成、恢复和删除任务
cargo run -- edit 1 buy oat milk
cargo run -- done 1
cargo run -- undone 1
cargo run -- rm 1
```

任务 ID 会自动递增，删除任务后不会复用旧 ID。运行帮助可以查看完整命令说明：

```bash
cargo run -- --help
```

## 测试

```bash
cargo test
```
