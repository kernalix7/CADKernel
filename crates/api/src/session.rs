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
use cadkernel_modeling::body::{Body, FeatureKind};
use cadkernel_modeling::measure::solid_mass_properties;
use cadkernel_modeling::primitives::{make_cone, make_helix};
use cadkernel_modeling::quick::{
    quick_box, quick_cone, quick_cylinder, quick_intersect, quick_sphere, quick_subtract,
    quick_torus, quick_union,
};
use cadkernel_modeling::{extrude, mirror_solid};
use cadkernel_topology::naming::SegmentKind;
use cadkernel_topology::{BRepModel, FaceData, Handle, SolidData, VertexData};
use serde::{Deserialize, Serialize};

use crate::command::{
    AxisRef, BodyId, ChamferMode, ChamferSpec, Command, DraftDirection, DraftSpec, EdgeRef,
    ExtrudeKind, FaceRef, FeatureSpec, FilletSpec, GrooveSpec, HelixSpec, HoleKind, HoleSpec,
    EntityId, InstanceOverride, LoftMode, LoftSpec, PadDirection, PadSpec, PadType, PlaneRef,
    PocketSpec, PocketType, RevolveSpec, ShellMode, ShellSpec, SketchConstraint, SketchEdit,
    SketchEntity, SketchId, SketchRef, SweepMode, SweepSpec,
};
use crate::document::{Document, FeatureId, HistoryEvent, PersistedSketch, SolidId, SolidSlot};
use crate::outcome::{Outcome, Plane};
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
            Command::Pad {
                sketch,
                distance,
                direction,
                symmetric,
                type_,
            } => self.dispatch_pad(sketch, *distance, *direction, *symmetric, *type_),
            Command::Pocket {
                sketch,
                distance,
                through_all,
                type_,
            } => self.dispatch_pocket(sketch, *distance, *through_all, *type_),
            Command::Revolve {
                sketch,
                axis,
                angle_rad,
                symmetric,
            } => self.dispatch_revolve(sketch, axis, *angle_rad, *symmetric),
            Command::Groove {
                sketch,
                axis,
                angle_rad,
            } => self.dispatch_groove(sketch, axis, *angle_rad),
            Command::Hole {
                face,
                position,
                radius,
                depth,
                through_all,
                kind,
            } => self.dispatch_hole(face, *position, *radius, *depth, *through_all, *kind),
            Command::Sweep {
                profile_sketch,
                path_sketch,
                mode,
            } => self.dispatch_sweep(profile_sketch, path_sketch, *mode),
            Command::Loft {
                profiles,
                mode,
                ruled,
                closed,
            } => self.dispatch_loft(profiles, *mode, *ruled, *closed),
            Command::Helix {
                axis,
                radius,
                pitch,
                height,
                turns,
                cone_angle,
            } => self.dispatch_helix(axis, *radius, *pitch, *height, *turns, *cone_angle),
            Command::Fillet {
                edges,
                radius,
                variable,
            } => self.dispatch_fillet(edges, *radius, variable.as_ref()),
            Command::Chamfer {
                edges,
                distance,
                mode,
            } => self.dispatch_chamfer(edges, *distance, *mode),
            Command::Shell {
                solid,
                removed_faces,
                thickness,
                mode,
            } => self.dispatch_shell(*solid, removed_faces, *thickness, *mode),
            Command::Draft {
                faces,
                neutral_plane,
                angle_rad,
                direction,
            } => self.dispatch_draft(faces, neutral_plane, *angle_rad, *direction),
            Command::CreateSketch { plane, name } => self.dispatch_create_sketch(plane, name),
            Command::EditSketch { sketch, edits } => self.dispatch_edit_sketch(*sketch, edits),
            Command::DeleteSketch { sketch } => self.dispatch_delete_sketch(*sketch),
            Command::MapSketchToFace { sketch, face } => {
                self.dispatch_map_sketch_to_face(*sketch, face)
            }
            Command::CreateBody { name, base_plane } => self.dispatch_create_body(name, base_plane),
            Command::SetTip { body, feature } => self.dispatch_set_tip(*body, *feature),
            Command::SuppressFeature {
                feature,
                suppressed,
            } => self.dispatch_suppress_feature(*feature, *suppressed),
            Command::ReorderFeature { from, to_position } => {
                self.dispatch_reorder_feature(*from, *to_position)
            }
            Command::RecomputeBody { body } => self.recompute_body(*body),
            Command::EditFeature { feature, new_spec } => {
                self.dispatch_edit_feature(*feature, new_spec.clone())
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

    fn dispatch_pad(
        &mut self,
        sketch: &SketchRef,
        distance: f64,
        direction: PadDirection,
        symmetric: bool,
        type_: PadType,
    ) -> ApiResult<Outcome> {
        let body_id = self.active_solid_for_feature("Pad")?;
        let profile = self.resolve_sketch_profile(sketch)?;
        let dir_vec = match direction {
            PadDirection::Normal => Vec3::Z,
            PadDirection::Reversed => -Vec3::Z,
            PadDirection::TwoSided => Vec3::Z,
        };
        let dist = if symmetric || matches!(direction, PadDirection::TwoSided) {
            distance * 0.5
        } else {
            distance
        };
        let (base_model, base_handle) = self.base_model_and_handle(body_id)?;
        let result =
            cadkernel_modeling::features::pad(&base_model, base_handle, &profile, dir_vec, dist)?;
        let spec = FeatureSpec::Pad(PadSpec {
            sketch: sketch.clone(),
            distance,
            direction,
            symmetric,
            type_,
        });
        self.replace_feature_body(
            body_id,
            result.model,
            result.solid,
            "Pad",
            spec.spec_kind(),
            feature_spec_value(&spec),
        )
    }

    fn dispatch_pocket(
        &mut self,
        sketch: &SketchRef,
        distance: f64,
        through_all: bool,
        type_: PocketType,
    ) -> ApiResult<Outcome> {
        let body_id = self.active_solid_for_feature("Pocket")?;
        let profile = self.resolve_sketch_profile(sketch)?;
        let (base_model, base_handle) = self.base_model_and_handle(body_id)?;
        let result = cadkernel_modeling::features::pocket(
            &base_model,
            base_handle,
            &profile,
            Vec3::Z,
            distance,
        )?;
        let spec = FeatureSpec::Pocket(PocketSpec {
            sketch: sketch.clone(),
            distance,
            through_all,
            type_,
        });
        self.replace_feature_body(
            body_id,
            result.model,
            result.solid,
            "Pocket",
            spec.spec_kind(),
            feature_spec_value(&spec),
        )
    }

    fn dispatch_revolve(
        &mut self,
        sketch: &SketchRef,
        axis: &AxisRef,
        angle_rad: f64,
        _symmetric: bool,
    ) -> ApiResult<Outcome> {
        let _body_id = self.active_solid_for_feature("Revolve")?;
        let profile = self.resolve_sketch_profile(sketch)?;
        let (axis_origin, axis_dir) = self.resolve_axis(axis)?;
        let mut model = BRepModel::new();
        let result = cadkernel_modeling::features::revolve(
            &mut model,
            &profile,
            axis_origin,
            axis_dir,
            angle_rad,
            64,
        )?;
        let solid = self.document.insert(model, result.solid, "Revolve");
        let spec = FeatureSpec::Revolve(RevolveSpec {
            sketch: sketch.clone(),
            axis: axis.clone(),
            angle_rad,
            symmetric: _symmetric,
        });
        let body = self.append_feature_to_active_body(AppendFeatureArgs {
            solid,
            label: "Revolve",
            kind: FeatureKind::Revolve,
            spec_kind: spec.spec_kind(),
            spec_value: feature_spec_value(&spec),
            handle: result.solid,
            feature_id: self.predicted_feature_id(),
        });
        Ok(Outcome::FeatureAdded {
            feature_id: self.predicted_feature_id(),
            body,
            solid,
        })
    }

    fn dispatch_groove(
        &mut self,
        sketch: &SketchRef,
        axis: &AxisRef,
        angle_rad: f64,
    ) -> ApiResult<Outcome> {
        let body_id = self.active_solid_for_feature("Groove")?;
        let profile = self.resolve_sketch_profile(sketch)?;
        let (axis_origin, axis_dir) = self.resolve_axis(axis)?;
        let (base_model, base_handle) = self.base_model_and_handle(body_id)?;
        let result = cadkernel_modeling::features::groove(
            &base_model,
            base_handle,
            &profile,
            axis_origin,
            axis_dir,
            angle_rad,
            64,
        )?;
        let spec = FeatureSpec::Groove(GrooveSpec {
            sketch: sketch.clone(),
            axis: axis.clone(),
            angle_rad,
        });
        self.replace_feature_body(
            body_id,
            result.model,
            result.solid,
            "Groove",
            spec.spec_kind(),
            feature_spec_value(&spec),
        )
    }

    fn dispatch_hole(
        &mut self,
        face: &FaceRef,
        position: [f64; 2],
        radius: f64,
        depth: f64,
        _through_all: bool,
        _kind: HoleKind,
    ) -> ApiResult<Outcome> {
        let body_id = self.active_solid_for_feature("Hole")?;
        let (_face_solid, _face_handle) = self.resolve_face_handle(face)?;
        let center = Point3::new(position[0], position[1], 0.0);
        let (base_model, base_handle) = self.base_model_and_handle(body_id)?;
        let result = cadkernel_modeling::features::hole(
            &base_model,
            base_handle,
            center,
            Vec3::Z,
            radius,
            depth,
            32,
        )?;
        let spec = FeatureSpec::Hole(HoleSpec {
            face: face.clone(),
            position,
            radius,
            depth,
            through_all: _through_all,
            kind: _kind,
        });
        self.replace_feature_body(
            body_id,
            result.model,
            result.solid,
            "Hole",
            spec.spec_kind(),
            feature_spec_value(&spec),
        )
    }

    fn dispatch_sweep(
        &mut self,
        profile_sketch: &SketchRef,
        path_sketch: &SketchRef,
        _mode: SweepMode,
    ) -> ApiResult<Outcome> {
        let body_id = self.active_solid_for_feature("Sweep")?;
        let profile = self.resolve_sketch_profile(profile_sketch)?;
        let path = self.resolve_sketch_profile(path_sketch)?;
        let (mut model, _) = self.base_model_and_handle(body_id)?;
        let result = cadkernel_modeling::features::sweep(&mut model, &profile, &path)?;
        let spec = FeatureSpec::Sweep(SweepSpec {
            profile_sketch: profile_sketch.clone(),
            path_sketch: path_sketch.clone(),
            mode: _mode,
        });
        self.replace_feature_body(
            body_id,
            model,
            result.solid,
            "Sweep",
            spec.spec_kind(),
            feature_spec_value(&spec),
        )
    }

    fn dispatch_loft(
        &mut self,
        profiles: &[SketchRef],
        _mode: LoftMode,
        _ruled: bool,
        _closed: bool,
    ) -> ApiResult<Outcome> {
        let body_id = self.active_solid_for_feature("Loft")?;
        let mut resolved = Vec::with_capacity(profiles.len());
        for profile in profiles {
            resolved.push(self.resolve_sketch_profile(profile)?);
        }
        let profile_refs: Vec<&[Point3]> = resolved.iter().map(Vec::as_slice).collect();
        let (mut model, _) = self.base_model_and_handle(body_id)?;
        let result = cadkernel_modeling::features::loft(&mut model, &profile_refs)?;
        let spec = FeatureSpec::Loft(LoftSpec {
            profiles: profiles.to_vec(),
            mode: _mode,
            ruled: _ruled,
            closed: _closed,
        });
        self.replace_feature_body(
            body_id,
            model,
            result.solid,
            "Loft",
            spec.spec_kind(),
            feature_spec_value(&spec),
        )
    }

    fn dispatch_helix(
        &mut self,
        axis: &AxisRef,
        radius: f64,
        pitch: f64,
        height: f64,
        turns: f64,
        cone_angle: f64,
    ) -> ApiResult<Outcome> {
        let (center, _axis_dir) = self.resolve_axis(axis)?;
        let tube_radius = 0.5_f64.min(radius * 0.25);
        let spec = FeatureSpec::Helix(HelixSpec {
            axis: axis.clone(),
            radius,
            pitch,
            height,
            turns,
            cone_angle,
        });
        if self.document.solid_count() == 0 {
            let mut model = BRepModel::new();
            let result = make_helix(
                &mut model,
                center,
                radius,
                pitch,
                turns,
                tube_radius,
                32,
                16,
            )?;
            let id = self.document.insert(model, result.solid, "Helix");
            self.append_feature_to_active_body(AppendFeatureArgs {
                solid: id,
                label: "Helix",
                kind: FeatureKind::Helix,
                spec_kind: spec.spec_kind(),
                spec_value: feature_spec_value(&spec),
                handle: result.solid,
                feature_id: self.predicted_feature_id(),
            });
            return Ok(Outcome::SolidCreated {
                id,
                label: "Helix".into(),
            });
        }

        let body_id = self.active_solid_for_feature("Helix")?;
        let (base_model, base_handle) = self.base_model_and_handle(body_id)?;
        let model = cadkernel_modeling::features::additive_helix(
            &base_model,
            base_handle,
            center,
            radius,
            pitch,
            turns,
            tube_radius,
            32,
            16,
        )?;
        let handle = first_solid_handle(&model)
            .ok_or_else(|| ApiError::Kernel("additive_helix produced no solid".into()))?;
        self.replace_feature_body(
            body_id,
            model,
            handle,
            "Helix",
            spec.spec_kind(),
            feature_spec_value(&spec),
        )
    }

    fn dispatch_fillet(
        &mut self,
        edges: &[EdgeRef],
        radius: f64,
        variable: Option<&crate::command::VariableRadius>,
    ) -> ApiResult<Outcome> {
        if variable.is_some_and(|v| !v.samples.is_empty()) {
            return Err(ApiError::InvalidArgument(
                "variable radius not yet supported".into(),
            ));
        }
        let body_id = self.active_solid_for_feature("Fillet")?;
        let resolved = self.resolve_edge_refs(edges)?;
        let slot = slot_mut(&mut self.document, body_id)?;
        let solid = slot
            .handle
            .ok_or_else(|| ApiError::UnknownSolid(format!("{body_id} has no solid handle")))?;
        let result =
            cadkernel_modeling::features::fillet_edges(&mut slot.model, solid, &resolved, radius)?;
        slot.handle = Some(result.solid);
        let spec = FeatureSpec::Fillet(FilletSpec {
            edges: edges.to_vec(),
            radius,
            variable: variable.cloned(),
        });
        let feature_id = self.predicted_feature_id();
        let body = self.append_feature_to_active_body(AppendFeatureArgs {
            solid: body_id,
            label: "Fillet",
            kind: FeatureKind::Fillet,
            spec_kind: spec.spec_kind(),
            spec_value: feature_spec_value(&spec),
            handle: result.solid,
            feature_id,
        });
        Ok(Outcome::FeatureAdded {
            feature_id,
            body,
            solid: body_id,
        })
    }

    fn dispatch_chamfer(
        &mut self,
        edges: &[EdgeRef],
        distance: f64,
        _mode: ChamferMode,
    ) -> ApiResult<Outcome> {
        let body_id = self.active_solid_for_feature("Chamfer")?;
        let resolved = self.resolve_edge_refs(edges)?;
        let slot = slot_mut(&mut self.document, body_id)?;
        let solid = slot
            .handle
            .ok_or_else(|| ApiError::UnknownSolid(format!("{body_id} has no solid handle")))?;
        let result = cadkernel_modeling::features::chamfer_edges(
            &mut slot.model,
            solid,
            &resolved,
            distance,
        )?;
        slot.handle = Some(result.solid);
        let spec = FeatureSpec::Chamfer(ChamferSpec {
            edges: edges.to_vec(),
            distance,
            mode: _mode,
        });
        let feature_id = self.predicted_feature_id();
        let body = self.append_feature_to_active_body(AppendFeatureArgs {
            solid: body_id,
            label: "Chamfer",
            kind: FeatureKind::Chamfer,
            spec_kind: spec.spec_kind(),
            spec_value: feature_spec_value(&spec),
            handle: result.solid,
            feature_id,
        });
        Ok(Outcome::FeatureAdded {
            feature_id,
            body,
            solid: body_id,
        })
    }

    fn dispatch_shell(
        &mut self,
        solid_id: SolidId,
        removed_faces: &[FaceRef],
        thickness: f64,
        _mode: ShellMode,
    ) -> ApiResult<Outcome> {
        let _ = self
            .document
            .get_slot(solid_id)
            .ok_or_else(|| ApiError::UnknownSolid(format!("{solid_id}")))?;
        let faces = self.resolve_face_refs(removed_faces)?;
        let slot = slot_mut(&mut self.document, solid_id)?;
        let solid = slot
            .handle
            .ok_or_else(|| ApiError::UnknownSolid(format!("{solid_id} has no solid handle")))?;
        let result =
            cadkernel_modeling::features::shell_solid(&mut slot.model, solid, &faces, thickness)?;
        slot.handle = Some(result.solid);
        let spec = FeatureSpec::Shell(ShellSpec {
            solid: solid_id,
            removed_faces: removed_faces.to_vec(),
            thickness,
            mode: _mode,
        });
        let feature_id = self.predicted_feature_id();
        let body = self.append_feature_to_active_body(AppendFeatureArgs {
            solid: solid_id,
            label: "Shell",
            kind: FeatureKind::Shell,
            spec_kind: spec.spec_kind(),
            spec_value: feature_spec_value(&spec),
            handle: result.solid,
            feature_id,
        });
        Ok(Outcome::FeatureAdded {
            feature_id,
            body,
            solid: solid_id,
        })
    }

    fn dispatch_draft(
        &mut self,
        faces: &[FaceRef],
        neutral_plane: &FaceRef,
        angle_rad: f64,
        direction: DraftDirection,
    ) -> ApiResult<Outcome> {
        let solid_id = neutral_plane.solid;
        let _neutral = self.resolve_face_handle(neutral_plane)?;
        let resolved = self.resolve_face_refs(faces)?;
        let pull_direction = match direction {
            DraftDirection::Pull => Vec3::Z,
            DraftDirection::Push => -Vec3::Z,
        };
        let slot = slot_mut(&mut self.document, solid_id)?;
        let solid = slot
            .handle
            .ok_or_else(|| ApiError::UnknownSolid(format!("{solid_id} has no solid handle")))?;
        let result = cadkernel_modeling::features::draft_faces(
            &mut slot.model,
            solid,
            &resolved,
            pull_direction,
            angle_rad,
        )?;
        slot.handle = Some(result.solid);
        let spec = FeatureSpec::Draft(DraftSpec {
            faces: faces.to_vec(),
            neutral_plane: neutral_plane.clone(),
            angle_rad,
            direction,
        });
        let feature_id = self.predicted_feature_id();
        let body = self.append_feature_to_active_body(AppendFeatureArgs {
            solid: solid_id,
            label: "Draft",
            kind: FeatureKind::Draft,
            spec_kind: spec.spec_kind(),
            spec_value: feature_spec_value(&spec),
            handle: result.solid,
            feature_id,
        });
        Ok(Outcome::FeatureAdded {
            feature_id,
            body,
            solid: solid_id,
        })
    }

    fn dispatch_create_sketch(&mut self, plane_ref: &PlaneRef, name: &str) -> ApiResult<Outcome> {
        let plane = plane_from_ref(plane_ref)?;
        let sketch_name = if name.is_empty() {
            format!("Sketch{}", self.document.sketch_count() + 1)
        } else {
            name.to_string()
        };
        let id = self
            .document
            .push_sketch(PersistedSketch::new(sketch_name, plane));
        Ok(Outcome::SketchCreated {
            sketch_id: id,
            plane,
        })
    }

    fn dispatch_edit_sketch(
        &mut self,
        sketch_id: SketchId,
        edits: &[SketchEdit],
    ) -> ApiResult<Outcome> {
        {
            let sketch = self.document.sketch_mut(sketch_id).ok_or_else(|| {
                ApiError::InvalidArgument(format!("Track 6 sketch {sketch_id:?} not found"))
            })?;
            for edit in edits {
                apply_sketch_edit(sketch, edit)?;
            }
        }
        self.document.set_active_sketch(sketch_id);
        self.recompute_sketch_dependents(sketch_id)
    }

    fn dispatch_delete_sketch(&mut self, sketch_id: SketchId) -> ApiResult<Outcome> {
        if self.document.sketch(sketch_id).is_none() {
            return Err(ApiError::InvalidArgument(format!(
                "Track 6 sketch {sketch_id:?} not found"
            )));
        }
        let dependents = self.features_depending_on_sketch(sketch_id);
        if !dependents.is_empty() {
            return Err(ApiError::InvalidArgument(format!(
                "sketch {sketch_id:?} has dependent features: {dependents:?}"
            )));
        }
        if self.document.remove_sketch(sketch_id) {
            Ok(Outcome::SolidDeleted {
                id: SolidId(sketch_id.0 as u32),
            })
        } else {
            Err(ApiError::InvalidArgument(format!(
                "Track 6 sketch {sketch_id:?} not found"
            )))
        }
    }

    fn dispatch_map_sketch_to_face(
        &mut self,
        sketch_id: SketchId,
        face: &FaceRef,
    ) -> ApiResult<Outcome> {
        let index = face_index_from_tag(&face.tag).ok_or_else(|| {
            ApiError::InvalidArgument("face tag does not contain a generated face index".into())
        })?;
        let (model, _) = self.document.solid_brep(face.solid).ok_or_else(|| {
            ApiError::InvalidArgument(format!("face solid {} not found", face.solid))
        })?;
        let (origin, normal) = face_plane(model, index)?;
        let plane = Plane {
            origin: [origin.x, origin.y, origin.z],
            normal: [normal.x, normal.y, normal.z],
        };
        let sketch = self.document.sketch_mut(sketch_id).ok_or_else(|| {
            ApiError::InvalidArgument(format!("Track 6 sketch {sketch_id:?} not found"))
        })?;
        sketch.plane = plane;
        self.document.set_active_sketch(sketch_id);
        self.recompute_sketch_dependents(sketch_id)
    }

    fn dispatch_create_body(&mut self, name: &str, base_plane: &PlaneRef) -> ApiResult<Outcome> {
        let plane = plane_from_ref(base_plane)?;
        let body_name = if name.is_empty() {
            format!("Body{}", self.document.body_count() + 1)
        } else {
            name.to_string()
        };
        let body = Body::new_with_plane(0, &body_name, plane.origin, plane.normal);
        let should_activate = self.document.active_body().is_none();
        let id = self.document.push_body(body);
        if should_activate {
            self.document.set_active_body(id);
        }
        Ok(Outcome::BodyCreated {
            body: id,
            name: body_name,
            plane,
        })
    }

    fn dispatch_set_tip(&mut self, body_id: BodyId, feature_id: FeatureId) -> ApiResult<Outcome> {
        let Some(body) = self.document.body_mut(body_id) else {
            return Err(ApiError::InvalidArgument(format!(
                "unknown body {body_id:?}"
            )));
        };
        if !body.set_tip_by_feature_id(feature_id.0) {
            return Err(ApiError::InvalidArgument(format!(
                "feature {feature_id} not found in body {body_id:?}"
            )));
        }
        self.recompute_body(body_id)
    }

    fn dispatch_suppress_feature(
        &mut self,
        feature_id: FeatureId,
        suppressed: bool,
    ) -> ApiResult<Outcome> {
        let body_id = self
            .document
            .find_body_for_feature(feature_id)
            .ok_or_else(|| ApiError::InvalidArgument(format!("unknown feature {feature_id}")))?;
        let Some(body) = self.document.body_mut(body_id) else {
            return Err(ApiError::InvalidArgument(format!(
                "unknown body {body_id:?}"
            )));
        };
        if !body.suppress_feature_by_id(feature_id.0, suppressed) {
            return Err(ApiError::InvalidArgument(format!(
                "unknown feature {feature_id}"
            )));
        }
        self.recompute_body(body_id)
    }

    fn dispatch_reorder_feature(
        &mut self,
        feature_id: FeatureId,
        to_position: u32,
    ) -> ApiResult<Outcome> {
        let body_id = self
            .document
            .find_body_for_feature(feature_id)
            .ok_or_else(|| ApiError::InvalidArgument(format!("unknown feature {feature_id}")))?;
        let Some(body) = self.document.body_mut(body_id) else {
            return Err(ApiError::InvalidArgument(format!(
                "unknown body {body_id:?}"
            )));
        };
        if !body.move_feature_by_feature_id(feature_id.0, to_position as usize) {
            return Err(ApiError::InvalidArgument(format!(
                "cannot move feature {feature_id} to position {to_position}"
            )));
        }
        self.recompute_body(body_id)
    }

    fn dispatch_edit_feature(
        &mut self,
        feature_id: FeatureId,
        new_spec: FeatureSpec,
    ) -> ApiResult<Outcome> {
        let body_id = self
            .document
            .find_body_for_feature(feature_id)
            .ok_or_else(|| ApiError::InvalidArgument(format!("unknown feature {feature_id}")))?;
        let Some(body) = self.document.body_mut(body_id) else {
            return Err(ApiError::InvalidArgument(format!(
                "unknown body {body_id:?}"
            )));
        };
        let Some(index) = body.feature_index_of(feature_id.0) else {
            return Err(ApiError::InvalidArgument(format!(
                "unknown feature {feature_id}"
            )));
        };
        body.features[index].spec_kind = new_spec.spec_kind().to_string();
        body.features[index].kind = spec_kind_to_feature_kind(new_spec.spec_kind());
        body.features[index].spec = Some(feature_spec_value(&new_spec));
        self.recompute_body(body_id)
    }

    fn recompute_body(&mut self, body_id: BodyId) -> ApiResult<Outcome> {
        let (plan, current_solid) = {
            let body = self
                .document
                .body(body_id)
                .ok_or_else(|| ApiError::InvalidArgument(format!("unknown body {body_id:?}")))?;
            let limit = body.tip.map(|tip| tip + 1).unwrap_or(0);
            let mut plan = Vec::new();
            for (index, feature) in body.features.iter().enumerate().take(limit) {
                if feature.suppressed {
                    continue;
                }
                let Some(json) = &feature.spec else {
                    continue;
                };
                if json.is_null() {
                    continue;
                }
                let spec: FeatureSpec = serde_json::from_value(json.clone()).map_err(|err| {
                    ApiError::Codec(format!("feature {} bad spec: {err}", feature.feature_id))
                })?;
                plan.push((index, FeatureId(feature.feature_id), spec));
            }
            (plan, body.current_solid.map(SolidId))
        };

        let mut last_result: Option<(BRepModel, Handle<SolidData>)> = None;
        let mut last_feature_index: Option<usize> = None;
        let mut downstream_invalidated = Vec::new();

        for (feature_index, feature_id, spec) in plan {
            match self.replay_spec(spec, last_result.clone()) {
                Ok((model, handle)) => {
                    if let Some(body) = self.document.body_mut(body_id)
                        && let Some(feature) = body.features.get_mut(feature_index)
                    {
                        feature.cached_solid = Some(handle);
                        feature.solid = handle;
                    }
                    last_result = Some((model, handle));
                    last_feature_index = Some(feature_index);
                }
                Err(_) => {
                    if let Some(body) = self.document.body_mut(body_id)
                        && let Some(feature) = body.features.get_mut(feature_index)
                    {
                        feature.cached_solid = None;
                    }
                    downstream_invalidated.push(feature_id);
                }
            }
        }

        if let Some((model, handle)) = last_result {
            let label = self
                .document
                .body(body_id)
                .and_then(|body| {
                    last_feature_index
                        .and_then(|index| body.features.get(index).map(|f| f.name.clone()))
                })
                .unwrap_or_else(|| "Body".to_string());
            let solid = if let Some(existing) = current_solid {
                if self
                    .document
                    .replace_solid(existing, model.clone(), handle, label.clone())
                {
                    existing
                } else {
                    self.document.insert(model, handle, label)
                }
            } else {
                self.document.insert(model, handle, label)
            };
            if let Some(body) = self.document.body_mut(body_id) {
                body.current_solid = Some(solid.0);
            }
            return Ok(Outcome::FeatureRecomputed {
                feature_id: FeatureId(0),
                solid,
                downstream_invalidated,
            });
        }

        if let Some(existing) = current_solid
            && self.document.get_slot(existing).is_some()
        {
            return Ok(Outcome::FeatureRecomputed {
                feature_id: FeatureId(0),
                solid: existing,
                downstream_invalidated,
            });
        }

        Err(ApiError::InvalidArgument(format!(
            "body {body_id:?} recomputed to empty result"
        )))
    }

    fn replay_spec(
        &self,
        spec: FeatureSpec,
        base: Option<(BRepModel, Handle<SolidData>)>,
    ) -> ApiResult<(BRepModel, Handle<SolidData>)> {
        match spec {
            FeatureSpec::Pad(s) => {
                let (base_model, base_handle) = replay_base("Pad", base)?;
                let profile = self.resolve_sketch_profile(&s.sketch)?;
                let dir_vec = match s.direction {
                    PadDirection::Normal => Vec3::Z,
                    PadDirection::Reversed => -Vec3::Z,
                    PadDirection::TwoSided => Vec3::Z,
                };
                let dist = if s.symmetric || matches!(s.direction, PadDirection::TwoSided) {
                    s.distance * 0.5
                } else {
                    s.distance
                };
                let result = cadkernel_modeling::features::pad(
                    &base_model,
                    base_handle,
                    &profile,
                    dir_vec,
                    dist,
                )?;
                Ok((result.model, result.solid))
            }
            FeatureSpec::Pocket(s) => {
                let (base_model, base_handle) = replay_base("Pocket", base)?;
                let profile = self.resolve_sketch_profile(&s.sketch)?;
                let result = cadkernel_modeling::features::pocket(
                    &base_model,
                    base_handle,
                    &profile,
                    Vec3::Z,
                    s.distance,
                )?;
                Ok((result.model, result.solid))
            }
            FeatureSpec::Revolve(s) => {
                let profile = self.resolve_sketch_profile(&s.sketch)?;
                let (axis_origin, axis_dir) = self.resolve_axis(&s.axis)?;
                let mut model = BRepModel::new();
                let result = cadkernel_modeling::features::revolve(
                    &mut model,
                    &profile,
                    axis_origin,
                    axis_dir,
                    s.angle_rad,
                    64,
                )?;
                Ok((model, result.solid))
            }
            FeatureSpec::Groove(s) => {
                let (base_model, base_handle) = replay_base("Groove", base)?;
                let profile = self.resolve_sketch_profile(&s.sketch)?;
                let (axis_origin, axis_dir) = self.resolve_axis(&s.axis)?;
                let result = cadkernel_modeling::features::groove(
                    &base_model,
                    base_handle,
                    &profile,
                    axis_origin,
                    axis_dir,
                    s.angle_rad,
                    64,
                )?;
                Ok((result.model, result.solid))
            }
            FeatureSpec::Hole(s) => {
                let (base_model, base_handle) = replay_base("Hole", base)?;
                let (_face_solid, _face_handle) = self.resolve_face_handle(&s.face)?;
                let center = Point3::new(s.position[0], s.position[1], 0.0);
                let result = cadkernel_modeling::features::hole(
                    &base_model,
                    base_handle,
                    center,
                    Vec3::Z,
                    s.radius,
                    s.depth,
                    32,
                )?;
                Ok((result.model, result.solid))
            }
            FeatureSpec::Sweep(s) => {
                let (mut model, _base_handle) = replay_base("Sweep", base)?;
                let profile = self.resolve_sketch_profile(&s.profile_sketch)?;
                let path = self.resolve_sketch_profile(&s.path_sketch)?;
                let result = cadkernel_modeling::features::sweep(&mut model, &profile, &path)?;
                Ok((model, result.solid))
            }
            FeatureSpec::Loft(s) => {
                let (mut model, _base_handle) = replay_base("Loft", base)?;
                let mut resolved = Vec::with_capacity(s.profiles.len());
                for profile in &s.profiles {
                    resolved.push(self.resolve_sketch_profile(profile)?);
                }
                let profile_refs: Vec<&[Point3]> = resolved.iter().map(Vec::as_slice).collect();
                let result = cadkernel_modeling::features::loft(&mut model, &profile_refs)?;
                Ok((model, result.solid))
            }
            FeatureSpec::Helix(s) => {
                let (center, _axis_dir) = self.resolve_axis(&s.axis)?;
                let tube_radius = 0.5_f64.min(s.radius * 0.25);
                if let Some((base_model, base_handle)) = base {
                    let model = cadkernel_modeling::features::additive_helix(
                        &base_model,
                        base_handle,
                        center,
                        s.radius,
                        s.pitch,
                        s.turns,
                        tube_radius,
                        32,
                        16,
                    )?;
                    let handle = first_solid_handle(&model).ok_or_else(|| {
                        ApiError::Kernel("additive_helix replay produced no solid".into())
                    })?;
                    Ok((model, handle))
                } else {
                    let mut model = BRepModel::new();
                    let result = make_helix(
                        &mut model,
                        center,
                        s.radius,
                        s.pitch,
                        s.turns,
                        tube_radius,
                        32,
                        16,
                    )?;
                    Ok((model, result.solid))
                }
            }
            FeatureSpec::Fillet(s) => {
                let (mut model, solid) = replay_base("Fillet", base)?;
                let resolved = self.resolve_edge_refs(&s.edges)?;
                let result = cadkernel_modeling::features::fillet_edges(
                    &mut model, solid, &resolved, s.radius,
                )?;
                Ok((model, result.solid))
            }
            FeatureSpec::Chamfer(s) => {
                let (mut model, solid) = replay_base("Chamfer", base)?;
                let resolved = self.resolve_edge_refs(&s.edges)?;
                let result = cadkernel_modeling::features::chamfer_edges(
                    &mut model, solid, &resolved, s.distance,
                )?;
                Ok((model, result.solid))
            }
            FeatureSpec::Shell(s) => {
                let (mut model, solid) = replay_base("Shell", base)?;
                let faces = self.resolve_face_refs(&s.removed_faces)?;
                let result = cadkernel_modeling::features::shell_solid(
                    &mut model,
                    solid,
                    &faces,
                    s.thickness,
                )?;
                Ok((model, result.solid))
            }
            FeatureSpec::Draft(s) => {
                let (mut model, solid) = replay_base("Draft", base)?;
                let _neutral = self.resolve_face_handle(&s.neutral_plane)?;
                let resolved = self.resolve_face_refs(&s.faces)?;
                let pull_direction = match s.direction {
                    DraftDirection::Pull => Vec3::Z,
                    DraftDirection::Push => -Vec3::Z,
                };
                let result = cadkernel_modeling::features::draft_faces(
                    &mut model,
                    solid,
                    &resolved,
                    pull_direction,
                    s.angle_rad,
                )?;
                Ok((model, result.solid))
            }
        }
    }

    fn features_depending_on_sketch(&self, sketch_id: SketchId) -> Vec<FeatureId> {
        let mut out = Vec::new();
        for body_id in self.document.body_ids() {
            let Some(body) = self.document.body(body_id) else {
                continue;
            };
            let limit = body.tip.map(|tip| tip + 1).unwrap_or(0);
            for feature in body.features.iter().take(limit) {
                let Some(json) = &feature.spec else {
                    continue;
                };
                let Ok(spec) = serde_json::from_value::<FeatureSpec>(json.clone()) else {
                    continue;
                };
                if spec_references_sketch(&spec, sketch_id) {
                    out.push(FeatureId(feature.feature_id));
                }
            }
        }
        out.sort();
        out.dedup();
        out
    }

    fn bodies_depending_on_sketch(&self, sketch_id: SketchId) -> Vec<BodyId> {
        let mut out = Vec::new();
        for body_id in self.document.body_ids() {
            let Some(body) = self.document.body(body_id) else {
                continue;
            };
            let limit = body.tip.map(|tip| tip + 1).unwrap_or(0);
            let depends = body.features.iter().take(limit).any(|feature| {
                let Some(json) = &feature.spec else {
                    return false;
                };
                serde_json::from_value::<FeatureSpec>(json.clone())
                    .map(|spec| spec_references_sketch(&spec, sketch_id))
                    .unwrap_or(false)
            });
            if depends {
                out.push(body_id);
            }
        }
        out
    }

    fn recompute_sketch_dependents(&mut self, sketch_id: SketchId) -> ApiResult<Outcome> {
        let mut downstream_invalidated = self.features_depending_on_sketch(sketch_id);
        let bodies = self.bodies_depending_on_sketch(sketch_id);
        let mut solid = self
            .document
            .solid_ids()
            .into_iter()
            .next()
            .unwrap_or(SolidId(0));
        for body_id in bodies {
            if let Outcome::FeatureRecomputed {
                solid: recomputed,
                downstream_invalidated: mut invalidated,
                ..
            } = self.recompute_body(body_id)?
            {
                solid = recomputed;
                downstream_invalidated.append(&mut invalidated);
            }
        }
        downstream_invalidated.sort();
        downstream_invalidated.dedup();
        Ok(Outcome::FeatureRecomputed {
            feature_id: FeatureId(0),
            solid,
            downstream_invalidated,
        })
    }

    fn active_solid_for_feature(&self, op_name: &str) -> ApiResult<SolidId> {
        if let Some(body_id) = self.document.active_body()
            && let Some(body) = self.document.body(body_id)
            && let Some(solid) = body.current_solid
        {
            let id = SolidId(solid);
            if self.document.get_slot(id).is_some() {
                return Ok(id);
            }
        }
        match self.document.solid_ids().as_slice() {
            [id] => Ok(*id),
            [] => Err(ApiError::InvalidArgument(format!(
                "no active body - {op_name} requires a base body"
            ))),
            _ => Err(ApiError::InvalidArgument(
                "multiple bodies present; explicit body selection arrives in Track 2".into(),
            )),
        }
    }

    fn base_model_and_handle(&self, id: SolidId) -> ApiResult<(BRepModel, Handle<SolidData>)> {
        let slot = self
            .document
            .get_slot(id)
            .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
        let handle = slot
            .handle
            .ok_or_else(|| ApiError::UnknownSolid(format!("{id} has no solid handle")))?;
        Ok((slot.model.clone(), handle))
    }

    fn replace_feature_body(
        &mut self,
        old_id: SolidId,
        model: BRepModel,
        handle: Handle<SolidData>,
        label: &str,
        spec_kind: &str,
        spec_value: serde_json::Value,
    ) -> ApiResult<Outcome> {
        let feature_id = self.predicted_feature_id();
        self.document.remove(old_id);
        let solid = self.document.insert(model, handle, label);
        let body = self.append_feature_to_active_body(AppendFeatureArgs {
            solid,
            label,
            kind: spec_kind_to_feature_kind(spec_kind),
            spec_kind,
            spec_value,
            handle,
            feature_id,
        });
        Ok(Outcome::FeatureAdded {
            feature_id,
            body,
            solid,
        })
    }

    fn ensure_active_body(&mut self) -> BodyId {
        if let Some(id) = self.document.active_body() {
            return id;
        }
        let name = format!("Body{}", self.document.body_count() + 1);
        self.document
            .push_body(Body::new_with_plane(0, &name, [0.0; 3], [0.0, 0.0, 1.0]))
    }

    fn append_feature_to_active_body(&mut self, args: AppendFeatureArgs<'_>) -> BodyId {
        let body_id = self.ensure_active_body();
        if let Some(body) = self.document.body_mut(body_id) {
            body.add_feature_with_spec(
                args.feature_id.0,
                args.label,
                args.kind,
                args.spec_kind,
                args.spec_value,
            );
            if let Some(feature) = body.features.last_mut() {
                feature.solid = args.handle;
                feature.cached_solid = Some(args.handle);
            }
            body.current_solid = Some(args.solid.0);
        }
        body_id
    }

    fn predicted_feature_id(&self) -> FeatureId {
        // execute() appends history after dispatch(), so feature outcomes use
        // the next history index as a stable Track 1 placeholder.
        FeatureId(self.document.history().len() as u64 + 1)
    }

    fn resolve_sketch_profile(&self, sketch: &SketchRef) -> ApiResult<Vec<Point3>> {
        let persisted = self.document.sketch(sketch.sketch_id).ok_or_else(|| {
            ApiError::InvalidArgument(format!(
                "sketch resolution arrives in Track 6: sketch {:?} not found",
                sketch.sketch_id
            ))
        })?;
        profile_from_persisted_sketch(persisted)
    }

    fn resolve_face_handle(&self, face: &FaceRef) -> ApiResult<(SolidId, Handle<FaceData>)> {
        let _ = face;
        Err(ApiError::InvalidArgument(
            "face/edge resolution arrives in Track 4".into(),
        ))
    }

    fn resolve_edge_vertices(
        &self,
        edge: &EdgeRef,
    ) -> ApiResult<(SolidId, Handle<VertexData>, Handle<VertexData>)> {
        let _ = edge;
        Err(ApiError::InvalidArgument(
            "face/edge resolution arrives in Track 4".into(),
        ))
    }

    fn resolve_face_refs(&self, faces: &[FaceRef]) -> ApiResult<Vec<Handle<FaceData>>> {
        let mut resolved = Vec::with_capacity(faces.len());
        for face in faces {
            let (_, handle) = self.resolve_face_handle(face)?;
            resolved.push(handle);
        }
        Ok(resolved)
    }

    fn resolve_edge_refs(
        &self,
        edges: &[EdgeRef],
    ) -> ApiResult<Vec<(Handle<VertexData>, Handle<VertexData>)>> {
        let mut resolved = Vec::with_capacity(edges.len());
        for edge in edges {
            let (_, a, b) = self.resolve_edge_vertices(edge)?;
            resolved.push((a, b));
        }
        Ok(resolved)
    }

    fn resolve_axis(&self, axis: &AxisRef) -> ApiResult<(Point3, Vec3)> {
        let (origin, direction) = match axis {
            AxisRef::Origin | AxisRef::Z => (Point3::ORIGIN, Vec3::Z),
            AxisRef::X => (Point3::ORIGIN, Vec3::X),
            AxisRef::Y => (Point3::ORIGIN, Vec3::Y),
            AxisRef::Custom {
                position,
                direction,
            } => (
                Point3::new(position[0], position[1], position[2]),
                Vec3::new(direction[0], direction[1], direction[2]),
            ),
        };
        let dir = direction
            .normalized()
            .ok_or_else(|| ApiError::InvalidArgument("axis direction must be non-zero".into()))?;
        Ok((origin, dir))
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

struct AppendFeatureArgs<'a> {
    solid: SolidId,
    label: &'a str,
    kind: FeatureKind,
    spec_kind: &'a str,
    spec_value: serde_json::Value,
    handle: Handle<SolidData>,
    feature_id: FeatureId,
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

fn feature_spec_value(spec: &FeatureSpec) -> serde_json::Value {
    serde_json::to_value(spec).unwrap_or(serde_json::Value::Null)
}

fn spec_kind_to_feature_kind(kind: &str) -> FeatureKind {
    match kind {
        "pad" => FeatureKind::Pad,
        "pocket" => FeatureKind::Pocket,
        "revolve" => FeatureKind::Revolve,
        "groove" => FeatureKind::Groove,
        "hole" => FeatureKind::Hole,
        "sweep" => FeatureKind::Sweep,
        "loft" => FeatureKind::Loft,
        "helix" => FeatureKind::Helix,
        "fillet" => FeatureKind::Fillet,
        "chamfer" => FeatureKind::Chamfer,
        "shell" => FeatureKind::Shell,
        "draft" => FeatureKind::Draft,
        "mirror" => FeatureKind::Mirror,
        "pattern" => FeatureKind::Pattern,
        _ => FeatureKind::Pad,
    }
}

fn spec_references_sketch(spec: &FeatureSpec, sketch_id: SketchId) -> bool {
    match spec {
        FeatureSpec::Pad(s) => s.sketch.sketch_id == sketch_id,
        FeatureSpec::Pocket(s) => s.sketch.sketch_id == sketch_id,
        FeatureSpec::Revolve(s) => s.sketch.sketch_id == sketch_id,
        FeatureSpec::Groove(s) => s.sketch.sketch_id == sketch_id,
        FeatureSpec::Sweep(s) => {
            s.profile_sketch.sketch_id == sketch_id || s.path_sketch.sketch_id == sketch_id
        }
        FeatureSpec::Loft(s) => s.profiles.iter().any(|profile| profile.sketch_id == sketch_id),
        FeatureSpec::Hole(_)
        | FeatureSpec::Helix(_)
        | FeatureSpec::Fillet(_)
        | FeatureSpec::Chamfer(_)
        | FeatureSpec::Shell(_)
        | FeatureSpec::Draft(_) => false,
    }
}

fn apply_sketch_edit(sketch: &mut PersistedSketch, edit: &SketchEdit) -> ApiResult<()> {
    match edit {
        SketchEdit::AddPoint { x, y } => {
            sketch.entities.push(SketchEntity::Point { x: *x, y: *y });
        }
        SketchEdit::AddLine { start, end } => {
            validate_point_ref(sketch, *start)?;
            validate_point_ref(sketch, *end)?;
            if start == end {
                return Err(ApiError::InvalidArgument(
                    "sketch line endpoints must differ".into(),
                ));
            }
            sketch.entities.push(SketchEntity::Line {
                start: *start,
                end: *end,
            });
        }
        SketchEdit::AddSegment { entity } => {
            validate_sketch_entity(sketch, entity)?;
            sketch.entities.push(entity.clone());
        }
        SketchEdit::RemoveSegment { entity } => {
            if !remove_sketch_entity(sketch, *entity) {
                return Err(ApiError::InvalidArgument(format!(
                    "sketch entity {entity:?} not found"
                )));
            }
        }
        SketchEdit::UpdateConstraint { index, constraint } => {
            validate_sketch_constraint(sketch, constraint)?;
            let idx = *index as usize;
            if idx < sketch.constraints.len() {
                sketch.constraints[idx] = constraint.clone();
            } else if idx == sketch.constraints.len() {
                sketch.constraints.push(constraint.clone());
            } else {
                return Err(ApiError::InvalidArgument(format!(
                    "constraint index {index} out of range"
                )));
            }
        }
        SketchEdit::UpdateParameter { name, value } => {
            apply_sketch_parameter(sketch, name, *value)?;
        }
    }
    Ok(())
}

fn validate_sketch_entity(sketch: &PersistedSketch, entity: &SketchEntity) -> ApiResult<()> {
    match entity {
        SketchEntity::Point { .. } => Ok(()),
        SketchEntity::Line { start, end } => {
            validate_point_ref(sketch, *start)?;
            validate_point_ref(sketch, *end)?;
            if start == end {
                return Err(ApiError::InvalidArgument(
                    "sketch line endpoints must differ".into(),
                ));
            }
            Ok(())
        }
        SketchEntity::Circle { center, radius } => {
            validate_point_ref(sketch, *center)?;
            if *radius <= 0.0 {
                return Err(ApiError::InvalidArgument(format!(
                    "circle radius must be > 0, got {radius}"
                )));
            }
            Ok(())
        }
        SketchEntity::Arc {
            center,
            start_point,
            end_point,
            radius,
            ..
        } => {
            validate_point_ref(sketch, *center)?;
            validate_point_ref(sketch, *start_point)?;
            validate_point_ref(sketch, *end_point)?;
            if *radius <= 0.0 {
                return Err(ApiError::InvalidArgument(format!(
                    "arc radius must be > 0, got {radius}"
                )));
            }
            Ok(())
        }
    }
}

fn validate_sketch_constraint(
    sketch: &PersistedSketch,
    constraint: &SketchConstraint,
) -> ApiResult<()> {
    match constraint {
        SketchConstraint::Horizontal { line }
        | SketchConstraint::Vertical { line }
        | SketchConstraint::Length { line, .. } => validate_line_ref(sketch, *line),
        SketchConstraint::Fixed { point, .. } => validate_point_ref(sketch, *point),
        SketchConstraint::Distance { a, b, distance } => {
            validate_point_ref(sketch, *a)?;
            validate_point_ref(sketch, *b)?;
            if *distance <= 0.0 {
                return Err(ApiError::InvalidArgument(format!(
                    "distance constraint must be > 0, got {distance}"
                )));
            }
            Ok(())
        }
    }
}

fn apply_sketch_parameter(
    sketch: &mut PersistedSketch,
    name: &str,
    value: f64,
) -> ApiResult<()> {
    match name {
        "origin_x" => sketch.plane.origin[0] = value,
        "origin_y" => sketch.plane.origin[1] = value,
        "origin_z" => sketch.plane.origin[2] = value,
        "normal_x" => sketch.plane.normal[0] = value,
        "normal_y" => sketch.plane.normal[1] = value,
        "normal_z" => sketch.plane.normal[2] = value,
        "" => {}
        other => {
            return Err(ApiError::InvalidArgument(format!(
                "unknown sketch parameter {other:?}"
            )));
        }
    }
    Ok(())
}

fn remove_sketch_entity(sketch: &mut PersistedSketch, entity: EntityId) -> bool {
    let mut seen = 0_u64;
    let pos = sketch.entities.iter().position(|candidate| {
        let matches_kind = matches!(
            (entity, candidate),
            (EntityId::Point { .. }, SketchEntity::Point { .. })
                | (EntityId::Line { .. }, SketchEntity::Line { .. })
                | (EntityId::Circle { .. }, SketchEntity::Circle { .. })
                | (EntityId::Arc { .. }, SketchEntity::Arc { .. })
        );
        if !matches_kind {
            return false;
        }
        let target = match entity {
            EntityId::Point { index }
            | EntityId::Line { index }
            | EntityId::Circle { index }
            | EntityId::Arc { index } => index,
        };
        let found = seen == target;
        seen += 1;
        found
    });
    if let Some(pos) = pos {
        sketch.entities.remove(pos);
        true
    } else {
        false
    }
}

fn validate_point_ref(sketch: &PersistedSketch, index: u64) -> ApiResult<()> {
    let points = sketch_point_count(sketch);
    if index >= points {
        return Err(ApiError::InvalidArgument(format!(
            "point index {index} out of range (points={points})"
        )));
    }
    Ok(())
}

fn validate_line_ref(sketch: &PersistedSketch, index: u64) -> ApiResult<()> {
    let lines = sketch_line_count(sketch);
    if index >= lines {
        return Err(ApiError::InvalidArgument(format!(
            "line index {index} out of range (lines={lines})"
        )));
    }
    Ok(())
}

fn sketch_point_count(sketch: &PersistedSketch) -> u64 {
    sketch
        .entities
        .iter()
        .filter(|entity| matches!(entity, SketchEntity::Point { .. }))
        .count() as u64
}

fn sketch_line_count(sketch: &PersistedSketch) -> u64 {
    sketch
        .entities
        .iter()
        .filter(|entity| matches!(entity, SketchEntity::Line { .. }))
        .count() as u64
}

fn profile_from_persisted_sketch(sketch: &PersistedSketch) -> ApiResult<Vec<Point3>> {
    let mut points = Vec::new();
    let mut lines = Vec::new();
    for entity in &sketch.entities {
        match *entity {
            SketchEntity::Point { x, y } => points.push([x, y]),
            SketchEntity::Line { start, end } => lines.push((start as usize, end as usize)),
            SketchEntity::Circle { .. } | SketchEntity::Arc { .. } => {}
        }
    }
    if points.len() < 3 {
        return Err(ApiError::InvalidArgument(format!(
            "sketch {:?} has fewer than 3 profile points",
            sketch.id
        )));
    }
    let ordered = if lines.is_empty() {
        (0..points.len()).collect()
    } else {
        ordered_closed_profile(&points, &lines)?
    };
    Ok(ordered
        .into_iter()
        .map(|idx| plane_to_world(&sketch.plane, points[idx][0], points[idx][1]))
        .collect())
}

fn ordered_closed_profile(points: &[[f64; 2]], lines: &[(usize, usize)]) -> ApiResult<Vec<usize>> {
    if lines.len() < 3 {
        return Err(ApiError::InvalidArgument(
            "sketch profile needs at least 3 line segments".into(),
        ));
    }
    let mut adjacency: Vec<Vec<(usize, usize)>> = vec![Vec::new(); points.len()];
    for (line_idx, (a, b)) in lines.iter().copied().enumerate() {
        if a >= points.len() || b >= points.len() {
            return Err(ApiError::InvalidArgument(format!(
                "sketch line {line_idx} references missing point"
            )));
        }
        if a == b {
            return Err(ApiError::InvalidArgument(format!(
                "sketch line {line_idx} has identical endpoints"
            )));
        }
        adjacency[a].push((line_idx, b));
        adjacency[b].push((line_idx, a));
    }
    let used_points: Vec<usize> = adjacency
        .iter()
        .enumerate()
        .filter_map(|(idx, edges)| (!edges.is_empty()).then_some(idx))
        .collect();
    if used_points.len() < 3 {
        return Err(ApiError::InvalidArgument(
            "sketch profile has fewer than 3 connected points".into(),
        ));
    }
    if let Some(point) = used_points
        .iter()
        .find(|idx| adjacency[**idx].len() != 2)
        .copied()
    {
        return Err(ApiError::InvalidArgument(format!(
            "sketch profile is open or branched at point {point}"
        )));
    }

    let start = used_points[0];
    let mut ordered = vec![start];
    let mut current = adjacency[start][0].1;
    let mut previous_line = adjacency[start][0].0;
    let mut used_lines = std::collections::HashSet::from([previous_line]);

    while current != start {
        if ordered.len() > lines.len() {
            return Err(ApiError::InvalidArgument(
                "sketch profile does not form a single closed loop".into(),
            ));
        }
        ordered.push(current);
        let Some((next_line, next_point)) = adjacency[current]
            .iter()
            .copied()
            .find(|(line_idx, _)| *line_idx != previous_line)
        else {
            return Err(ApiError::InvalidArgument(
                "sketch profile chain terminated early".into(),
            ));
        };
        if !used_lines.insert(next_line) && next_point != start {
            return Err(ApiError::InvalidArgument(
                "sketch profile revisits a line before closing".into(),
            ));
        }
        previous_line = next_line;
        current = next_point;
    }
    if used_lines.len() != lines.len() {
        return Err(ApiError::InvalidArgument(
            "sketch profile has multiple disconnected loops".into(),
        ));
    }
    Ok(ordered)
}

fn plane_to_world(plane: &Plane, x: f64, y: f64) -> Point3 {
    let origin = Point3::new(plane.origin[0], plane.origin[1], plane.origin[2]);
    let normal = Vec3::new(plane.normal[0], plane.normal[1], plane.normal[2])
        .normalized()
        .unwrap_or(Vec3::Z);
    let hint = if normal.dot(Vec3::X).abs() < 0.9 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let x_axis = (hint - normal * normal.dot(hint))
        .normalized()
        .unwrap_or(Vec3::X);
    let y_axis = normal.cross(x_axis);
    origin + x_axis * x + y_axis * y
}

fn face_index_from_tag(tag: &cadkernel_topology::Tag) -> Option<u32> {
    tag.segments.last().and_then(|segment| match segment.kind {
        SegmentKind::Generated(index) | SegmentKind::Split(index) => Some(index),
        SegmentKind::Modified | SegmentKind::Merged => None,
    })
}

fn plane_from_ref(plane: &PlaneRef) -> ApiResult<crate::outcome::Plane> {
    let (origin, normal) = match plane {
        PlaneRef::XY => ([0.0, 0.0, 0.0], [0.0, 0.0, 1.0]),
        PlaneRef::XZ => ([0.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        PlaneRef::YZ => ([0.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
        PlaneRef::Custom { origin, normal } => (*origin, *normal),
    };
    let n = Vec3::new(normal[0], normal[1], normal[2])
        .normalized()
        .ok_or_else(|| ApiError::InvalidArgument("base_plane normal must be non-zero".into()))?;
    Ok(crate::outcome::Plane {
        origin,
        normal: [n.x, n.y, n.z],
    })
}

fn replay_base(
    op_name: &str,
    base: Option<(BRepModel, Handle<SolidData>)>,
) -> ApiResult<(BRepModel, Handle<SolidData>)> {
    base.ok_or_else(|| ApiError::InvalidArgument(format!("{op_name} replay requires a base solid")))
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
        (Command::Pad { distance, .. }, _) => format!("Pad {distance}"),
        (Command::Pocket { distance, .. }, _) => format!("Pocket {distance}"),
        (Command::Revolve { angle_rad, .. }, _) => format!("Revolve {angle_rad} rad"),
        (Command::Groove { angle_rad, .. }, _) => format!("Groove {angle_rad} rad"),
        (Command::Hole { radius, depth, .. }, _) => {
            format!("Hole r={radius} depth={depth}")
        }
        (Command::Sweep { .. }, _) => "Sweep".to_string(),
        (Command::Loft { profiles, .. }, _) => format!("Loft {} profiles", profiles.len()),
        (Command::Helix { radius, turns, .. }, _) => format!("Helix r={radius} turns={turns}"),
        (Command::Fillet { edges, radius, .. }, _) => {
            format!("Fillet {} edges r={radius}", edges.len())
        }
        (
            Command::Chamfer {
                edges, distance, ..
            },
            _,
        ) => format!("Chamfer {} edges d={distance}", edges.len()),
        (
            Command::Shell {
                removed_faces,
                thickness,
                ..
            },
            _,
        ) => format!("Shell {} faces t={thickness}", removed_faces.len()),
        (
            Command::Draft {
                faces, angle_rad, ..
            },
            _,
        ) => format!("Draft {} faces angle={angle_rad}", faces.len()),
        (Command::CreateSketch { name, .. }, _) => format!("CreateSketch {name}"),
        (Command::EditSketch { sketch, edits }, _) => {
            format!("EditSketch {sketch:?} ({} edits)", edits.len())
        }
        (Command::DeleteSketch { sketch }, _) => format!("DeleteSketch {sketch:?}"),
        (Command::MapSketchToFace { sketch, face }, _) => {
            format!("MapSketchToFace {sketch:?} -> {}", face.solid)
        }
        (Command::CreateBody { name, .. }, _) => format!("CreateBody {name}"),
        (Command::SetTip { body, feature }, _) => {
            format!("SetTip {body:?} -> {feature}")
        }
        (
            Command::SuppressFeature {
                feature,
                suppressed,
            },
            _,
        ) => format!("SuppressFeature {feature}={suppressed}"),
        (Command::ReorderFeature { from, to_position }, _) => {
            format!("ReorderFeature {from} -> {to_position}")
        }
        (Command::RecomputeBody { body }, _) => format!("RecomputeBody {body:?}"),
        (Command::EditFeature { feature, new_spec }, _) => {
            format!("EditFeature {feature} {}", new_spec.spec_kind())
        }
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
