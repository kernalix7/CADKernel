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

use cadkernel_math::{Point3, Vec3};
use cadkernel_modeling::measure::solid_mass_properties;
use cadkernel_modeling::quick::{
    quick_box, quick_cone, quick_cylinder, quick_intersect, quick_sphere, quick_subtract,
    quick_torus, quick_union,
};
use cadkernel_modeling::{extrude, mirror_solid};
use cadkernel_topology::{BRepModel, Handle, SolidData};
use serde::{Deserialize, Serialize};

use crate::command::Command;
use crate::document::{Document, HistoryEvent, SolidId, SolidSlot};
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

    /// Restore a session from a `.cadk` byte buffer produced by
    /// [`Self::save_cadk`]. The redo stack is **not** preserved by this
    /// codec — only the applied prefix round-trips. Use
    /// [`Self::save_to_json`] / [`Self::load_from_json`] when redo state
    /// matters.
    pub fn load_cadk(bytes: &[u8]) -> ApiResult<Self> {
        let commands = crate::cadk::decode(bytes)?;
        Self::replay(&commands)
    }

    /// Restore a session from a [`SessionSnapshot`] JSON document.
    /// Replays `commands[..cursor]` and keeps `commands[cursor..]` as the
    /// pending redo stack.
    pub fn load_from_json(json: &str) -> ApiResult<Self> {        let snap: SessionSnapshot = serde_json::from_str(json)?;
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
            (
                Command::Scale {
                    id: a,
                    factor: f1,
                },
                Command::Scale {
                    id: b,
                    factor: f2,
                },
            ) if a == b => Some(Command::Scale {
                id: *a,
                factor: f1 * f2,
            }),
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
            Command::CreateCone { radius, height } => {
                let model = quick_cone(*radius, *height)?;
                self.insert_first_solid(model, "Cone")
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
            } => self.extrude_profile(profile, *direction, *distance),
            Command::LinearPattern {
                id,
                direction,
                spacing,
                count,
            } => self.linear_pattern(*id, *direction, *spacing, *count),
            Command::Mirror { id, point, normal } => self.mirror(*id, *point, *normal),
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

    fn rename(&mut self, id: SolidId, label: String) -> ApiResult<Outcome> {
        let slot = slot_mut(&mut self.document, id)?;
        slot.label = label;
        Ok(Outcome::SolidModified { id })
    }

    fn extrude_profile(
        &mut self,
        profile: &[[f64; 3]],
        direction: [f64; 3],
        distance: f64,
    ) -> ApiResult<Outcome> {
        if profile.len() < 3 {
            return Err(ApiError::InvalidArgument(format!(
                "extrude profile must have at least 3 points, got {}",
                profile.len()
            )));
        }
        if distance <= 0.0 {
            return Err(ApiError::InvalidArgument(format!(
                "extrude distance must be > 0, got {distance}"
            )));
        }
        let pts: Vec<Point3> = profile
            .iter()
            .map(|p| Point3::new(p[0], p[1], p[2]))
            .collect();
        let dir = Vec3::new(direction[0], direction[1], direction[2]);
        if dir.length() < 1e-12 {
            return Err(ApiError::InvalidArgument(
                "extrude direction must be non-zero".into(),
            ));
        }
        let mut model = BRepModel::new();
        let result = extrude(&mut model, &pts, dir, distance)?;
        let id = self.document.insert(model, result.solid, "Extrude");
        Ok(Outcome::SolidCreated {
            id,
            label: "Extrude".into(),
        })
    }

    fn linear_pattern(
        &mut self,
        id: SolidId,
        direction: [f64; 3],
        spacing: f64,
        count: u32,
    ) -> ApiResult<Outcome> {
        if count < 2 {
            return Err(ApiError::InvalidArgument(format!(
                "linear_pattern count must be ≥ 2, got {count}"
            )));
        }
        let dir_v = Vec3::new(direction[0], direction[1], direction[2]);
        let dir = dir_v.normalized().ok_or_else(|| {
            ApiError::InvalidArgument("linear_pattern direction must be non-zero".into())
        })?;
        // Snapshot the source model + label up front so subsequent
        // mutations to the document do not invalidate references.
        let (src_model, label) = {
            let slot = self
                .document
                .get_slot(id)
                .ok_or_else(|| ApiError::UnknownSolid(format!("{id}")))?;
            (slot.model.clone(), slot.label.clone())
        };
        let mut ids = vec![id];
        for i in 1..count {
            let mut copy = src_model.clone();
            let dx = dir.x * spacing * i as f64;
            let dy = dir.y * spacing * i as f64;
            let dz = dir.z * spacing * i as f64;
            for (_h, v) in copy.vertices.iter_mut() {
                v.point = Point3::new(v.point.x + dx, v.point.y + dy, v.point.z + dz);
            }
            let handle = first_solid_handle(&copy).ok_or_else(|| {
                ApiError::Kernel("linear_pattern source had no solid handle".into())
            })?;
            let new_id = self
                .document
                .insert(copy, handle, format!("{label} (pattern {i})"));
            ids.push(new_id);
        }
        Ok(Outcome::PatternCreated { ids })
    }

    fn mirror(
        &mut self,
        id: SolidId,
        point: [f64; 3],
        normal: [f64; 3],
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
            .ok_or_else(|| {
                ApiError::Kernel("mirror_solid produced no new solid handle".into())
            })?;
        let new_id = self
            .document
            .insert(work, mirror_handle, format!("{label} (mirror)"));
        Ok(Outcome::SolidCreated {
            id: new_id,
            label: format!("{label} (mirror)"),
        })
    }
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
        (Command::CreateCone { radius, height }, _) => {
            format!("Cone r={radius} h={height}")
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
        (Command::Rename { id, label }, _) => format!("Rename {id} → {label:?}"),
        (Command::DeleteSolid { id }, _) => format!("Delete {id}"),
        (
            Command::Extrude {
                profile, distance, ..
            },
            _,
        ) => format!("Extrude ({} pts, h={distance})", profile.len()),
        (
            Command::LinearPattern { id, count, .. },
            Outcome::PatternCreated { ids },
        ) => format!("LinearPattern {id} ×{count} ({} solids)", ids.len()),
        (Command::LinearPattern { id, count, .. }, _) => {
            format!("LinearPattern {id} ×{count}")
        }
        (Command::Mirror { id, .. }, _) => format!("Mirror {id}"),
        (Command::NewDocument, _) => "New document".into(),
        (Command::Noop, _) => "Noop".into(),
    }
}
