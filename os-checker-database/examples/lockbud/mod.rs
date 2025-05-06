use super::RE;
use serde::{Deserialize, Serialize};
use serde_json::Value;

//  [
//       {
//         "AtomicityViolation": {
//           "bug_kind": "AtomicityViolation",
//           "possibility": "Possibly",
//           "diagnosis": {
//             "fn_name": "imp::atomic128::x86_64::detect::detect",
//             "atomic_reader": "/home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/portable-atomic-1.11.0/src/imp/atomic128/../de
// tect/common.rs:32:28: 32:57 (#0)",
//             "atomic_writer": "/home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/portable-atomic-1.11.0/src/imp/atomic128/../de
// tect/common.rs:43:5: 43:43 (#0)",
//             "dep_kind": "Control"
//           },
//           "explanation": "atomic::store is data/control dependent on atomic::load"
//         }
//       }
//     ]
//
//  [
//       {
//         "UseAfterFree": {
//           "bug_kind": "UseAfterFree",
//           "possibility": "Possibly",
//           "diagnosis": "Escape to Param/Return: Raw ptr _0 at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/nix-0.26.4/src
// /pty.rs:209:37: 209:51 (#0) escapes to [_0] but pointee is dropped at /home/runner/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/nix-0.
// 26.4/src/pty.rs:222:1: 222:2 (#0)",
//           "explanation": "Raw ptr is used or escapes the current function after the pointed value is dropped"
//         }
//       }
//     ]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Lockbud {
    pub bug_kind: BugKind,
    pub possibility: Possibility,
    pub diagnosis: serde_json::Value,
    pub explanation: String,
}

impl Lockbud {
    pub fn file_paths(&self) -> Vec<String> {
        // parse: https://github.com/BurtonQin/lockbud/blob/2ebd731f7775db1b0a3b588ba2d8aa37b393bfe7/src/detector/report.rs#L34
        //
        // ```
        // pub enum Report {
        //     DoubleLock(ReportContent<DeadlockDiagnosis>),
        //     ConflictLock(ReportContent<Vec<DeadlockDiagnosis>>),
        //     CondvarDeadlock(ReportContent<CondvarDeadlockDiagnosis>),
        //     AtomicityViolation(ReportContent<AtomicityViolationDiagnosis>),
        //     InvalidFree(ReportContent<String>),
        //     UseAfterFree(ReportContent<String>),
        // }
        // ```
        let val = &self.diagnosis;
        match self.bug_kind {
            BugKind::DoubleLock => DeadlockDiagnosis::double_lock(val).1,
            BugKind::ConflictLock => DeadlockDiagnosis::conflict_lock(val).1,
            BugKind::CondvarDeadlock => CondvarDeadlockDiagnosis::new(val).1,
            BugKind::AtomicityViolation => AtomicityViolationDiagnosis::new(val).1,
            BugKind::InvalidFree | BugKind::UseAfterFree => {
                RE.parse_file_paths(val.as_str().unwrap())
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum BugKind {
    DoubleLock,
    ConflictLock,
    CondvarDeadlock,
    AtomicityViolation,
    InvalidFree,
    UseAfterFree,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Possibility {
    Probably,
    Possibly,
}

// https://github.com/BurtonQin/lockbud/blob/2ebd731f7775db1b0a3b588ba2d8aa37b393bfe7/src/detector/atomic/mod.rs#L151
#[derive(Debug, Serialize, Deserialize)]
pub struct AtomicityViolationDiagnosis {
    pub fn_name: String,
    pub atomic_reader: String,
    pub atomic_writer: String,
    pub dep_kind: String,
}

impl AtomicityViolationDiagnosis {
    fn new(val: &Value) -> (Self, Vec<String>) {
        let this = Self::deserialize(val).unwrap();
        let mut files = Vec::new();
        files.extend(RE.parse_file_paths(&this.atomic_reader));
        files.extend(RE.parse_file_paths(&this.atomic_writer));
        (this, files)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeadlockDiagnosis {
    pub first_lock_type: String,
    pub first_lock_span: String,
    pub second_lock_type: String,
    pub second_lock_span: String,
    pub callchains: Vec<Vec<Vec<String>>>,
}

impl DeadlockDiagnosis {
    fn double_lock(val: &Value) -> (Self, Vec<String>) {
        let this = Self::deserialize(val).unwrap();
        let mut files = Vec::new();
        files.extend(RE.parse_file_paths(&this.first_lock_span));
        files.extend(RE.parse_file_paths(&this.second_lock_span));
        (this, files)
    }

    fn conflict_lock(val: &Value) -> (Vec<Self>, Vec<String>) {
        let v = Vec::<Self>::deserialize(val).unwrap();
        let mut files = Vec::new();
        for this in &v {
            files.extend(RE.parse_file_paths(&this.first_lock_span));
            files.extend(RE.parse_file_paths(&this.second_lock_span));
        }
        (v, files)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CondvarDeadlockDiagnosis {
    pub condvar_wait_type: String,
    pub condvar_wait_callsite_span: String,
    pub condvar_notify_type: String,
    pub condvar_notify_callsite_span: String,
    pub deadlocks: Vec<WaitNotifyLocks>,
}

impl CondvarDeadlockDiagnosis {
    fn new(val: &Value) -> (Self, Vec<String>) {
        let this = Self::deserialize(val).unwrap();
        let mut files = Vec::new();
        files.extend(RE.parse_file_paths(&this.condvar_wait_callsite_span));
        files.extend(RE.parse_file_paths(&this.condvar_notify_callsite_span));
        for deadlock in &this.deadlocks {
            files.extend(RE.parse_file_paths(&deadlock.wait_lock_span));
            files.extend(RE.parse_file_paths(&deadlock.notify_lock_span));
        }
        (this, files)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WaitNotifyLocks {
    pub wait_lock_type: String,
    pub wait_lock_span: String,
    pub notify_lock_type: String,
    pub notify_lock_span: String,
}
