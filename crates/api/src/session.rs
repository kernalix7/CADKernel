//! Session — the execute/replay engine.
//!
//! A [`Session`] owns one [`Document`] and applies [`Command`]s to it. Every
//! successful command is appended to a log so that:
//!
//! - The log can be serialized to JSON and replayed elsewhere
//!   (regression tests, AI evaluation harnesses, "save the steps that built
//!   this part" workflows).
//! - The log can be inspected for audit / debugging.
//! - Undo / redo walk a cursor over the log, rebuilding the document via
//!   replay (Phase 2A: replay-based; Phase 7 may add lazy snapshots).
//!
//! The whole session can be saved as a [`SessionSnapshot`] (log + cursor)
//! to JSON and reloaded; this is the foundation on which Phase 3's `.cadk`
//! native format will be built.

use cadkernel_math::{Point3, Quaternion, Vec3};
use cadkernel_modeling::measure::solid_mass_properties;
use cadkernel_modeling::primitives::make_cone;
use cadkernel_modeling::quick::{
    quick_box, quick_cone, quick_cylinder, quick_intersect, quick_sphere, quick_subtract,
    quick_torus, quick_union,
};
use cadkernel_modeling::{extrude, mirror_solid};
use cadkernel_topology::{BRepModel, Handle, SolidData};
use serde::{Deserialize, Serialize};

use crate::command::{Command, ExtrudeKind, InstanceOverride};
use crate::document::{Document, FeatureId, HistoryEvent, SolidId, SolidSlot};
use crate::outcome::Outcome;
use crate::{ApiError, ApiResult};

/// The execute/replay engine.
///
/// See the crate-level docs for the high-level contract. A `Session` is
/// `Send` but not `Sync` — concurrent mutation is not part of the API
/// contract.
///
/// Internal invariant: `log[..cursor]` has been applied to `document`;
/// `log[cursor..]` is the redo stack (entries that were undone but not
/// overwritten).
pub struct Session {
    document: Document,
    log: Vec<Command>,
    cursor: usize,
    /// Wall-clock instant when the most recent command was executed.
    /// Used to coalesce rapid-fire property edits (Translate / Scale /
    /// Rename) into a single log entry within `coalesce_window_ms`.
    /// Not serialized — coalescing is a runtime-only interactive feature.
    last_command_at: Option<std::time::Instant>,
    /// Window during which a same-kind same-target command will be
    /// folded into the previous log entry instead of producing a new
    /// HistoryEvent. 0 disables coalescing entirely.
    coalesce_window_ms: u64,
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// On-disk session snapshot. Round-trips with [`Session::save_to_json`] and
/// [`Session::load_from_json`]. Captures both the log and the cursor so
/// reload preserves the redo stack.
///
/// `document_hash`, `log_position`, `timestamp`, and `label` are A2-spec
/// metadata fields. They are serialized with `#[serde(default)]` so older
/// schema-v1 snapshots that lack them still load cleanly.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionSnapshot {
    /// Schema version. Bump in lockstep with breaking [`Command`] changes.
    pub schema_version: u32,
    pub commands: Vec<Command>,
    pub cursor: usize,
    /// FNV-1a hash of the serialized command log up through `cursor`. Used
    /// by tooling to detect divergence without replaying the whole log.
    #[serde(default)]
    pub document_hash: String,
    /// Mirror of `cursor` — included as a separate field so external
    /// tools can detect snapshot identity without parsing `commands`.
    #[serde(default)]
    pub log_position: usize,
    /// Unix epoch seconds at save time (best-effort; 0 if unavailable).
    #[serde(default)]
    pub timestamp: i64,
    /// Optional human label (e.g. "before boolean", "release-v0.5-tag").
    #[serde(default)]
    pub label: Option<String>,
}

impl SessionSnapshot {
    pub const CURRENT_SCHEMA: u32 = 1;
}

/// Stable FNV-1a-64 hex digest of the serialized command prefix.
/// Cheap, deterministic, no extra dep — sufficient for snapshot identity.
fn hash_command_prefix(commands: &[Command]) -> String {
    // Serialize to canonical JSON; ignore errors (Command is always
    // serializable) and fall back to empty string on the impossible case.
    let json = serde_json::to_string(commands).unwrap_or_default();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in json.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    format!("{h:016x}")
}

fn now_unix_seconds() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl Session {
    /// Creates a fresh session with an empty document and empty log.
    /// Coalesce window defaults to 1 000 ms per A2 spec.
    pub fn new() -> Self {
        Self {
            document: Document::default(),
            log: Vec::new(),
            cursor: 0,
            last_command_at: None,
            coalesce_window_ms: 1000,
        }
    }

    /// Sets the coalesce window in milliseconds. `0` disables coalescing.
    /// Consecutive `Translate` / `Scale` / `Rename` commands targeting the
    /// same solid id within this window are folded into the previous log
    /// entry instead of producing a new history record.
    pub fn set_coalesce_window_ms(&mut self, ms: u64) {
        self.coalesce_window_ms = ms;
    }

    /// Returns the current coalesce window in milliseconds.
    pub fn coalesce_window_ms(&self) -> u64 {
        self.coalesce_window_ms
    }

    /// Returns a reference to the underlying [`Document`].
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// Convenience: returns `self.document().canonical_hash()`.
    ///
    /// See [`Document::canonical_hash`] for the contract. Used by the
    /// autosave subsystem (A3.1) to elide redundant snapshots and to
    /// build the `autosave-{epoch_ms}-{hash16}.cadk` filename suffix.
    pub fn canonical_hash(&self) -> u64 {
        self.document.canonical_hash()
    }

    /// Returns the active prefix of the command log — i.e. the commands
    /// that have actually been applied to the document.
    pub fn log(&self) -> &[Command] {
        &self.log[..self.cursor]
    }

    /// Returns the full command list including the redo stack. `log()`
    /// returns the applied prefix; `full_log()[cursor()..]` is the redo
    /// stack waiting for [`Self::redo`].
    pub fn full_log(&self) -> &[Command] {
        &self.log
    }

    /// Number of commands currently applied to the document.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Returns true if there is at least one command available to undo.
    pub fn can_undo(&self) -> bool {
        self.cursor > 0
    }

    /// Returns true if there is at least one command available to redo.
    pub fn can_redo(&self) -> bool {
        self.cursor < self.log.len()
    }

    /// Replays a slice of commands from scratch and returns the resulting
    /// session. The first failure aborts replay and is returned.
    /// Coalescing is disabled during replay so the resulting log is
    /// identical to the input slice — important for deterministic
    /// reproduction from snapshots.
    pub fn replay(commands: &[Command]) -> ApiResult<Self> {
        let mut session = Self::new();
        session.coalesce_window_ms = 0;
        for cmd in commands {
            session.execute(cmd.clone())?;
        }
        session.coalesce_window_ms = 1000;
        Ok(session)
    }

    /// Serializes only the applied prefix of the command log to a JSON
    /// string. Round-trips with [`Self::replay_from_json`]. To preserve the
    /// redo stack as well, use [`Self::save_to_json`] /
    /// [`Self::load_from_json`].
    pub fn log_to_json(&self) -> ApiResult<String> {
        Ok(serde_json::to_string_pretty(self.log())?)
    }

    /// Replays a session from a JSON-encoded log produced by
    /// [`Self::log_to_json`].
    pub fn replay_from_json(json: &str) -> ApiResult<Self> {
        let log: Vec<Command> = serde_json::from_str(json)?;
        Self::replay(&log)
    }

    /// Save the entire session (log + redo stack + cursor) as a JSON
    /// [`SessionSnapshot`]. Populates A2 metadata fields (`document_hash`,
    /// `log_position`, `timestamp`) automatically. `label` defaults to
    /// `None`; use [`Self::save_to_json_with_label`] to set it.
    pub fn save_to_json(&self) -> ApiResult<String> {
        self.save_to_json_with_label(None)
    }

    /// Variant of [`Self::save_to_json`] that attaches a human label to
    /// the snapshot.
    pub fn save_to_json_with_label(&self, label: Option<String>) -> ApiResult<String> {
        let snap = SessionSnapshot {
            schema_version: SessionSnapshot::CURRENT_SCHEMA,
            commands: self.log.clone(),
            cursor: self.cursor,
            document_hash: hash_command_prefix(&self.log[..self.cursor]),
            log_position: self.cursor,
            timestamp: now_unix_seconds(),
            label,
        };
        Ok(serde_json::to_string_pretty(&snap)?)
    }

    /// Encode the applied prefix of the command log into the `.cadk`
    /// binary container format. Round-trips with [`Self::load_cadk`].
    /// See [`crate::cadk`] for the on-disk layout.
    pub fn save_cadk(&self) -> ApiResult<Vec<u8>> {
        crate::cadk::encode(self.log())
    }

    /// Same as [`Self::save_cadk`] but additionally embeds an arbitrary
    /// thumbnail payload (typically PNG bytes) into a
    /// `BlobKind::Thumbnail` record. Recoverable via
    /// [`crate::cadk::decode_thumbnail`].
    pub fn save_cadk_with_thumbnail(&self, thumbnail: &[u8]) -> ApiResult<Vec<u8>> {
        crate::cadk::encode_with_thumbnail(self.log(), Some(thumbnail))
    }

    /// Same as [`Self::save_cadk`] but honours [`crate::cadk::SaveOptions`]
    /// — zstd compression of the document blob and / or an embedded
    /// thumbnail blob. Output round-trips through [`Self::load_cadk`] /
    /// [`Self::load_cadk_from_path`] (the compression flag is
    /// auto-detected on read).
    pub fn save_cadk_with_options(&self, options: &crate::cadk::SaveOptions) -> ApiResult<Vec<u8>> {
        crate::cadk::encode_with_options(self.log(), options)
    }

    /// Restore a session from a `.cadk` byte buffer produced by
    /// [`Self::save_cadk`]. The redo stack is **not** preserved by this
    /// codec — only the applied prefix round-trips. Use
    /// [`Self::save_to_json`] / [`Self::load_from_json`] when redo state
    /// matters.
    pub fn load_cadk(bytes: &[u8]) -> ApiResult<Self> {
        let commands = crate::cadk::decode(bytes)?;
        Self::replay(&commands)
    }

    /// Encode the applied command prefix and write it to `path` as a
    /// `.cadk` binary container. Convenience wrapper around
    /// [`Self::save_cadk`] + `std::fs::write`. The destination is
    /// truncated and replaced atomically only as far as the underlying
    /// `std::fs::write` semantics permit (which is **not** crash-safe — see
    /// the autosave API for that).
    ///
    /// I/O errors are surfaced as [`ApiError::Codec`] with a `file io:`
    /// prefix so existing match arms over `ApiError` continue to compile.
    pub fn save_cadk_to_path(&self, path: impl AsRef<std::path::Path>) -> ApiResult<()> {
        let bytes = self.save_cadk()?;
        std::fs::write(path.as_ref(), bytes)
            .map_err(|err| ApiError::Codec(format!("file io: {err}")))
    }

    /// Variant of [`Self::save_cadk_to_path`] that additionally embeds a
    /// thumbnail payload via [`Self::save_cadk_with_thumbnail`].
    pub fn save_cadk_to_path_with_thumbnail(
        &self,
        path: impl AsRef<std::path::Path>,
        thumbnail: &[u8],
    ) -> ApiResult<()> {
        let bytes = self.save_cadk_with_thumbnail(thumbnail)?;
        std::fs::write(path.as_ref(), bytes)
            .map_err(|err| ApiError::Codec(format!("file io: {err}")))
    }

    /// Variant of [`Self::save_cadk_to_path`] that honours
    /// [`crate::cadk::SaveOptions`] (compression + thumbnail).
    pub fn save_cadk_to_path_with_options(
        &self,
        path: impl AsRef<std::path::Path>,
        options: &crate::cadk::SaveOptions,
    ) -> ApiResult<()> {
        let bytes = self.save_cadk_with_options(options)?;
        std::fs::write(path.as_ref(), bytes)
            .map_err(|err| ApiError::Codec(format!("file io: {err}")))
    }

    /// Write a rotated autosave snapshot under `policy.dir` and prune
    /// older snapshots so at most `policy.retain` remain.
    ///
    /// A3.1 (2026-05-13). The snapshot filename follows the convention
    /// `autosave-{epoch_ms}-{hash16}.cadk`, where `epoch_ms` is the
    /// current wall-clock time in milliseconds since the Unix epoch
    /// (best-effort — 0 on clocks earlier than 1970) and `hash16` is
    /// the leading 16 hex chars of [`Self::canonical_hash`]. This
    /// keeps the filename self-describing (timestamp + content
    /// fingerprint) without requiring callers to parse the cadk
    /// header back out.
    ///
    /// `policy.dir` is created on demand via `std::fs::create_dir_all`.
    /// When `policy.compress` is `true`, the document blob is written
    /// with [`crate::cadk::SaveOptions::with_compression`] at level 3
    /// — a deliberate fixed level: autosave optimises for write
    /// latency, not compression ratio.
    ///
    /// After the write succeeds, [`crate::cadk::prune`] removes
    /// snapshots in excess of `policy.retain`. The interval check
    /// (`policy.interval`) is **not** enforced here — it belongs to
    /// the viewer's tick loop. This function writes unconditionally
    /// so unit tests and CLI tools can drive autosave on demand.
    ///
    /// I/O errors are surfaced as [`ApiError::Codec`] with the
    /// `file io:` prefix, matching the convention set by
    /// [`Self::save_cadk_to_path`].
    pub fn write_autosave_snapshot(
        &self,
        policy: &crate::cadk::AutosavePolicy,
    ) -> ApiResult<std::path::PathBuf> {
        std::fs::create_dir_all(&policy.dir)
            .map_err(|err| ApiError::Codec(format!("file io: {err}")))?;

        let epoch_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let hash16 = format!("{:016x}", self.canonical_hash());
        let filename = format!("autosave-{epoch_ms}-{hash16}.cadk");
        let path = policy.dir.join(filename);

        let opts = if policy.compress {
            crate::cadk::SaveOptions::default().with_compression(3)
        } else {
            crate::cadk::SaveOptions::default()
        };
        self.save_cadk_to_path_with_options(&path, &opts)?;

        crate::cadk::prune(&policy.dir, policy.retain)?;
        Ok(path)
    }

    /// Read a `.cadk` container from `path` and restore the session.
    /// Convenience wrapper around `std::fs::read` + [`Self::load_cadk`].
    /// I/O errors are surfaced as [`ApiError::Codec`] with a `file io:`
    /// prefix.
    pub fn load_cadk_from_path(path: impl AsRef<std::path::Path>) -> ApiResult<Self> {
        let bytes = std::fs::read(path.as_ref())
            .map_err(|err| ApiError::Codec(format!("file io: {err}")))?;
        Self::load_cadk(&bytes)
    }

    /// Restore a session from a [`SessionSnapshot`] JSON document.
    /// Replays `commands[..cursor]` and keeps `commands[cursor..]` as the
    /// pending redo stack.
    pub fn load_from_json(json: &str) -> ApiResult<Self> {
        let snap: SessionSnapshot = serde_json::from_str(json)?;
        if snap.schema_version != SessionSnapshot::CURRENT_SCHEMA {
            return Err(ApiError::Codec(format!(
                "unsupported session schema version {} (expected {})",
                snap.schema_version,
                SessionSnapshot::CURRENT_SCHEMA
            )));
        }
        if snap.cursor > snap.commands.len() {
            return Err(ApiError::InvalidArgument(format!(
                "snapshot cursor {} exceeds log length {}",
                snap.cursor,
                snap.commands.len()
            )));
        }
        let mut session = Self::replay(&snap.commands[..snap.cursor])?;
        // Replay() leaves cursor at the active end. Append the unexecuted
        // tail so it is available for redo without re-applying it.
        for cmd in &snap.commands[snap.cursor..] {
            session.log.push(cmd.clone());
        }
        Ok(session)
    }

    /// Executes a single command. On success it is appended to the log,
    /// the cursor advances, and one [`HistoryEvent`] is recorded. Any
    /// pending redo stack is discarded (standard CAD/editor behaviour).
    ///
    /// **Coalescing**: if the previous applied command and the new command
    /// are both property-edit commands of the same kind targeting the
    /// same solid id, and arrive within `coalesce_window_ms`, the
    /// previous log entry is replaced with a merged command instead of
    /// pushing a new one. `Translate` deltas accumulate, `Scale` factors
    /// multiply, `Rename` labels are replaced. No new history event is
    /// emitted in that case.
    pub fn execute(&mut self, command: Command) -> ApiResult<Outcome> {
        // Pure-observer commands: dispatch and return without touching
        // the log, cursor, or history. They cannot be undone/redone
        // because they don't mutate the document.
        if matches!(
            command,
            Command::Measure { .. }
                | Command::Validate
                | Command::ListSolids
                | Command::FindByLabel { .. }
                | Command::HistoryEvents
                | Command::Stats
                | Command::Bounds { .. }
                | Command::Distance { .. }
                | Command::Volume { .. }
                | Command::SurfaceArea { .. }
                | Command::Centroid { .. }
                | Command::IntersectsAabb { .. }
                | Command::Exists { .. }
                | Command::Diagonal { .. }
                | Command::AabbCenter { .. }
                | Command::AabbVolume { .. }
                | Command::ContainsAabb { .. }
                | Command::AabbCorners { .. }
                | Command::SolidLabel { .. }
                | Command::IsEmpty
                | Command::AabbSurfaceArea { .. }
                | Command::SolidCount
                | Command::HistoryCount
                | Command::HasLabel { .. }
                | Command::SolidIds
                | Command::AabbExtents { .. }
                | Command::AabbLongestAxis { .. }
                | Command::AabbShortestAxis { .. }
                | Command::AabbAspectRatio { .. }
                | Command::IsCubic { .. }
                | Command::IsSquareXy { .. }
                | Command::HistoryDescription { .. }
                | Command::IsSquareYz { .. }
                | Command::IsSquareXz { .. }
                | Command::OperationCount { .. }
                | Command::LastOperation
                | Command::HasOperation { .. }
                | Command::FirstOperation
        ) {
            return self.dispatch(&command);
        }

        // Discard the redo stack — a new branch starts here.
        self.log.truncate(self.cursor);

        // Try to coalesce with the previous command. If it succeeds we
        // dispatch the new (delta) command onto the live document, then
        // overwrite the previous log entry with the merged form so a
        // future replay reproduces the same final state in one step. No
        // new history event is emitted.
        if let Some(merged) = self.try_coalesce_with_previous(&command) {
            let outcome = self.dispatch(&command)?;
            self.log[self.cursor - 1] = merged;
            self.last_command_at = Some(std::time::Instant::now());
            return Ok(outcome);
        }

        let outcome = self.dispatch(&command)?;
        let event = HistoryEvent {
            op: command.op_name().to_string(),
            primary: outcome.primary_id(),
            description: history_description(&command, &outcome),
            feature_id: FeatureId::default(),
        };
        self.document.push_history(event);
        self.log.push(command);
        self.cursor += 1;
        self.last_command_at = Some(std::time::Instant::now());
        Ok(outcome)
    }

    /// Returns the merged command if `incoming` is coalescable with the
    /// previous applied command, else `None`.
    fn try_coalesce_with_previous(&self, incoming: &Command) -> Option<Command> {
        if self.coalesce_window_ms == 0 || self.cursor == 0 {
            return None;
        }
        let last_at = self.last_command_at?;
        if last_at.elapsed().as_millis() as u64 > self.coalesce_window_ms {
            return None;
        }
        let prev = &self.log[self.cursor - 1];
        match (prev, incoming) {
            (
                Command::Translate {
                    id: a,
                    dx: x1,
                    dy: y1,
                    dz: z1,
                },
                Command::Translate {
                    id: b,
                    dx: x2,
                    dy: y2,
                    dz: z2,
                },
            ) if a == b => Some(Command::Translate {
                id: *a,
                dx: x1 + x2,
                dy: y1 + y2,
                dz: z1 + z2,
            }),
            (Command::Scale { id: a, factor: f1 }, Command::Scale { id: b, factor: f2 })
                if a == b =>
            {
                Some(Command::Scale {
                    id: *a,
                    factor: f1 * f2,
                })
            }
            (Command::Rename { id: a, .. }, Command::Rename { id: b, label }) if a == b => {
                Some(Command::Rename {
                    id: *a,
                    label: label.clone(),
                })
            }
            _ => None,
        }
    }

    /// Undo the most recently applied command. Returns the [`Command`] that
    /// was undone (handed back so callers can inspect it). Rebuilds the
    /// document by replaying `log[..cursor-1]` from scratch.
    pub fn undo(&mut self) -> ApiResult<Command> {
        if !self.can_undo() {
            return Err(ApiError::InvalidArgument(
                "nothing to undo (cursor is at start of log)".into(),
            ));
        }
        let last = self.log[self.cursor - 1].clone();
        let new_cursor = self.cursor - 1;
        let prefix: Vec<Command> = self.log[..new_cursor].to_vec();
        let rebuilt = Self::replay(&prefix)?;
        // Adopt the rebuilt document but preserve the original log
        // (rebuilt.log only contains the prefix; we need to keep the redo
        // stack as well).
        self.document = rebuilt.document;
        self.cursor = new_cursor;
        Ok(last)
    }

    /// Redo the next command on the redo stack. Returns the [`Outcome`] of
    /// re-executing it.
    pub fn redo(&mut self) -> ApiResult<Outcome> {
        if !self.can_redo() {
            return Err(ApiError::InvalidArgument(
                "nothing to redo (cursor is at end of log)".into(),
            ));
        }
        let cmd = self.log[self.cursor].clone();
        let outcome = self.dispatch(&cmd)?;
        let event = HistoryEvent {
            op: cmd.op_name().to_string(),
            primary: outcome.primary_id(),
            description: history_description(&cmd, &outcome),
            feature_id: FeatureId::default(),
        };
        self.document.push_history(event);
        self.cursor += 1;
        Ok(outcome)
    }

    fn dispatch(&mut self, command: &Command) -> ApiResult<Outcome> {
        match command {
            Command::CreateBox { dx, dy, dz } => {
                let model = quick_box(*dx, *dy, *dz)?;
                self.insert_first_solid(model, "Box")
            }
            Command::CreateCylinder { radius, height } => {
                let model = quick_cylinder(*radius, *height)?;
                self.insert_first_solid(model, "Cylinder")
            }
            Command::CreateSphere { radius } => {
                let model = quick_sphere(*radius)?;
                self.insert_first_solid(model, "Sphere")
            }
            Command::CreateCone {
                radius,
                height,
                top_radius,
            } => {
                let model = create_cone_model(*radius, *height, *top_radius)?;
                let label = if *top_radius > 0.0 { "Frustum" } else { "Cone" };
                self.insert_first_solid(model, label)
            }
            Command::CreateTorus {
                major_radius,
                minor_radius,
            } => {
                let model = quick_torus(*major_radius, *minor_radius)?;
                self.insert_first_solid(model, "Torus")
            }
            Command::BooleanUnion { lhs, rhs } => self.boolean(*lhs, *rhs, BooleanKind::Union),
            Command::BooleanSubtract { lhs, rhs } => {
                self.boolean(*lhs, *rhs, BooleanKind::Subtract)
            }
            Command::BooleanIntersect { lhs, rhs } => {
                self.boolean(*lhs, *rhs, BooleanKind::Intersect)
            }
            Command::Translate { id, dx, dy, dz } => self.translate(*id, *dx, *dy, *dz),
            Command::Scale { id, factor } => self.scale_uniform(*id, *factor),
            Command::ScaleNonUniform { id, factors, point } => {
                self.scale_non_uniform(*id, *factors, *point)
            }
            Command::CenterOnOrigin { id } => self.center_on_origin(*id),
            Command::AlignTo { id, target_id } => self.align_to(*id, *target_id),
            Command::ScaleToFit { id, target_size } => self.scale_to_fit(*id, *target_size),
            Command::TranslateTo { id, point } => self.translate_to(*id, *point),
            Command::Rename { id, label } => self.rename(*id, label.clone()),
            Command::DeleteSolid { id } => {
                if self.document.remove(*id) {
                    Ok(Outcome::SolidDeleted { id: *id })
                } else {
                    Err(ApiError::UnknownSolid(format!("{id}")))
                }
            }
            Command::Extrude {
                profile,
                direction,
                distance,
                kind,
            } => self.extrude_profile(profile, *direction, *distance, *kind),
            Command::LinearPattern {
                id,
                direction,
                spacing,
                count,
                skip_instances,
                features,
                mirror_alternate,
                instance_overrides,
            } => {
                let params = LinearPatternParams {
                    direction: *direction,
                    spacing: *spacing,
                    count: *count,
                    skip_instances,
                    instance_overrides,
                    mirror_alternate: *mirror_alternate,
                };
                if features.is_empty() {
                    self.linear_pattern_extended(*id, params)
                } else {
                    self.linear_pattern_features(features, params)
                }
            }
            Command::Mirror {
                id,
                point,
                normal,
                merge,
                features,
            } => {
                if features.is_empty() {
                    self.mirror(*id, *point, *normal, *merge)
                } else {
                    self.mirror_features(features, *point, *normal, *merge)
                }
            }
            Command::Measure { id } => self.measure(*id),
            Command::Validate => Ok(Outcome::Validated {
                issues: self.document.validate(),
            }),
            Command::ListSolids => Ok(Outcome::SolidsListed {
                entries: self
                    .document
                    .solid_ids()
                    .into_iter()
                    .map(|id| crate::outcome::SolidEntry {
                        id,
                        label: self.document.solid_label(id).unwrap_or("").to_string(),
                    })
                    .collect(),
            }),
            Command::FindByLabel { query } => {
                let needle = query.to_lowercase();
                Ok(Outcome::SolidsListed {
                    entries: self
                        .document
                        .solid_ids()
                        .into_iter()
                        .filter_map(|id| {
                            let label = self.document.solid_label(id).unwrap_or("");
                            if needle.is_empty() || label.to_lowercase().contains(&needle) {
                                Some(crate::outcome::SolidEntry {
                                    id,
                                    label: label.to_string(),
                                })
                            } else {
                                None
                            }
                        })
                        .collect(),
                })
            }
            Command::Duplicate { id } => self.duplicate(*id),
            Command::HistoryEvents => Ok(Outcome::HistoryListed {
                events: self.document.history().to_vec(),
            }),
            Command::Stats => Ok(Outcome::Stats {
                solid_count: self.document.solid_count() as u32,
                history_count: self.document.history().len() as u32,
            }),
            Command::Bounds { id } => {
                let bbox = self
                    .document
                    .bounding_box(*id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
                Ok(Outcome::Bounds {
                    id: *id,
                    min: bbox.min,
                    max: bbox.max,
                })
            }
            Command::Distance { id_a, id_b } => {
                let a = self
                    .document
                    .measure_solid(*id_a)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id_a}")))?;
                let b = self
                    .document
                    .measure_solid(*id_b)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id_b}")))?;
                let delta = [
                    b.centroid[0] - a.centroid[0],
                    b.centroid[1] - a.centroid[1],
                    b.centroid[2] - a.centroid[2],
                ];
                let distance =
                    (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
                Ok(Outcome::Distance {
                    id_a: *id_a,
                    id_b: *id_b,
                    distance,
                    delta,
                })
            }
            Command::Volume { id } => {
                let m = self
                    .document
                    .measure_solid(*id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
                Ok(Outcome::Volume {
                    id: *id,
                    volume: m.volume,
                })
            }
            Command::SurfaceArea { id } => {
                let m = self
                    .document
                    .measure_solid(*id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
                Ok(Outcome::SurfaceArea {
                    id: *id,
                    surface_area: m.surface_area,
                })
            }
            Command::Centroid { id } => {
                let m = self
                    .document
                    .measure_solid(*id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
                Ok(Outcome::Centroid {
                    id: *id,
                    centroid: m.centroid,
                })
            }
            Command::IntersectsAabb { id_a, id_b } => {
                let a = self
                    .document
                    .bounding_box(*id_a)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id_a}")))?;
                let b = self
                    .document
                    .bounding_box(*id_b)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id_b}")))?;
                let mut intersects = true;
                let mut omin = [0.0; 3];
                let mut omax = [0.0; 3];
                for i in 0..3 {
                    let lo = a.min[i].max(b.min[i]);
                    let hi = a.max[i].min(b.max[i]);
                    if lo > hi {
                        intersects = false;
                    }
                    omin[i] = lo;
                    omax[i] = hi;
                }
                if !intersects {
                    omin = [0.0; 3];
                    omax = [0.0; 3];
                }
                Ok(Outcome::AabbIntersection {
                    id_a: *id_a,
                    id_b: *id_b,
                    intersects,
                    overlap_min: omin,
                    overlap_max: omax,
                })
            }
            Command::Exists { id } => {
                let exists = self.document.solid_label(*id).is_some();
                Ok(Outcome::Exists { id: *id, exists })
            }
            Command::Diagonal { id } => {
                let bbox = self
                    .document
                    .bounding_box(*id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
                let extents = [
                    bbox.max[0] - bbox.min[0],
                    bbox.max[1] - bbox.min[1],
                    bbox.max[2] - bbox.min[2],
                ];
                let length =
                    (extents[0] * extents[0] + extents[1] * extents[1] + extents[2] * extents[2])
                        .sqrt();
                Ok(Outcome::Diagonal {
                    id: *id,
                    length,
                    extents,
                })
            }
            Command::AabbCenter { id } => {
                let bbox = self
                    .document
                    .bounding_box(*id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
                let center = [
                    (bbox.min[0] + bbox.max[0]) * 0.5,
                    (bbox.min[1] + bbox.max[1]) * 0.5,
                    (bbox.min[2] + bbox.max[2]) * 0.5,
                ];
                Ok(Outcome::AabbCenter { id: *id, center })
            }
            Command::AabbVolume { id } => {
                let bbox = self
                    .document
                    .bounding_box(*id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
                let dx = bbox.max[0] - bbox.min[0];
                let dy = bbox.max[1] - bbox.min[1];
                let dz = bbox.max[2] - bbox.min[2];
                Ok(Outcome::AabbVolume {
                    id: *id,
                    volume: dx * dy * dz,
                })
            }
            Command::ContainsAabb { id_outer, id_inner } => {
                let outer = self
                    .document
                    .bounding_box(*id_outer)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id_outer}")))?;
                let inner = self
                    .document
                    .bounding_box(*id_inner)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id_inner}")))?;
                let contains =
                    (0..3).all(|i| outer.min[i] <= inner.min[i] && inner.max[i] <= outer.max[i]);
                Ok(Outcome::AabbContainment {
                    id_outer: *id_outer,
                    id_inner: *id_inner,
                    contains,
                })
            }
            Command::AabbCorners { id } => {
                let bbox = self
                    .document
                    .bounding_box(*id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
                let lo = bbox.min;
                let hi = bbox.max;
                let corners = [
                    [lo[0], lo[1], lo[2]],
                    [hi[0], lo[1], lo[2]],
                    [lo[0], hi[1], lo[2]],
                    [hi[0], hi[1], lo[2]],
                    [lo[0], lo[1], hi[2]],
                    [hi[0], lo[1], hi[2]],
                    [lo[0], hi[1], hi[2]],
                    [hi[0], hi[1], hi[2]],
                ];
                Ok(Outcome::AabbCorners { id: *id, corners })
            }
            Command::SolidLabel { id } => {
                let label = self
                    .document
                    .solid_label(*id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?
                    .to_string();
                Ok(Outcome::SolidLabel { id: *id, label })
            }
            Command::IsEmpty => Ok(Outcome::IsEmpty {
                is_empty: self.document.solid_count() == 0,
            }),
            Command::AabbSurfaceArea { id } => {
                let bbox = self
                    .document
                    .bounding_box(*id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
                let dx = bbox.max[0] - bbox.min[0];
                let dy = bbox.max[1] - bbox.min[1];
                let dz = bbox.max[2] - bbox.min[2];
                let surface_area = 2.0 * (dx * dy + dy * dz + dz * dx);
                Ok(Outcome::AabbSurfaceArea {
                    id: *id,
                    surface_area,
                })
            }
            Command::SolidCount => Ok(Outcome::SolidCount {
                count: self.document.solid_count() as u32,
            }),
            Command::HistoryCount => Ok(Outcome::HistoryCount {
                count: self.document.history().len() as u32,
            }),
            Command::HasLabel { query } => {
                let needle = query.to_lowercase();
                let has_label = !needle.is_empty()
                    && self.document.solid_ids().into_iter().any(|id| {
                        self.document
                            .solid_label(id)
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&needle)
                    });
                Ok(Outcome::HasLabel {
                    query: query.clone(),
                    has_label,
                })
            }
            Command::SolidIds => Ok(Outcome::SolidIds {
                ids: self.document.solid_ids(),
            }),
            Command::AabbExtents { id } => {
                let bbox = self
                    .document
                    .bounding_box(*id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
                Ok(Outcome::AabbExtents {
                    id: *id,
                    extents: [
                        bbox.max[0] - bbox.min[0],
                        bbox.max[1] - bbox.min[1],
                        bbox.max[2] - bbox.min[2],
                    ],
                })
            }
            Command::AabbLongestAxis { id } => {
                let bbox = self
                    .document
                    .bounding_box(*id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
                let dx = bbox.max[0] - bbox.min[0];
                let dy = bbox.max[1] - bbox.min[1];
                let dz = bbox.max[2] - bbox.min[2];
                let axis: u8 = if dx >= dy && dx >= dz {
                    0
                } else if dy >= dz {
                    1
                } else {
                    2
                };
                Ok(Outcome::AabbLongestAxis { id: *id, axis })
            }
            Command::AabbShortestAxis { id } => {
                let bbox = self
                    .document
                    .bounding_box(*id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
                let dx = bbox.max[0] - bbox.min[0];
                let dy = bbox.max[1] - bbox.min[1];
                let dz = bbox.max[2] - bbox.min[2];
                let axis: u8 = if dx <= dy && dx <= dz {
                    0
                } else if dy <= dz {
                    1
                } else {
                    2
                };
                Ok(Outcome::AabbShortestAxis { id: *id, axis })
            }
            Command::AabbAspectRatio { id } => {
                let bbox = self
                    .document
                    .bounding_box(*id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
                let dx = bbox.max[0] - bbox.min[0];
                let dy = bbox.max[1] - bbox.min[1];
                let dz = bbox.max[2] - bbox.min[2];
                let longest = dx.max(dy).max(dz);
                let shortest = dx.min(dy).min(dz);
                let ratio = if shortest == 0.0 {
                    f64::INFINITY
                } else {
                    longest / shortest
                };
                Ok(Outcome::AabbAspectRatio { id: *id, ratio })
            }
            Command::IsCubic { id } => {
                let bbox = self
                    .document
                    .bounding_box(*id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
                let dx = bbox.max[0] - bbox.min[0];
                let dy = bbox.max[1] - bbox.min[1];
                let dz = bbox.max[2] - bbox.min[2];
                let longest = dx.max(dy).max(dz);
                let shortest = dx.min(dy).min(dz);
                let cubic = (longest - shortest) <= 1e-9;
                Ok(Outcome::IsCubic { id: *id, cubic })
            }
            Command::IsSquareXy { id } => {
                let bbox = self
                    .document
                    .bounding_box(*id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
                let dx = bbox.max[0] - bbox.min[0];
                let dy = bbox.max[1] - bbox.min[1];
                let square = (dx - dy).abs() <= 1e-9;
                Ok(Outcome::IsSquareXy { id: *id, square })
            }
            Command::HistoryDescription { index } => {
                let history = self.document.history();
                let event = history.get(*index as usize).ok_or_else(|| {
                    ApiError::InvalidArgument(format!(
                        "history index {index} out of bounds (len = {})",
                        history.len()
                    ))
                })?;
                Ok(Outcome::HistoryDescription {
                    index: *index,
                    description: event.description.clone(),
                })
            }
            Command::IsSquareYz { id } => {
                let bbox = self
                    .document
                    .bounding_box(*id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
                let dy = bbox.max[1] - bbox.min[1];
                let dz = bbox.max[2] - bbox.min[2];
                let square = (dy - dz).abs() <= 1e-9;
                Ok(Outcome::IsSquareYz { id: *id, square })
            }
            Command::IsSquareXz { id } => {
                let bbox = self
                    .document
                    .bounding_box(*id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
                let dx = bbox.max[0] - bbox.min[0];
                let dz = bbox.max[2] - bbox.min[2];
                let square = (dx - dz).abs() <= 1e-9;
                Ok(Outcome::IsSquareXz { id: *id, square })
            }
            Command::OperationCount { op_name } => {
                let count = self
                    .document
                    .history()
                    .iter()
                    .filter(|event| event.op == *op_name)
                    .count() as u32;
                Ok(Outcome::OperationCount {
                    op_name: op_name.clone(),
                    count,
                })
            }
            Command::LastOperation => {
                let history = self.document.history();
                let len = history.len();
                if len == 0 {
                    return Err(ApiError::InvalidArgument("history is empty".to_string()));
                }
                let index = (len - 1) as u32;
                let event = &history[len - 1];
                Ok(Outcome::LastOperation {
                    index,
                    op_name: event.op.clone(),
                    description: event.description.clone(),
                })
            }
            Command::HasOperation { op_name } => {
                let present = self
                    .document
                    .history()
                    .iter()
                    .any(|event| event.op == *op_name);
                Ok(Outcome::HasOperation {
                    op_name: op_name.clone(),
                    present,
                })
            }
            Command::FirstOperation => {
                let history = self.document.history();
                if history.is_empty() {
                    return Err(ApiError::InvalidArgument("history is empty".to_string()));
                }
                let event = &history[0];
                Ok(Outcome::FirstOperation {
                    op_name: event.op.clone(),
                    description: event.description.clone(),
                })
            }
            Command::Rotate {
                id,
                axis,
                angle_rad,
                point,
            } => self.rotate(*id, *axis, *angle_rad, *point),
            Command::NewDocument => {
                self.document = Document::new();
                self.log.clear();
                self.cursor = 0;
                Ok(Outcome::DocumentReset)
            }
            Command::Noop => Ok(Outcome::Empty),
        }
    }

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn insert_first_solid(&mut self, model: BRepModel, label: &str) -> ApiResult<Outcome> {
        let handle = first_solid_handle(&model).ok_or_else(|| {
            ApiError::Kernel("primitive constructor returned a model with no solid".into())
        })?;
        let id = self.document.insert(model, handle, label);
        Ok(Outcome::SolidCreated {
            id,
            label: label.to_string(),
        })
    }

    fn boolean(&mut self, lhs: SolidId, rhs: SolidId, kind: BooleanKind) -> ApiResult<Outcome> {
        // Both operands must exist before we mutate anything.
        let (lhs_model, rhs_model) = self.borrow_two(lhs, rhs)?;
        let result_model = match kind {
            BooleanKind::Union => quick_union(lhs_model, rhs_model),
            BooleanKind::Subtract => quick_subtract(lhs_model, rhs_model),
            BooleanKind::Intersect => quick_intersect(lhs_model, rhs_model),
        }?;

        let handle = first_solid_handle(&result_model).ok_or_else(|| {
            ApiError::Kernel("boolean produced a model with no resulting solid".into())
        })?;
        // Only mutate the document after the kernel call succeeded.
        self.document.remove(lhs);
        self.document.remove(rhs);
        let label = match kind {
            BooleanKind::Union => "Union",
            BooleanKind::Subtract => "Subtract",
            BooleanKind::Intersect => "Intersect",
        };
        let result = self.document.insert(result_model, handle, label);
        Ok(Outcome::Booleaned {
            result,
            consumed: vec![lhs, rhs],
        })
    }

    fn borrow_two(&self, lhs: SolidId, rhs: SolidId) -> ApiResult<(&BRepModel, &BRepModel)> {
        if lhs == rhs {
            return Err(ApiError::InvalidArgument(format!(
                "boolean lhs and rhs must differ, got {lhs} twice"
            )));
        }
        let l = self
            .document
            .get_slot(lhs)
            .ok_or_else(|| ApiError::UnknownSolid(format!("{lhs}")))?;
        let r = self
            .document
            .get_slot(rhs)
            .ok_or_else(|| ApiError::UnknownSolid(format!("{rhs}")))?;
        Ok((&l.model, &r.model))
    }

    fn translate(&mut self, id: SolidId, dx: f64, dy: f64, dz: f64) -> ApiResult<Outcome> {
        let slot = slot_mut(&mut self.document, id)?;
        for (_h, v) in slot.model.vertices.iter_mut() {
            v.point = Point3::new(v.point.x + dx, v.point.y + dy, v.point.z + dz);
        }
        Ok(Outcome::SolidModified { id })
    }

    fn scale_uniform(&mut self, id: SolidId, factor: f64) -> ApiResult<Outcome> {
        if factor <= 0.0 {
            return Err(ApiError::InvalidArgument(format!(
                "scale factor must be > 0, got {factor}"
            )));
        }
        let centroid = {
            let slot = self
                .document
                .get_slot(id)
                .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
            let handle = slot
                .handle
                .ok_or_else(|| ApiError::UnknownSolid(format!("{id} has no solid handle")))?;
            let mp = solid_mass_properties(&slot.model, handle)?;
            mp.centroid
        };
        let slot = slot_mut(&mut self.document, id)?;
        for (_h, v) in slot.model.vertices.iter_mut() {
            v.point = Point3::new(
                centroid.x + (v.point.x - centroid.x) * factor,
                centroid.y + (v.point.y - centroid.y) * factor,
                centroid.z + (v.point.z - centroid.z) * factor,
            );
        }
        Ok(Outcome::SolidModified { id })
    }

    fn scale_non_uniform(
        &mut self,
        id: SolidId,
        factors: [f64; 3],
        point: [f64; 3],
    ) -> ApiResult<Outcome> {
        if factors[0] <= 0.0 || factors[1] <= 0.0 || factors[2] <= 0.0 {
            return Err(ApiError::InvalidArgument(format!(
                "scale factors must all be > 0, got [{}, {}, {}]",
                factors[0], factors[1], factors[2]
            )));
        }
        let slot = slot_mut(&mut self.document, id)?;
        let [sx, sy, sz] = factors;
        let [px, py, pz] = point;
        for (_h, v) in slot.model.vertices.iter_mut() {
            v.point = Point3::new(
                px + (v.point.x - px) * sx,
                py + (v.point.y - py) * sy,
                pz + (v.point.z - pz) * sz,
            );
        }
        Ok(Outcome::SolidModified { id })
    }

    fn center_on_origin(&mut self, id: SolidId) -> ApiResult<Outcome> {
        let centroid = {
            let slot = self
                .document
                .get_slot(id)
                .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
            let handle = slot
                .handle
                .ok_or_else(|| ApiError::UnknownSolid(format!("{id} has no solid handle")))?;
            let mp = solid_mass_properties(&slot.model, handle)?;
            mp.centroid
        };
        let slot = slot_mut(&mut self.document, id)?;
        for (_h, v) in slot.model.vertices.iter_mut() {
            v.point = Point3::new(
                v.point.x - centroid.x,
                v.point.y - centroid.y,
                v.point.z - centroid.z,
            );
        }
        Ok(Outcome::SolidModified { id })
    }

    fn align_to(&mut self, id: SolidId, target_id: SolidId) -> ApiResult<Outcome> {
        if id == target_id {
            return Ok(Outcome::SolidModified { id });
        }
        let target_centroid = {
            let slot = self
                .document
                .get_slot(target_id)
                .ok_or_else(|| ApiError::UnknownSolid(format!("{target_id}")))?;
            let handle = slot.handle.ok_or_else(|| {
                ApiError::UnknownSolid(format!("{target_id} has no solid handle"))
            })?;
            let mp = solid_mass_properties(&slot.model, handle)?;
            mp.centroid
        };
        let source_centroid = {
            let slot = self
                .document
                .get_slot(id)
                .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
            let handle = slot
                .handle
                .ok_or_else(|| ApiError::UnknownSolid(format!("{id} has no solid handle")))?;
            let mp = solid_mass_properties(&slot.model, handle)?;
            mp.centroid
        };
        let dx = target_centroid.x - source_centroid.x;
        let dy = target_centroid.y - source_centroid.y;
        let dz = target_centroid.z - source_centroid.z;
        let slot = slot_mut(&mut self.document, id)?;
        for (_h, v) in slot.model.vertices.iter_mut() {
            v.point = Point3::new(v.point.x + dx, v.point.y + dy, v.point.z + dz);
        }
        Ok(Outcome::SolidModified { id })
    }

    fn scale_to_fit(&mut self, id: SolidId, target_size: f64) -> ApiResult<Outcome> {
        if target_size <= 0.0 {
            return Err(ApiError::InvalidArgument(format!(
                "target_size must be > 0, got {target_size}"
            )));
        }
        let bbox = self
            .document
            .bounding_box(id)
            .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
        let extents = [
            bbox.max[0] - bbox.min[0],
            bbox.max[1] - bbox.min[1],
            bbox.max[2] - bbox.min[2],
        ];
        let max_extent = extents.iter().cloned().fold(0.0_f64, f64::max);
        if max_extent <= 0.0 {
            return Err(ApiError::InvalidArgument(format!(
                "{id} has degenerate bounding box (max extent {max_extent})"
            )));
        }
        let factor = target_size / max_extent;
        self.scale_uniform(id, factor)
    }

    fn translate_to(&mut self, id: SolidId, point: [f64; 3]) -> ApiResult<Outcome> {
        let centroid = {
            let slot = self
                .document
                .get_slot(id)
                .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
            let handle = slot
                .handle
                .ok_or_else(|| ApiError::UnknownSolid(format!("{id} has no solid handle")))?;
            let mp = solid_mass_properties(&slot.model, handle)?;
            mp.centroid
        };
        let dx = point[0] - centroid.x;
        let dy = point[1] - centroid.y;
        let dz = point[2] - centroid.z;
        let slot = slot_mut(&mut self.document, id)?;
        for (_h, v) in slot.model.vertices.iter_mut() {
            v.point = Point3::new(v.point.x + dx, v.point.y + dy, v.point.z + dz);
        }
        Ok(Outcome::SolidModified { id })
    }

    fn rename(&mut self, id: SolidId, label: String) -> ApiResult<Outcome> {
        let slot = slot_mut(&mut self.document, id)?;
        slot.label = label;
        Ok(Outcome::SolidModified { id })
    }

    fn measure(&self, id: SolidId) -> ApiResult<Outcome> {
        let m = self
            .document
            .measure_solid(id)
            .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
        let bbox = self
            .document
            .bounding_box(id)
            .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
        Ok(Outcome::Measured {
            id,
            volume: m.volume,
            surface_area: m.surface_area,
            centroid: m.centroid,
            bbox_min: bbox.min,
            bbox_max: bbox.max,
        })
    }

    fn duplicate(&mut self, src_id: SolidId) -> ApiResult<Outcome> {
        let (model, handle, label) = {
            let slot = self
                .document
                .get_slot(src_id)
                .ok_or_else(|| ApiError::UnknownSolid(format!("{src_id}")))?;
            let handle = slot
                .handle
                .ok_or_else(|| ApiError::UnknownSolid(format!("{src_id} has no solid handle")))?;
            (slot.model.clone(), handle, format!("{} (copy)", slot.label))
        };
        let new_id = self.document.insert(model, handle, label.clone());
        Ok(Outcome::SolidCreated { id: new_id, label })
    }

    fn rotate(
        &mut self,
        id: SolidId,
        axis: [f64; 3],
        angle_rad: f64,
        point: [f64; 3],
    ) -> ApiResult<Outcome> {
        let axis_vec = Vec3::new(axis[0], axis[1], axis[2]);
        let len_sq = axis_vec.x * axis_vec.x + axis_vec.y * axis_vec.y + axis_vec.z * axis_vec.z;
        if len_sq <= f64::EPSILON {
            return Err(ApiError::InvalidArgument(format!(
                "rotation axis must have non-zero length, got {axis:?}"
            )));
        }
        let inv_len = 1.0 / len_sq.sqrt();
        let unit_axis = Vec3::new(
            axis_vec.x * inv_len,
            axis_vec.y * inv_len,
            axis_vec.z * inv_len,
        );
        let q = Quaternion::from_axis_angle(unit_axis, angle_rad);
        let pivot = Point3::new(point[0], point[1], point[2]);
        let slot = slot_mut(&mut self.document, id)?;
        for (_h, v) in slot.model.vertices.iter_mut() {
            let rel = Vec3::new(
                v.point.x - pivot.x,
                v.point.y - pivot.y,
                v.point.z - pivot.z,
            );
            let rotated = q.rotate_vec(rel);
            v.point = Point3::new(
                pivot.x + rotated.x,
                pivot.y + rotated.y,
                pivot.z + rotated.z,
            );
        }
        Ok(Outcome::SolidModified { id })
    }

    fn extrude_profile(
        &mut self,
        profile: &[[f64; 3]],
        direction: [f64; 3],
        distance: f64,
        kind: ExtrudeKind,
    ) -> ApiResult<Outcome> {
        if profile.len() < 3 {
            return Err(ApiError::InvalidArgument(format!(
                "extrude profile must have at least 3 points, got {}",
                profile.len()
            )));
        }
        let dir = Vec3::new(direction[0], direction[1], direction[2]);
        if dir.length() < 1e-12 {
            return Err(ApiError::InvalidArgument(
                "extrude direction must be non-zero".into(),
            ));
        }
        let dir_unit = dir
            .normalized()
            .ok_or_else(|| ApiError::Kernel("extrude direction failed to normalize".into()))?;
        let (back_shift, total_distance) = match kind {
            ExtrudeKind::Blind => {
                validate_extrude_distance(distance)?;
                (0.0, distance)
            }
            ExtrudeKind::MidPlane => {
                validate_extrude_distance(distance)?;
                (distance * 0.5, distance)
            }
            ExtrudeKind::TwoSided { back_distance } => {
                validate_extrude_distance(distance)?;
                if back_distance <= 0.0 {
                    return Err(ApiError::InvalidArgument(format!(
                        "extrude TwoSided back_distance must be > 0, got {back_distance}"
                    )));
                }
                (back_distance, distance + back_distance)
            }
            ExtrudeKind::ThroughAll => (0.0, self.extrude_through_all_distance(dir_unit)?),
            ExtrudeKind::UpToFace {
                face_solid,
                face_index,
            } => (
                0.0,
                self.extrude_up_to_face_distance(profile, dir_unit, face_solid, face_index)?,
            ),
        };
        let pts: Vec<Point3> = profile
            .iter()
            .map(|p| {
                Point3::new(
                    p[0] - dir_unit.x * back_shift,
                    p[1] - dir_unit.y * back_shift,
                    p[2] - dir_unit.z * back_shift,
                )
            })
            .collect();
        let mut model = BRepModel::new();
        // Pass `dir` (un-normalized) so the kernel's existing direction
        // handling is preserved; total_distance is the post-kind span.
        let result = extrude(&mut model, &pts, dir, total_distance)?;
        let id = self.document.insert(model, result.solid, "Extrude");
        Ok(Outcome::SolidCreated {
            id,
            label: "Extrude".into(),
        })
    }

    fn extrude_through_all_distance(&self, dir_unit: Vec3) -> ApiResult<f64> {
        let mut found = false;
        let mut global_min = [f64::INFINITY; 3];
        let mut global_max = [f64::NEG_INFINITY; 3];
        for id in self.document.solid_ids() {
            let Some(bbox) = self.document.bounding_box(id) else {
                continue;
            };
            found = true;
            for axis in 0..3 {
                global_min[axis] = global_min[axis].min(bbox.min[axis]);
                global_max[axis] = global_max[axis].max(bbox.max[axis]);
            }
        }
        if !found {
            return Err(ApiError::InvalidArgument(
                "ThroughAll requires at least one existing solid to bound against".into(),
            ));
        }

        let corners = [
            [global_min[0], global_min[1], global_min[2]],
            [global_min[0], global_min[1], global_max[2]],
            [global_min[0], global_max[1], global_min[2]],
            [global_min[0], global_max[1], global_max[2]],
            [global_max[0], global_min[1], global_min[2]],
            [global_max[0], global_min[1], global_max[2]],
            [global_max[0], global_max[1], global_min[2]],
            [global_max[0], global_max[1], global_max[2]],
        ];
        let mut min_dot = f64::INFINITY;
        let mut max_dot = f64::NEG_INFINITY;
        for corner in corners {
            let dot = corner[0] * dir_unit.x + corner[1] * dir_unit.y + corner[2] * dir_unit.z;
            min_dot = min_dot.min(dot);
            max_dot = max_dot.max(dot);
        }
        let span = max_dot - min_dot;
        Ok(span + span * 0.01)
    }

    fn extrude_up_to_face_distance(
        &self,
        profile: &[[f64; 3]],
        dir_unit: Vec3,
        face_solid: SolidId,
        face_index: u32,
    ) -> ApiResult<f64> {
        let _slot = self.document.get_slot(face_solid).ok_or_else(|| {
            ApiError::InvalidArgument(format!("UpToFace target solid {face_solid} not found"))
        })?;
        let (model, _) = self.document.solid_brep(face_solid).ok_or_else(|| {
            ApiError::InvalidArgument(format!("UpToFace target solid {face_solid} not found"))
        })?;
        let (face_centroid, face_normal) = face_plane(model, face_index)?;
        let profile_centroid = profile_centroid(profile);
        let denom = face_normal.dot(dir_unit);
        if denom.abs() < 1e-12 {
            return Err(ApiError::InvalidArgument(
                "UpToFace target plane is parallel to extrude direction".into(),
            ));
        }
        let t = (face_centroid - profile_centroid).dot(face_normal) / denom;
        if t <= 0.0 {
            return Err(ApiError::InvalidArgument(
                "UpToFace target plane is behind the profile along direction".into(),
            ));
        }
        Ok(t)
    }

    fn linear_pattern_extended(
        &mut self,
        id: SolidId,
        params: LinearPatternParams<'_>,
    ) -> ApiResult<Outcome> {
        let plan = linear_pattern_plan(params)?;
        let source = self.pattern_source(id)?;
        let (ids, total_features) = self.insert_linear_pattern_sources(&[source], &plan)?;
        let instance_count = ids.len() as u32;
        Ok(Outcome::PatternCreated {
            pattern_id: id,
            instance_count,
            total_features,
            ids,
        })
    }

    fn linear_pattern_features(
        &mut self,
        features: &[FeatureId],
        params: LinearPatternParams<'_>,
    ) -> ApiResult<Outcome> {
        let plan = linear_pattern_plan(params)?;

        let mut sources = Vec::with_capacity(features.len());
        for fid in features {
            let event = self
                .document
                .feature(*fid)
                .ok_or_else(|| ApiError::InvalidArgument(format!("unknown feature id: {fid}")))?;
            if let Some(primary) = event.primary {
                sources.push(self.pattern_source(primary)?);
            }
        }
        if sources.is_empty() {
            return Err(ApiError::InvalidArgument(
                "LinearPattern::features resolved to no patternable solids".into(),
            ));
        }

        let pattern_id = sources[0].id;
        let (ids, total_features) = self.insert_linear_pattern_sources(&sources, &plan)?;
        Ok(Outcome::PatternCreated {
            pattern_id,
            instance_count: ids.len() as u32,
            total_features,
            ids,
        })
    }

    fn pattern_source(&self, id: SolidId) -> ApiResult<PatternSource> {
        let slot = self
            .document
            .get_slot(id)
            .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
        let handle = slot
            .handle
            .ok_or_else(|| ApiError::UnknownSolid(format!("{id} has no solid handle")))?;
        Ok(PatternSource {
            id,
            model: slot.model.clone(),
            handle,
            label: slot.label.clone(),
        })
    }

    fn insert_linear_pattern_sources(
        &mut self,
        sources: &[PatternSource],
        plan: &LinearPatternPlan<'_>,
    ) -> ApiResult<(Vec<SolidId>, u32)> {
        let original_skipped = plan.skip.contains(&0);
        let mut ids = Vec::with_capacity(sources.len() * plan.count as usize);
        if !original_skipped {
            ids.extend(sources.iter().map(|source| source.id));
        } else {
            for source in sources {
                self.document.remove(source.id);
            }
        }

        let mut inserted = 0_u32;
        for source in sources {
            for i in 1..plan.count {
                if plan.skip.contains(&i) {
                    continue;
                }
                let adjust = override_offset_for_index(i, plan.instance_overrides);
                let offset = Vec3::new(
                    plan.dir.x * plan.spacing * i as f64 + adjust[0],
                    plan.dir.y * plan.spacing * i as f64 + adjust[1],
                    plan.dir.z * plan.spacing * i as f64 + adjust[2],
                );
                let (copy, handle) = if plan.mirror_alternate && i % 2 == 1 {
                    mirrored_pattern_model(source, offset, plan.dir)?
                } else {
                    let mut copy = source.model.clone();
                    translate_model(&mut copy, offset);
                    (copy, source.handle)
                };
                let new_id =
                    self.document
                        .insert(copy, handle, format!("{} (pattern {i})", source.label));
                ids.push(new_id);
                inserted += 1;
            }
        }
        Ok((ids, inserted))
    }

    fn mirror(
        &mut self,
        id: SolidId,
        point: [f64; 3],
        normal: [f64; 3],
        merge: bool,
    ) -> ApiResult<Outcome> {
        let plane_point = Point3::new(point[0], point[1], point[2]);
        let plane_normal = Vec3::new(normal[0], normal[1], normal[2]);
        if plane_normal.length() < 1e-12 {
            return Err(ApiError::InvalidArgument(
                "mirror plane normal must be non-zero".into(),
            ));
        }
        // We snapshot the source model into its own working BRepModel so the
        // kernel's mirror_solid (which appends a new solid to the model)
        // does not contaminate the source slot. We then carve the mirrored
        // solid out with a fresh BRepModel via vertex-level transform; this
        // matches translate / scale and gives one-solid-per-slot semantics.
        let (src_model, src_handle, label) = {
            let slot = self
                .document
                .get_slot(id)
                .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
            let handle = slot
                .handle
                .ok_or_else(|| ApiError::UnknownSolid(format!("{id} has no solid handle")))?;
            (slot.model.clone(), handle, slot.label.clone())
        };
        let mut work = src_model;
        let _ = mirror_solid(&mut work, src_handle, plane_point, plane_normal)?;
        // The mirrored copy is the most recently inserted solid; keep just
        // it by pulling its handle and storing the model under a new slot.
        // (work still contains both originals and mirror, but only the
        // mirror's handle is used by the slot — the originals remain
        // reachable as orphan topology, harmless for measurement.)
        let mirror_handle = work
            .solids
            .iter()
            .map(|(h, _)| h)
            .filter(|h| *h != src_handle)
            .last()
            .ok_or_else(|| ApiError::Kernel("mirror_solid produced no new solid handle".into()))?;
        let new_id = self
            .document
            .insert(work, mirror_handle, format!("{label} (mirror)"));
        if merge {
            // Fuse the original with the freshly inserted mirror via
            // boolean union. This consumes both source slots and produces
            // a single Booleaned outcome — matches FreeCAD's PartDesign
            // Mirrored feature behaviour where the result is a single body.
            return self.boolean(id, new_id, BooleanKind::Union);
        }
        Ok(Outcome::SolidCreated {
            id: new_id,
            label: format!("{label} (mirror)"),
        })
    }

    /// A2.2 feature-list mirror. Resolves every [`FeatureId`] in `features`
    /// to its history event's `primary` [`SolidId`], mirrors each, and
    /// returns a single [`Outcome::PatternCreated`] summarising the batch.
    ///
    /// When `merge` is true, each source is fused with its mirrored copy
    /// (consuming the source slot, mirroring the legacy `merge=true` path)
    /// and the resulting `SolidId` is what appears in the `ids` vector.
    /// When `merge` is false, the sources are preserved and the `ids`
    /// vector starts with the first resolved source followed by every
    /// newly inserted mirror in resolution order.
    fn mirror_features(
        &mut self,
        features: &[FeatureId],
        point: [f64; 3],
        normal: [f64; 3],
        merge: bool,
    ) -> ApiResult<Outcome> {
        let plane_point = Point3::new(point[0], point[1], point[2]);
        let plane_normal = Vec3::new(normal[0], normal[1], normal[2]);
        if plane_normal.length() < 1e-12 {
            return Err(ApiError::InvalidArgument(
                "mirror plane normal must be non-zero".into(),
            ));
        }

        // Resolve every FeatureId to its history event's primary SolidId.
        // Unknown / sentinel ids fail loudly with the offending id surfaced
        // so downstream AI/test consumers can debug. Events with `None`
        // primary (e.g. pure observer events) are silently filtered out.
        let mut sources: Vec<SolidId> = Vec::with_capacity(features.len());
        for fid in features {
            let event = self
                .document
                .feature(*fid)
                .ok_or_else(|| ApiError::InvalidArgument(format!("unknown feature id: {fid}")))?;
            if let Some(primary) = event.primary {
                sources.push(primary);
            }
        }
        if sources.is_empty() {
            return Err(ApiError::InvalidArgument(
                "Mirror::features resolved to no mirrorable solids".into(),
            ));
        }

        // `pattern_id` is the first resolved source; the `ids` vector
        // mirrors `LinearPattern`'s contract (original at index 0 when
        // preserved, new instances appended in creation order).
        let first_source = sources[0];
        let mut ids: Vec<SolidId> = Vec::with_capacity(sources.len() * 2);
        let mut new_instance_count: u32 = 0;
        if !merge {
            ids.push(first_source);
        }

        for src_id in &sources {
            let (src_model, src_handle, label) = {
                let slot = self
                    .document
                    .get_slot(*src_id)
                    .ok_or_else(|| ApiError::UnknownSolid(format!("{src_id}")))?;
                let handle = slot.handle.ok_or_else(|| {
                    ApiError::UnknownSolid(format!("{src_id} has no solid handle"))
                })?;
                (slot.model.clone(), handle, slot.label.clone())
            };
            let mut work = src_model;
            let _ = mirror_solid(&mut work, src_handle, plane_point, plane_normal)?;
            let mirror_handle = work
                .solids
                .iter()
                .map(|(h, _)| h)
                .filter(|h| *h != src_handle)
                .last()
                .ok_or_else(|| {
                    ApiError::Kernel("mirror_solid produced no new solid handle".into())
                })?;
            let new_id = self
                .document
                .insert(work, mirror_handle, format!("{label} (mirror)"));
            if merge {
                // Each merge consumes the source slot and its mirror, leaving
                // a single fused id in their place. That id becomes the
                // "instance" recorded in the pattern.
                let fused = self.boolean(*src_id, new_id, BooleanKind::Union)?;
                let fused_id = match fused {
                    Outcome::Booleaned { result, .. } => result,
                    other => {
                        return Err(ApiError::Kernel(format!(
                            "mirror_features expected Booleaned, got {other:?}"
                        )));
                    }
                };
                ids.push(fused_id);
            } else {
                ids.push(new_id);
            }
            new_instance_count += 1;
        }

        let instance_count = ids.len() as u32;
        Ok(Outcome::PatternCreated {
            pattern_id: first_source,
            instance_count,
            total_features: new_instance_count,
            ids,
        })
    }
}

#[derive(Clone)]
struct PatternSource {
    id: SolidId,
    model: BRepModel,
    handle: Handle<SolidData>,
    label: String,
}

struct LinearPatternParams<'a> {
    direction: [f64; 3],
    spacing: f64,
    count: u32,
    skip_instances: &'a [u32],
    instance_overrides: &'a [InstanceOverride],
    mirror_alternate: bool,
}

struct LinearPatternPlan<'a> {
    dir: Vec3,
    spacing: f64,
    count: u32,
    skip: std::collections::HashSet<u32>,
    instance_overrides: &'a [InstanceOverride],
    mirror_alternate: bool,
}

#[derive(Copy, Clone)]
enum BooleanKind {
    Union,
    Subtract,
    Intersect,
}

fn first_solid_handle(model: &BRepModel) -> Option<Handle<SolidData>> {
    model.solids.iter().next().map(|(h, _)| h)
}

fn validate_extrude_distance(distance: f64) -> ApiResult<()> {
    if distance <= 0.0 {
        return Err(ApiError::InvalidArgument(format!(
            "extrude distance must be > 0, got {distance}"
        )));
    }
    Ok(())
}

fn profile_centroid(profile: &[[f64; 3]]) -> Point3 {
    let inv = 1.0 / profile.len() as f64;
    let mut sum = [0.0; 3];
    for point in profile {
        sum[0] += point[0];
        sum[1] += point[1];
        sum[2] += point[2];
    }
    Point3::new(sum[0] * inv, sum[1] * inv, sum[2] * inv)
}

fn face_plane(model: &BRepModel, face_index: u32) -> ApiResult<(Point3, Vec3)> {
    let (face_handle, _) = model.faces.iter().nth(face_index as usize).ok_or_else(|| {
        ApiError::InvalidArgument(format!("UpToFace face_index {face_index} out of range"))
    })?;
    let vertex_handles = model.vertices_of_face(face_handle)?;
    if vertex_handles.len() < 3 {
        return Err(ApiError::InvalidArgument(
            "UpToFace target face has fewer than 3 vertices".into(),
        ));
    }

    let mut points = Vec::with_capacity(vertex_handles.len());
    for vertex_handle in vertex_handles {
        let vertex = model.vertices.get(vertex_handle).ok_or_else(|| {
            ApiError::Kernel("UpToFace target face references a missing vertex".into())
        })?;
        points.push(vertex.point);
    }

    let inv = 1.0 / points.len() as f64;
    let mut centroid = Point3::ORIGIN;
    for point in &points {
        centroid.x += point.x * inv;
        centroid.y += point.y * inv;
        centroid.z += point.z * inv;
    }

    let mut normal = Vec3::ZERO;
    for i in 0..points.len() {
        let current = points[i];
        let next = points[(i + 1) % points.len()];
        normal.x += (current.y - next.y) * (current.z + next.z);
        normal.y += (current.z - next.z) * (current.x + next.x);
        normal.z += (current.x - next.x) * (current.y + next.y);
    }
    if normal.length() < 1e-12 {
        normal = fallback_face_normal(&points)?;
    }
    let normal = normal.normalized().ok_or_else(|| {
        ApiError::InvalidArgument("UpToFace target face plane is degenerate".into())
    })?;
    Ok((centroid, normal))
}

fn fallback_face_normal(points: &[Point3]) -> ApiResult<Vec3> {
    let origin = points[0];
    for i in 1..points.len() {
        for j in (i + 1)..points.len() {
            let normal = (points[i] - origin).cross(points[j] - origin);
            if normal.length() >= 1e-12 {
                return Ok(normal);
            }
        }
    }
    Err(ApiError::InvalidArgument(
        "UpToFace target face plane is degenerate".into(),
    ))
}

fn combined_pattern_skip_set(
    count: u32,
    skip_instances: &[u32],
    instance_overrides: &[InstanceOverride],
) -> std::collections::HashSet<u32> {
    let mut skip: std::collections::HashSet<u32> = skip_instances
        .iter()
        .copied()
        .filter(|i| *i < count)
        .collect();
    skip.extend(
        instance_overrides
            .iter()
            .filter(|ov| ov.suppress && ov.index < count)
            .map(|ov| ov.index),
    );
    skip
}

fn linear_pattern_plan(params: LinearPatternParams<'_>) -> ApiResult<LinearPatternPlan<'_>> {
    if params.count < 2 {
        return Err(ApiError::InvalidArgument(format!(
            "linear_pattern count must be ≥ 2, got {}",
            params.count
        )));
    }
    let dir_v = Vec3::new(
        params.direction[0],
        params.direction[1],
        params.direction[2],
    );
    let dir = dir_v.normalized().ok_or_else(|| {
        ApiError::InvalidArgument("linear_pattern direction must be non-zero".into())
    })?;
    let skip = combined_pattern_skip_set(
        params.count,
        params.skip_instances,
        params.instance_overrides,
    );
    if skip.len() as u32 == params.count {
        return Err(ApiError::InvalidArgument(
            "linear_pattern skip_instances would suppress every position".into(),
        ));
    }
    Ok(LinearPatternPlan {
        dir,
        spacing: params.spacing,
        count: params.count,
        skip,
        instance_overrides: params.instance_overrides,
        mirror_alternate: params.mirror_alternate,
    })
}

fn override_offset_for_index(index: u32, instance_overrides: &[InstanceOverride]) -> [f64; 3] {
    let mut offset = [0.0; 3];
    for ov in instance_overrides {
        if ov.index == index && !ov.suppress {
            offset = ov.offset_adjust;
        }
    }
    offset
}

fn translate_model(model: &mut BRepModel, offset: Vec3) {
    for (_h, vertex) in model.vertices.iter_mut() {
        vertex.point = Point3::new(
            vertex.point.x + offset.x,
            vertex.point.y + offset.y,
            vertex.point.z + offset.z,
        );
    }
}

fn mirrored_pattern_model(
    source: &PatternSource,
    offset: Vec3,
    plane_normal: Vec3,
) -> ApiResult<(BRepModel, Handle<SolidData>)> {
    let mut work = source.model.clone();
    translate_model(&mut work, offset);
    let plane_point = Point3::new(offset.x, offset.y, offset.z);
    let result = mirror_solid(&mut work, source.handle, plane_point, plane_normal)?;
    clone_solid_to_clean_model(&work, result.solid)
}

fn clone_solid_to_clean_model(
    src: &BRepModel,
    solid: Handle<SolidData>,
) -> ApiResult<(BRepModel, Handle<SolidData>)> {
    let solid_data = src
        .solids
        .get(solid)
        .ok_or_else(|| ApiError::Kernel("pattern mirror result solid is missing".into()))?;
    let mut dst = BRepModel::new();
    let mut vertex_map = std::collections::HashMap::new();
    let mut new_shells = Vec::with_capacity(solid_data.shells.len());

    for shell_handle in &solid_data.shells {
        let shell = src
            .shells
            .get(*shell_handle)
            .ok_or_else(|| ApiError::Kernel("pattern mirror result shell is missing".into()))?;
        let mut new_faces = Vec::with_capacity(shell.faces.len());
        for face_handle in &shell.faces {
            let vertices = src.vertices_of_face(*face_handle)?;
            if vertices.len() < 3 {
                return Err(ApiError::Kernel(
                    "pattern mirror result face has fewer than 3 vertices".into(),
                ));
            }
            let mut new_vertices = Vec::with_capacity(vertices.len());
            for vertex_handle in vertices {
                let key = vertex_handle.index();
                let new_vertex = if let Some(mapped) = vertex_map.get(&key) {
                    *mapped
                } else {
                    let vertex = src.vertices.get(vertex_handle).ok_or_else(|| {
                        ApiError::Kernel("pattern mirror result vertex is missing".into())
                    })?;
                    let mapped = dst.add_vertex(vertex.point);
                    vertex_map.insert(key, mapped);
                    mapped
                };
                new_vertices.push(new_vertex);
            }

            let mut half_edges = Vec::with_capacity(new_vertices.len());
            for i in 0..new_vertices.len() {
                let next = (i + 1) % new_vertices.len();
                let (_, half_edge, _) = dst.add_edge(new_vertices[i], new_vertices[next]);
                half_edges.push(half_edge);
            }
            let loop_handle = dst.make_loop(&half_edges)?;
            new_faces.push(dst.make_face(loop_handle));
        }
        new_shells.push(dst.make_shell(&new_faces));
    }

    let new_solid = dst.make_solid(&new_shells);
    Ok((dst, new_solid))
}

fn create_cone_model(radius: f64, height: f64, top_radius: f64) -> ApiResult<BRepModel> {
    if top_radius <= 0.0 {
        // Pure cone — reuse the existing quick helper to match the pre-A3.3
        // codepath byte-for-byte (segments = 64).
        return Ok(quick_cone(radius, height)?);
    }
    if radius <= 0.0 {
        return Err(ApiError::InvalidArgument(format!(
            "radius must be > 0, got {radius}"
        )));
    }
    if height <= 0.0 {
        return Err(ApiError::InvalidArgument(format!(
            "height must be > 0, got {height}"
        )));
    }
    let mut model = BRepModel::new();
    make_cone(&mut model, Point3::ORIGIN, radius, top_radius, height, 64)?;
    Ok(model)
}

fn slot_mut(doc: &mut Document, id: SolidId) -> ApiResult<&mut SolidSlot> {
    doc.get_slot_mut(id)
        .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))
}

fn history_description(cmd: &Command, outcome: &Outcome) -> String {
    match (cmd, outcome) {
        (Command::CreateBox { dx, dy, dz }, _) => format!("Box {dx}×{dy}×{dz}"),
        (Command::CreateCylinder { radius, height }, _) => {
            format!("Cylinder r={radius} h={height}")
        }
        (Command::CreateSphere { radius }, _) => format!("Sphere r={radius}"),
        (
            Command::CreateCone {
                radius,
                height,
                top_radius,
            },
            _,
        ) => {
            if *top_radius > 0.0 {
                format!("Frustum r={radius} top={top_radius} h={height}")
            } else {
                format!("Cone r={radius} h={height}")
            }
        }
        (
            Command::CreateTorus {
                major_radius,
                minor_radius,
            },
            _,
        ) => format!("Torus R={major_radius} r={minor_radius}"),
        (Command::BooleanUnion { lhs, rhs }, _) => format!("Union {lhs} ∪ {rhs}"),
        (Command::BooleanSubtract { lhs, rhs }, _) => format!("Subtract {lhs} − {rhs}"),
        (Command::BooleanIntersect { lhs, rhs }, _) => format!("Intersect {lhs} ∩ {rhs}"),
        (Command::Translate { id, dx, dy, dz }, _) => {
            format!("Translate {id} ({dx}, {dy}, {dz})")
        }
        (Command::Scale { id, factor }, _) => format!("Scale {id} ×{factor}"),
        (Command::ScaleNonUniform { id, factors, .. }, _) => {
            format!(
                "ScaleNonUniform {id} [{},{},{}]",
                factors[0], factors[1], factors[2]
            )
        }
        (Command::CenterOnOrigin { id }, _) => format!("CenterOnOrigin {id}"),
        (Command::AlignTo { id, target_id }, _) => format!("AlignTo {id} -> {target_id}"),
        (Command::ScaleToFit { id, target_size }, _) => {
            format!("ScaleToFit {id} target_size={target_size}")
        }
        (Command::TranslateTo { id, point }, _) => {
            format!(
                "TranslateTo {id} -> [{}, {}, {}]",
                point[0], point[1], point[2]
            )
        }
        (Command::Rename { id, label }, _) => format!("Rename {id} → {label:?}"),
        (Command::DeleteSolid { id }, _) => format!("Delete {id}"),
        (
            Command::Extrude {
                profile,
                kind: ExtrudeKind::ThroughAll,
                ..
            },
            _,
        ) => format!("Extrude ThroughAll ({} pts)", profile.len()),
        (
            Command::Extrude {
                profile,
                kind:
                    ExtrudeKind::UpToFace {
                        face_solid,
                        face_index,
                    },
                ..
            },
            _,
        ) => format!(
            "Extrude UpToFace -> {face_solid} face {face_index} ({} pts)",
            profile.len()
        ),
        (
            Command::Extrude {
                profile, distance, ..
            },
            _,
        ) => format!("Extrude ({} pts, h={distance})", profile.len()),
        (
            Command::LinearPattern {
                features, count, ..
            },
            _,
        ) if !features.is_empty() => {
            format!("LinearPattern {} features ×{count}", features.len())
        }
        (Command::LinearPattern { id, count, .. }, Outcome::PatternCreated { ids, .. }) => {
            format!("LinearPattern {id} ×{count} ({} solids)", ids.len())
        }
        (Command::LinearPattern { id, count, .. }, _) => {
            format!("LinearPattern {id} ×{count}")
        }
        (
            Command::Mirror {
                normal, features, ..
            },
            Outcome::PatternCreated { total_features, .. },
        ) if !features.is_empty() => format!(
            "Mirror {} features across ({},{},{}) -> {} solids",
            features.len(),
            normal[0],
            normal[1],
            normal[2],
            total_features,
        ),
        (
            Command::Mirror {
                features, normal, ..
            },
            _,
        ) if !features.is_empty() => format!(
            "Mirror {} features across ({},{},{})",
            features.len(),
            normal[0],
            normal[1],
            normal[2],
        ),
        (Command::Mirror { id, .. }, _) => format!("Mirror {id}"),
        (Command::Measure { id }, _) => format!("Measure {id}"),
        (Command::Validate, _) => "Validate".into(),
        (Command::ListSolids, _) => "ListSolids".into(),
        (Command::FindByLabel { query }, _) => format!("FindByLabel '{query}'"),
        (Command::HistoryEvents, _) => "HistoryEvents".into(),
        (Command::Stats, _) => "Stats".into(),
        (Command::Bounds { id }, _) => format!("Bounds {id}"),
        (Command::Distance { id_a, id_b }, _) => format!("Distance {id_a} <-> {id_b}"),
        (Command::Volume { id }, _) => format!("Volume {id}"),
        (Command::SurfaceArea { id }, _) => format!("SurfaceArea {id}"),
        (Command::Centroid { id }, _) => format!("Centroid {id}"),
        (Command::IntersectsAabb { id_a, id_b }, _) => format!("IntersectsAabb {id_a} <-> {id_b}"),
        (Command::Exists { id }, _) => format!("Exists {id}"),
        (Command::Diagonal { id }, _) => format!("Diagonal {id}"),
        (Command::AabbCenter { id }, _) => format!("AabbCenter {id}"),
        (Command::AabbVolume { id }, _) => format!("AabbVolume {id}"),
        (Command::ContainsAabb { id_outer, id_inner }, _) => {
            format!("ContainsAabb {id_outer} ⊇ {id_inner}")
        }
        (Command::AabbCorners { id }, _) => format!("AabbCorners {id}"),
        (Command::SolidLabel { id }, _) => format!("SolidLabel {id}"),
        (Command::IsEmpty, _) => "IsEmpty".to_string(),
        (Command::AabbSurfaceArea { id }, _) => format!("AabbSurfaceArea {id}"),
        (Command::SolidCount, _) => "SolidCount".to_string(),
        (Command::HistoryCount, _) => "HistoryCount".to_string(),
        (Command::HasLabel { query }, _) => format!("HasLabel '{query}'"),
        (Command::SolidIds, _) => "SolidIds".to_string(),
        (Command::AabbExtents { id }, _) => format!("AabbExtents {id:?}"),
        (Command::AabbLongestAxis { id }, _) => format!("AabbLongestAxis {id:?}"),
        (Command::AabbShortestAxis { id }, _) => format!("AabbShortestAxis {id:?}"),
        (Command::AabbAspectRatio { id }, _) => format!("AabbAspectRatio {id:?}"),
        (Command::IsCubic { id }, _) => format!("IsCubic {id:?}"),
        (Command::IsSquareXy { id }, _) => format!("IsSquareXy {id:?}"),
        (Command::HistoryDescription { index }, _) => format!("HistoryDescription {index}"),
        (Command::IsSquareYz { id }, _) => format!("IsSquareYz {id:?}"),
        (Command::IsSquareXz { id }, _) => format!("IsSquareXz {id:?}"),
        (Command::OperationCount { op_name }, _) => format!("OperationCount '{op_name}'"),
        (Command::LastOperation, _) => "LastOperation".to_string(),
        (Command::HasOperation { op_name }, _) => format!("HasOperation '{op_name}'"),
        (Command::FirstOperation, _) => "FirstOperation".to_string(),
        (Command::Duplicate { id }, _) => format!("Duplicate {id}"),
        (Command::Rotate { id, angle_rad, .. }, _) => {
            format!("Rotate {id} ({angle_rad} rad)")
        }
        (Command::NewDocument, _) => "New document".into(),
        (Command::Noop, _) => "Noop".into(),
    }
}
