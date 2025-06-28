use serde::{Deserialize, Serialize};

// src: https://github.com/os-checker/AtomicVChecker/blob/bec82556be9af937a6b3d30ddcc277f7a89f094f/section-5-detection/AtomVChecker/src/detector/report.rs#L34
// {
//   "AtomicCorrelationViolation": {
//     "bug_kind": "AtomicCorrelationViolation",
//     "possibility": "Possibly",
//     "diagnosis": {
//       "atomic": "src/main.rs:298:41: 298:54"
//     },
//     "explanation": "Using an atomic operation with a weaker memory ordering than necessary can lead to an inconsistent memory state. Using Acquire is sufficient to ensure the program's correctness."
//   }
// }
#[derive(Debug, Serialize, Deserialize)]
pub enum Report {
    AtomicCorrelationViolation(ReportContent<AtomicityViolationDiagnosis>),
}

impl Report {
    pub fn file_path(&self) -> String {
        match self {
            Report::AtomicCorrelationViolation(report) => report.diagnosis.atomic.clone(),
        }
    }

    pub fn kind_str(&self) -> &'static str {
        match self {
            Report::AtomicCorrelationViolation(_) => "AtomicCorrelationViolation",
        }
    }

    pub fn diag(&self) -> String {
        match self {
            Report::AtomicCorrelationViolation(a) => serde_json::to_string_pretty(a).unwrap(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AtomicityViolationDiagnosis {
    pub atomic: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReportContent<D> {
    pub bug_kind: String,
    pub possibility: String,
    pub diagnosis: D,
    pub explanation: String,
}
