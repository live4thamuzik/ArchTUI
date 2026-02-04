//! Engine modules — the "brain" that translates user config into script sequences.
//!
//! The engine layer sits between configuration (what the user wants) and execution
//! (which scripts to run). It generates ordered, validated operation plans.

pub mod storage;
