//! 되돌리기 기록.
//!
//! 기록 하나는 [`UndoEntry`] trait을 구현한다. 기본 구현은 편집 전후 뼈대 전체를 보관하는 [`SnapshotEntry`]다.
//! (뼈대 데이터는 작으므로 통째 보관이 가장 단순하고 틀릴 일이 없다.)
//! 드래그처럼 여러 명령이 이어지는 편집은 `begin_group` ~ `end_group`으로 묶어 되돌리기 한 번에 되돌린다.
use crate::model::{Project, Skeleton};

/// 되돌릴 수 있는 기록 하나.
pub trait UndoEntry {
    /// 화면 표시용 이름(예: "본 변환").
    fn label(&self) -> &str;
    fn undo(&self, project: &mut Project);
    fn redo(&self, project: &mut Project);
}

/// 편집 전후 상태(뼈대 + 이미지 경로)를 통째로 보관하는 기록.
#[derive(Clone, Debug)]
pub struct SnapshotEntry {
    label: String,
    before: State,
    after: State,
}

/// 기록 대상 상태.
#[derive(Clone, Debug, PartialEq)]
pub struct State {
    pub skeleton: Skeleton,
    pub images_path: String,
}

impl State {
    pub fn of(p: &Project) -> State {
        State { skeleton: p.skeleton.clone(), images_path: p.images_path.clone() }
    }
    fn restore(&self, p: &mut Project) {
        p.skeleton = self.skeleton.clone();
        p.images_path = self.images_path.clone();
    }
}

impl UndoEntry for SnapshotEntry {
    fn label(&self) -> &str {
        &self.label
    }
    fn undo(&self, project: &mut Project) {
        self.before.restore(project);
    }
    fn redo(&self, project: &mut Project) {
        self.after.restore(project);
    }
}

struct Group {
    label: String,
    before: State,
    depth: u32,
}

/// 되돌리기·다시 실행 목록.
pub struct History {
    undo: Vec<Box<dyn UndoEntry>>,
    redo: Vec<Box<dyn UndoEntry>>,
    group: Option<Group>,
    limit: usize,
}

impl Default for History {
    fn default() -> Self {
        History { undo: vec![], redo: vec![], group: None, limit: 500 }
    }
}

impl History {
    pub fn new() -> Self {
        History::default()
    }

    /// 명령 하나가 끝난 뒤 호출. 묶음 안이면 묶음이 끝날 때 한 번에 기록한다. 바뀐 것이 없으면 기록하지 않는다.
    pub fn record(&mut self, label: &str, before: State, project: &Project) {
        if self.group.is_some() {
            return;
        }
        let after = State::of(project);
        if before != after {
            self.push(Box::new(SnapshotEntry { label: label.into(), before, after }));
        }
    }

    fn push(&mut self, e: Box<dyn UndoEntry>) {
        self.undo.push(e);
        if self.undo.len() > self.limit {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// 묶음 시작. 겹쳐 부르면 가장 바깥 묶음 하나로 합쳐진다.
    pub fn begin_group(&mut self, label: &str, project: &Project) {
        match &mut self.group {
            Some(g) => g.depth += 1,
            None => self.group = Some(Group { label: label.into(), before: State::of(project), depth: 1 }),
        }
    }

    /// 묶음 끝. 가장 바깥 묶음이 끝나면 기록 하나를 남긴다.
    pub fn end_group(&mut self, project: &Project) {
        let Some(g) = &mut self.group else { return };
        g.depth -= 1;
        if g.depth == 0 {
            let g = self.group.take().expect("묶음");
            self.record(&g.label, g.before, project);
        }
    }

    pub fn in_group(&self) -> bool {
        self.group.is_some()
    }

    /// 되돌린 기록의 이름을 돌려준다.
    pub fn undo(&mut self, project: &mut Project) -> Option<String> {
        if self.group.is_some() {
            self.end_group_all(project);
        }
        let e = self.undo.pop()?;
        e.undo(project);
        let label = e.label().to_string();
        self.redo.push(e);
        Some(label)
    }

    pub fn redo(&mut self, project: &mut Project) -> Option<String> {
        let e = self.redo.pop()?;
        e.redo(project);
        let label = e.label().to_string();
        self.undo.push(e);
        Some(label)
    }

    fn end_group_all(&mut self, project: &Project) {
        if let Some(g) = &mut self.group {
            g.depth = 1;
        }
        self.end_group(project);
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn undo_label(&self) -> Option<&str> {
        self.undo.last().map(|e| e.label())
    }
    pub fn redo_label(&self) -> Option<&str> {
        self.redo.last().map(|e| e.label())
    }
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.group = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Bone;

    #[test]
    fn f_i03_group_is_one_undo() {
        let mut p = Project::new();
        let mut h = History::new();
        h.begin_group("드래그", &p);
        for i in 0..5 {
            let before = State::of(&p);
            p.skeleton.bones[0].x = i as f32;
            h.record("이동", before, &p);
        }
        h.end_group(&p);
        assert_eq!(p.skeleton.bones[0].x, 4.0);
        assert_eq!(h.undo(&mut p).as_deref(), Some("드래그"));
        assert_eq!(p.skeleton.bones[0].x, 0.0);
        assert!(!h.can_undo());
        h.redo(&mut p);
        assert_eq!(p.skeleton.bones[0].x, 4.0);
    }

    #[test]
    fn unchanged_is_not_recorded() {
        let mut p = Project::new();
        let mut h = History::new();
        let before = State::of(&p);
        h.record("없음", before, &p);
        assert!(!h.can_undo());
        let before = State::of(&p);
        p.skeleton.bones.push(Bone::new("a", Some("root".into())));
        h.record("본 생성", before, &p);
        assert!(h.can_undo());
    }
}
