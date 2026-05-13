//! Viewer-side autosave plumbing (A3.1).
//!
//! Owns an [`AutosavePolicy`] and the tick-time bookkeeping (last save
//! instant + last canonical hash) needed to skip redundant writes.
//! The actual snapshot writer lives on [`cadkernel_api::Session`]
//! (`write_autosave_snapshot`); this module is the viewer-side caller.
//!
//! A3.1 ships plumbing only. The viewer currently holds an unused
//! [`cadkernel_api::Session`] that is not driven by `GuiAction`
//! dispatch, so `session.canonical_hash()` does not move when the user
//! edits geometry. Autosave will fire once on the first eligible tick
//! and then stop until the canonical hash actually changes — which
//! becomes real once a future patch rewires `GuiAction` dispatch
//! through `Session::execute`.

use cadkernel_api::cadk::AutosavePolicy;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Default location for autosave snapshots: a stable per-machine
/// scratch directory under the OS temp dir. We deliberately avoid the
/// `dirs` crate dependency — `std::env::temp_dir()` is one syscall and
/// behaves identically across CI and dev machines.
pub(crate) fn default_dir() -> PathBuf {
    std::env::temp_dir().join("cadkernel/autosave")
}

/// Tick-time bookkeeping wrapped around an [`AutosavePolicy`].
///
/// One instance lives on [`crate::app::CadApp`]. The interval check
/// runs every `process_actions` cycle but produces at most one write
/// per `policy.interval`, gated on canonical-hash change so a static
/// scene does not re-save the same blob.
pub(crate) struct AutosaveState {
    pub policy: AutosavePolicy,
    pub last_save: Option<Instant>,
    pub last_hash: Option<u64>,
    /// User-facing on/off, mirrored from `NavConfig::auto_save_enabled`
    /// at tick time. When `false`, `should_fire` returns `false`
    /// regardless of interval/hash state.
    pub enabled: bool,
}

impl AutosaveState {
    pub(crate) fn new() -> Self {
        Self {
            policy: AutosavePolicy {
                dir: default_dir(),
                interval: Duration::from_secs(30),
                retain: 5,
                compress: true,
            },
            last_save: None,
            last_hash: None,
            enabled: false,
        }
    }

    /// Returns `true` when the tick loop should write a new snapshot:
    /// autosave is enabled, the policy interval has elapsed since the
    /// last save (or no save has happened yet), and the canonical hash
    /// has moved since the last successful save.
    pub(crate) fn should_fire(&self, now: Instant, hash: u64) -> bool {
        if !self.enabled {
            return false;
        }
        let interval_ok = self
            .last_save
            .is_none_or(|t| now.duration_since(t) >= self.policy.interval);
        let hash_changed = self.last_hash.is_none_or(|h| h != hash);
        interval_ok && hash_changed
    }

    pub(crate) fn record_save(&mut self, now: Instant, hash: u64) {
        self.last_save = Some(now);
        self.last_hash = Some(hash);
    }

    /// Sync user-facing knobs from `NavConfig` (UI source of truth)
    /// into the policy. Called at tick time so changes from the
    /// Settings dialog take effect immediately.
    pub(crate) fn sync_from_nav(&mut self, enabled: bool, interval_secs: u32) {
        self.enabled = enabled;
        let secs = interval_secs.max(1) as u64;
        self.policy.interval = Duration::from_secs(secs);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disabled_never_fires() {
        let st = AutosaveState::new();
        assert!(!st.should_fire(Instant::now(), 0));
    }

    #[test]
    fn first_eligible_tick_fires_when_enabled() {
        let mut st = AutosaveState::new();
        st.enabled = true;
        assert!(st.should_fire(Instant::now(), 42));
    }

    #[test]
    fn same_hash_does_not_refire() {
        let mut st = AutosaveState::new();
        st.enabled = true;
        let t0 = Instant::now();
        st.record_save(t0, 42);
        let t1 = t0 + Duration::from_secs(60);
        assert!(!st.should_fire(t1, 42));
    }

    #[test]
    fn changed_hash_refires_after_interval() {
        let mut st = AutosaveState::new();
        st.enabled = true;
        let t0 = Instant::now();
        st.record_save(t0, 42);
        let t1 = t0 + Duration::from_secs(60);
        assert!(st.should_fire(t1, 99));
    }

    #[test]
    fn interval_gate_blocks_before_elapsed() {
        let mut st = AutosaveState::new();
        st.enabled = true;
        st.policy.interval = Duration::from_secs(30);
        let t0 = Instant::now();
        st.record_save(t0, 42);
        let t1 = t0 + Duration::from_secs(5);
        assert!(!st.should_fire(t1, 99));
    }

    #[test]
    fn sync_from_nav_clamps_zero_interval() {
        let mut st = AutosaveState::new();
        st.sync_from_nav(true, 0);
        assert_eq!(st.policy.interval, Duration::from_secs(1));
        assert!(st.enabled);
    }
}
