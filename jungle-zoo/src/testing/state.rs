//! Compact state fixtures for flow and scheduler tests.

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CounterState {
    pub value: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RaceState {
    pub fast_ms: u64,
    pub slow_ms: u64,
    pub winner: i32,
    pub joined_sum: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SleepCycleState {
    pub counter: i32,
    pub phase: u8,
    pub sleep_for_ms: u64,
}
