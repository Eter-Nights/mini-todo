//! 存储层：任务集合 `Store`、`todos.json` 读写与全部业务操作。
//!
//! - `add`/`set_done`/`remove`/`list` 只改内存，由调用方决定何时 `save`。
//! - 落盘为原子写：先写 `<name>.tmp` 再 `rename`，避免半截文件。
//! - 读取失败（JSON 损坏、ID 重复）只报错，绝不改写用户文件。
//!
//! `load`/`save` 用默认路径 `./todos.json`；`load_from`/`save_to` 供测试注入路径。

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::task::Task;

/// 默认数据文件名，相对于当前工作目录。
pub const DEFAULT_FILE: &str = "todos.json";

/// `list` 的筛选作用域。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Pending,
    Done,
    All,
}

/// 任务集合，即 `todos.json` 的顶层结构。
///
/// 字段私有，修改只能经带校验的方法，防止外部绕过 ID 规则。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Store {
    /// 下一个可分配的 ID，恒 > 所有现存任务 ID 且 >= 1。
    next_id: u64,
    /// 全部任务，含已完成的。
    tasks: Vec<Task>,
}

impl Store {
    /// 全新的空库：ID 从 1 开始。
    pub fn empty() -> Self {
        Self {
            next_id: 1,
            tasks: Vec::new(),
        }
    }

    /// 从默认路径 `./todos.json` 加载。
    pub fn load() -> Result<Self> {
        Self::load_from(Path::new(DEFAULT_FILE))
    }

    /// 从指定路径加载；文件不存在视为空库，不报错。
    pub fn load_from(path: &Path) -> Result<Self> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Self::empty()),
            Err(err) => {
                return Err(err).with_context(|| format!("failed to read {}", path.display()));
            }
        };

        let mut store: Store = serde_json::from_str(&text).with_context(|| {
            format!(
                "{} is not a valid todo file, fix or remove it before continuing",
                path.display()
            )
        })?;
        store
            .normalize()
            .with_context(|| format!("invalid task data in {}", path.display()))?;
        Ok(store)
    }

    /// 保存到默认路径。
    pub fn save(&self) -> Result<()> {
        self.save_to(Path::new(DEFAULT_FILE))
    }

    /// 原子保存：写同目录临时文件 → rename 覆盖。
    pub fn save_to(&self, path: &Path) -> Result<()> {
        let text = serde_json::to_string_pretty(self).context("failed to serialize tasks")? + "\n";
        let tmp = tmp_path(path);
        fs::write(&tmp, text).with_context(|| format!("failed to write {}", tmp.display()))?;
        fs::rename(&tmp, path).with_context(|| {
            format!(
                "failed to replace {} with {}",
                path.display(),
                tmp.display()
            )
        })
    }

    /// 新增任务并返回，ID 自增不回收。
    pub fn add(&mut self, desc: &str) -> Task {
        let task = Task::new(self.next_id, desc);
        self.next_id += 1;
        self.tasks.push(task.clone());
        task
    }

    /// 重写未完成任务的描述并返回。已完成的任务不可编辑。
    pub fn edit(&mut self, id: u64, desc: &str) -> Result<Task> {
        let task = self
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .with_context(|| format!("task {id} not found"))?;
        if task.done {
            bail!("task {id} is completed; run `undone {id}` first");
        }
        task.desc = desc.to_string();
        Ok(task.clone())
    }

    /// 设置完成状态并返回更新后的任务。
    pub fn set_done(&mut self, id: u64, done: bool) -> Result<Task> {
        let task = self
            .tasks
            .iter_mut()
            .find(|t| t.id == id)
            .with_context(|| format!("task {id} not found"))?;
        task.done = done;
        Ok(task.clone())
    }

    /// 删除任务，返回被删除的任务。
    pub fn remove(&mut self, id: u64) -> Result<Task> {
        let index = self
            .tasks
            .iter()
            .position(|t| t.id == id)
            .with_context(|| format!("task {id} not found"))?;
        Ok(self.tasks.remove(index))
    }

    /// 按作用域列出任务，按 ID 升序（不信任存储顺序，容忍手工重排）。
    pub fn list(&self, scope: Scope) -> Vec<&Task> {
        let mut selected: Vec<&Task> = self
            .tasks
            .iter()
            .filter(|t| match scope {
                Scope::Pending => !t.done,
                Scope::Done => t.done,
                Scope::All => true,
            })
            .collect();
        selected.sort_by_key(|t| t.id);
        selected
    }

    /// 修正手工编辑造成的 `next_id` 落后或为 0；ID 重复无法猜意图，直接报错。
    fn normalize(&mut self) -> Result<()> {
        let mut seen = HashSet::with_capacity(self.tasks.len());
        for task in &self.tasks {
            if !seen.insert(task.id) {
                bail!("duplicate task id {}", task.id);
            }
        }

        if let Some(max) = self.tasks.iter().map(|t| t.id).max() {
            self.next_id = self.next_id.max(max.saturating_add(1));
        }
        if self.next_id == 0 {
            self.next_id = 1;
        }
        Ok(())
    }
}

/// 与 `path` 同目录的临时写入路径：`todos.json` → `todos.json.tmp`。
fn tmp_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(DEFAULT_FILE);
    path.with_file_name(format!("{name}.tmp"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file_in(dir: &Path) -> PathBuf {
        dir.join(DEFAULT_FILE)
    }

    fn sample_store() -> Store {
        let mut store = Store::empty();
        store.add("one");
        store.add("two");
        store.add("three");
        store
    }

    /// 测试专用：按 ID 取描述，避免为断言给 Store 加公开 getter。
    fn desc_of(store: &Store, id: u64) -> Option<String> {
        store
            .list(Scope::All)
            .into_iter()
            .find(|t| t.id == id)
            .map(|t| t.desc.clone())
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::load_from(&file_in(dir.path())).unwrap();

        assert_eq!(store, Store::empty());
        assert!(!file_in(dir.path()).exists());
    }

    #[test]
    fn add_assigns_incrementing_ids_persisted_across_reload() {
        let dir = tempfile::tempdir().unwrap();
        let path = file_in(dir.path());

        let mut store = Store::empty();
        assert_eq!(store.add("a").id, 1);
        assert_eq!(store.add("b").id, 2);
        store.save_to(&path).unwrap();

        let mut reloaded = Store::load_from(&path).unwrap();
        assert_eq!(reloaded.add("c").id, 3);
    }

    #[test]
    fn removed_id_is_not_reused() {
        let dir = tempfile::tempdir().unwrap();
        let path = file_in(dir.path());

        let mut store = sample_store();
        store.remove(2).unwrap();
        store.save_to(&path).unwrap();

        let mut reloaded = Store::load_from(&path).unwrap();
        assert_eq!(reloaded.add("fourth").id, 4);
        let ids: Vec<u64> = reloaded.list(Scope::All).iter().map(|t| t.id).collect();
        assert_eq!(ids, vec![1, 3, 4]);
    }

    #[test]
    fn unknown_id_errors_for_mutating_methods() {
        let mut store = sample_store();

        let err = store.set_done(99, true).unwrap_err().to_string();
        assert_eq!(err, "task 99 not found");
        assert_eq!(store.remove(99).unwrap_err().to_string(), err);
        assert_eq!(store.edit(99, "x").unwrap_err().to_string(), err);
        // 失败不应破坏已有数据
        assert_eq!(store.list(Scope::All).len(), 3);
    }

    #[test]
    fn edit_replaces_desc_only_for_pending() {
        let mut store = sample_store();
        store.set_done(2, true).unwrap();

        let edited = store.edit(1, "renamed").unwrap();
        assert_eq!(edited.desc, "renamed");
        assert!(!edited.done);
        // ID 与创建时间不受影响，仅描述被替换
        assert_eq!(desc_of(&store, 1).as_deref(), Some("renamed"));

        let err = store.edit(2, "nope").unwrap_err().to_string();
        assert_eq!(err, "task 2 is completed; run `undone 2` first");
        assert_eq!(desc_of(&store, 2).as_deref(), Some("two"));
    }

    #[test]
    fn save_leaves_no_temp_file_behind() {
        let dir = tempfile::tempdir().unwrap();
        let path = file_in(dir.path());

        sample_store().save_to(&path).unwrap();

        assert!(path.exists());
        assert!(!tmp_path(&path).exists());
    }

    #[test]
    fn corrupt_file_errors_without_touching_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = file_in(dir.path());
        fs::write(&path, "not json at all").unwrap();

        let chain = format!("{:#}", Store::load_from(&path).unwrap_err());
        assert!(chain.contains("not a valid todo file"), "got: {chain}");
        assert_eq!(fs::read_to_string(&path).unwrap(), "not json at all");
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = file_in(dir.path());
        fs::write(
            &path,
            r#"{"next_id":9,"tasks":[{"id":1,"desc":"a"},{"id":1,"desc":"b"}]}"#,
        )
        .unwrap();

        let err = Store::load_from(&path).unwrap_err();
        // normalize 的错误被 with_context 包了一层，用 {:#} 检查完整错误链
        let chain = format!("{err:#}");
        assert!(chain.contains("duplicate task id 1"), "got: {chain}");
    }

    #[test]
    fn stale_next_id_is_repaired() {
        let dir = tempfile::tempdir().unwrap();
        let path = file_in(dir.path());
        // 手工编辑：next_id 落后于最大 ID
        fs::write(
            &path,
            r#"{"next_id":2,"tasks":[{"id":7,"desc":"hand added"}]}"#,
        )
        .unwrap();

        let mut store = Store::load_from(&path).unwrap();
        assert_eq!(store.add("new").id, 8);
    }

    #[test]
    fn list_filters_by_scope_sorted_by_id() {
        let mut store = sample_store();
        store.set_done(3, true).unwrap();
        store.set_done(1, true).unwrap();

        let pending: Vec<u64> = store.list(Scope::Pending).iter().map(|t| t.id).collect();
        let done: Vec<u64> = store.list(Scope::Done).iter().map(|t| t.id).collect();
        let all: Vec<u64> = store.list(Scope::All).iter().map(|t| t.id).collect();

        assert_eq!(pending, vec![2]);
        assert_eq!(done, vec![1, 3]);
        assert_eq!(all, vec![1, 2, 3]);
    }

    #[test]
    fn tmp_path_stays_beside_target() {
        assert_eq!(
            tmp_path(Path::new("todos.json")),
            PathBuf::from("todos.json.tmp")
        );
        assert_eq!(
            tmp_path(Path::new("/data/todos.json")),
            PathBuf::from("/data/todos.json.tmp")
        );
    }
}
