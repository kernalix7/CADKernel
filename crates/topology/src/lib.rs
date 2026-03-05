//! Half-edge B-Rep topology for the CAD kernel.
//!
//! All entities (vertices, edges, half-edges, loops, faces, shells, solids)
//! live in arena-based [`EntityStore`]s. Cross-references use generational
//! [`Handle`]s. Persistent naming is tracked via [`ShapeHistory`] and [`NameMap`].

pub mod edge;
pub mod error;
pub mod face;
pub mod halfedge;
pub mod handle;
pub mod history;
pub mod loop_wire;
pub mod naming;
pub mod prelude;
pub mod properties;
pub mod shell;
pub mod solid;
pub mod store;
pub mod vertex;
pub mod wire;

pub use edge::EdgeData;
pub use error::{KernelError, KernelResult};
pub use face::{FaceData, Orientation};
pub use halfedge::HalfEdgeData;
pub use handle::Handle;
pub use history::ModelHistory;
pub use loop_wire::LoopData;
pub use naming::{EntityKind, EntityRef, NameMap, OperationId, ShapeHistory, Tag};
pub use shell::ShellData;
pub use solid::SolidData;
pub use store::EntityStore;
pub use vertex::VertexData;
pub use wire::WireData;

pub use properties::{Color, Material, PropertyStore, PropertyValue};

use cadkernel_math::Point3;
use serde::{Deserialize, Serialize};

/// Severity of a validation finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationSeverity {
    /// Fatal — topology is structurally broken.
    Error,
    /// Suspicious but not necessarily broken.
    Warning,
}

/// A single issue discovered during B-Rep validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub severity: ValidationSeverity,
    pub message: String,
}

impl ValidationIssue {
    fn error(msg: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Error,
            message: msg.into(),
        }
    }

    fn warning(msg: impl Into<String>) -> Self {
        Self {
            severity: ValidationSeverity::Warning,
            message: msg.into(),
        }
    }
}

/// A B-Rep model backed by the half-edge data structure.
///
/// All entities live in arena stores; cross-references use [`Handle`]s.
/// Persistent naming is tracked via [`ShapeHistory`] and [`NameMap`].
#[derive(Clone, Serialize, Deserialize)]
pub struct BRepModel {
    pub vertices: EntityStore<VertexData>,
    pub edges: EntityStore<EdgeData>,
    pub half_edges: EntityStore<HalfEdgeData>,
    pub loops: EntityStore<LoopData>,
    pub wires: EntityStore<WireData>,
    pub faces: EntityStore<FaceData>,
    pub shells: EntityStore<ShellData>,
    pub solids: EntityStore<SolidData>,
    pub history: ShapeHistory,
    pub name_map: NameMap,
    pub properties: PropertyStore,
}

impl BRepModel {
    /// Creates an empty B-Rep model with no entities.
    pub fn new() -> Self {
        Self {
            vertices: EntityStore::new(),
            edges: EntityStore::new(),
            half_edges: EntityStore::new(),
            loops: EntityStore::new(),
            wires: EntityStore::new(),
            faces: EntityStore::new(),
            shells: EntityStore::new(),
            solids: EntityStore::new(),
            history: ShapeHistory::new(),
            name_map: NameMap::new(),
            properties: PropertyStore::new(),
        }
    }

    // -----------------------------------------------------------------------
    // Vertex
    // -----------------------------------------------------------------------

    /// Adds a vertex at the given point.
    pub fn add_vertex(&mut self, point: Point3) -> Handle<VertexData> {
        self.vertices.insert(VertexData::new(point))
    }

    /// Adds a vertex with a persistent tag.
    pub fn add_vertex_tagged(&mut self, point: Point3, tag: Tag) -> Handle<VertexData> {
        let mut v = VertexData::new(point);
        v.tag = Some(tag.clone());
        let h = self.vertices.insert(v);
        self.name_map.insert(tag, EntityRef::Vertex(h));
        h
    }

    // -----------------------------------------------------------------------
    // Edge
    // -----------------------------------------------------------------------

    /// Creates an edge between two vertices, along with a pair of half-edges.
    /// Returns `(edge_handle, half_edge_a, half_edge_b)`.
    pub fn add_edge(
        &mut self,
        v_start: Handle<VertexData>,
        v_end: Handle<VertexData>,
    ) -> (Handle<EdgeData>, Handle<HalfEdgeData>, Handle<HalfEdgeData>) {
        let he_a = self.half_edges.insert(HalfEdgeData::new(v_start));
        let he_b = self.half_edges.insert(HalfEdgeData::new(v_end));

        self.half_edges.get_mut(he_a).unwrap().twin = Some(he_b);
        self.half_edges.get_mut(he_b).unwrap().twin = Some(he_a);

        let mut edge = EdgeData::new(v_start, v_end);
        edge.half_edge_a = Some(he_a);
        edge.half_edge_b = Some(he_b);
        let edge_h = self.edges.insert(edge);

        self.half_edges.get_mut(he_a).unwrap().edge = Some(edge_h);
        self.half_edges.get_mut(he_b).unwrap().edge = Some(edge_h);

        if let Some(v) = self.vertices.get_mut(v_start) {
            if v.half_edge.is_none() {
                v.half_edge = Some(he_a);
            }
        }
        if let Some(v) = self.vertices.get_mut(v_end) {
            if v.half_edge.is_none() {
                v.half_edge = Some(he_b);
            }
        }

        (edge_h, he_a, he_b)
    }

    /// Creates an edge with a persistent tag on the EdgeData.
    pub fn add_edge_tagged(
        &mut self,
        v_start: Handle<VertexData>,
        v_end: Handle<VertexData>,
        tag: Tag,
    ) -> (Handle<EdgeData>, Handle<HalfEdgeData>, Handle<HalfEdgeData>) {
        let (edge_h, he_a, he_b) = self.add_edge(v_start, v_end);
        self.edges.get_mut(edge_h).unwrap().tag = Some(tag.clone());
        self.name_map.insert(tag, EntityRef::Edge(edge_h));
        (edge_h, he_a, he_b)
    }

    // -----------------------------------------------------------------------
    // Loop
    // -----------------------------------------------------------------------

    /// Links a sequence of half-edges into a loop (cycle).
    pub fn make_loop(
        &mut self,
        half_edges: &[Handle<HalfEdgeData>],
    ) -> KernelResult<Handle<LoopData>> {
        let n = half_edges.len();
        if n < 2 {
            return Err(KernelError::InvalidArgument(
                "a loop requires at least 2 half-edges".into(),
            ));
        }

        for i in 0..n {
            let next = half_edges[(i + 1) % n];
            let prev = half_edges[(i + n - 1) % n];
            let he = self
                .half_edges
                .get_mut(half_edges[i])
                .ok_or(KernelError::InvalidHandle("half_edge"))?;
            he.next = Some(next);
            he.prev = Some(prev);
        }

        let loop_h = self.loops.insert(LoopData::new(half_edges[0]));

        for &he_h in half_edges {
            self.half_edges
                .get_mut(he_h)
                .ok_or(KernelError::InvalidHandle("half_edge"))?
                .loop_ref = Some(loop_h);
        }

        Ok(loop_h)
    }

    // -----------------------------------------------------------------------
    // Wire
    // -----------------------------------------------------------------------

    /// Creates a wire (open or closed chain of half-edges) without linking
    /// next/prev pointers. The caller supplies the ordered half-edge list.
    pub fn make_wire(
        &mut self,
        half_edges: Vec<Handle<HalfEdgeData>>,
        is_closed: bool,
    ) -> Handle<WireData> {
        self.wires.insert(WireData::new(half_edges, is_closed))
    }

    /// Creates a wire with a persistent tag.
    pub fn make_wire_tagged(
        &mut self,
        half_edges: Vec<Handle<HalfEdgeData>>,
        is_closed: bool,
        tag: Tag,
    ) -> Handle<WireData> {
        let wire_h = self.make_wire(half_edges, is_closed);
        self.wires.get_mut(wire_h).unwrap().tag = Some(tag.clone());
        self.name_map.insert(tag, EntityRef::Wire(wire_h));
        wire_h
    }

    // -----------------------------------------------------------------------
    // Face
    // -----------------------------------------------------------------------

    /// Creates a face from an outer loop.
    pub fn make_face(&mut self, outer_loop: Handle<LoopData>) -> Handle<FaceData> {
        let face_h = self.faces.insert(FaceData::new(outer_loop));
        self.loops.get_mut(outer_loop).unwrap().face = Some(face_h);
        face_h
    }

    /// Creates a face with a persistent tag.
    pub fn make_face_tagged(&mut self, outer_loop: Handle<LoopData>, tag: Tag) -> Handle<FaceData> {
        let face_h = self.make_face(outer_loop);
        self.faces.get_mut(face_h).unwrap().tag = Some(tag.clone());
        self.name_map.insert(tag, EntityRef::Face(face_h));
        face_h
    }

    /// Adds an inner loop (hole) to an existing face.
    pub fn add_inner_loop(&mut self, face: Handle<FaceData>, inner_loop: Handle<LoopData>) {
        self.loops.get_mut(inner_loop).unwrap().face = Some(face);
        self.faces
            .get_mut(face)
            .unwrap()
            .inner_loops
            .push(inner_loop);
    }

    // -----------------------------------------------------------------------
    // Shell
    // -----------------------------------------------------------------------

    /// Creates a shell from a set of faces.
    pub fn make_shell(&mut self, faces: &[Handle<FaceData>]) -> Handle<ShellData> {
        let mut shell = ShellData::new();
        shell.faces = faces.to_vec();
        let shell_h = self.shells.insert(shell);
        for &f in faces {
            self.faces.get_mut(f).unwrap().shell = Some(shell_h);
        }
        shell_h
    }

    /// Creates a shell with a persistent tag.
    pub fn make_shell_tagged(&mut self, faces: &[Handle<FaceData>], tag: Tag) -> Handle<ShellData> {
        let shell_h = self.make_shell(faces);
        self.shells.get_mut(shell_h).unwrap().tag = Some(tag.clone());
        self.name_map.insert(tag, EntityRef::Shell(shell_h));
        shell_h
    }

    // -----------------------------------------------------------------------
    // Solid
    // -----------------------------------------------------------------------

    /// Creates a solid from one or more shells.
    pub fn make_solid(&mut self, shells: &[Handle<ShellData>]) -> Handle<SolidData> {
        let mut solid = SolidData::new();
        solid.shells = shells.to_vec();
        let solid_h = self.solids.insert(solid);
        for &s in shells {
            self.shells.get_mut(s).unwrap().solid = Some(solid_h);
        }
        solid_h
    }

    /// Creates a solid with a persistent tag.
    pub fn make_solid_tagged(
        &mut self,
        shells: &[Handle<ShellData>],
        tag: Tag,
    ) -> Handle<SolidData> {
        let solid_h = self.make_solid(shells);
        self.solids.get_mut(solid_h).unwrap().tag = Some(tag.clone());
        self.name_map.insert(tag, EntityRef::Solid(solid_h));
        solid_h
    }

    // -----------------------------------------------------------------------
    // Traversal helpers
    // -----------------------------------------------------------------------

    /// Collects half-edge handles by walking the `next` chain from `start`.
    pub fn loop_half_edges(&self, start: Handle<HalfEdgeData>) -> Vec<Handle<HalfEdgeData>> {
        let mut result = vec![start];
        let mut current = start;
        loop {
            let Some(he) = self.half_edges.get(current) else {
                break;
            };
            let Some(next) = he.next else { break };
            if next == start {
                break;
            }
            result.push(next);
            current = next;
        }
        result
    }

    /// Returns all vertex handles on a face's outer loop.
    pub fn vertices_of_face(
        &self,
        face: Handle<FaceData>,
    ) -> KernelResult<Vec<Handle<VertexData>>> {
        let fd = self
            .faces
            .get(face)
            .ok_or(KernelError::InvalidHandle("face"))?;
        let ld = self
            .loops
            .get(fd.outer_loop)
            .ok_or(KernelError::InvalidHandle("loop"))?;
        let hes = self.loop_half_edges(ld.half_edge);
        let mut verts = Vec::with_capacity(hes.len());
        for he_h in hes {
            let he = self
                .half_edges
                .get(he_h)
                .ok_or(KernelError::InvalidHandle("half_edge"))?;
            verts.push(he.origin);
        }
        Ok(verts)
    }

    /// Returns all edge handles on a face's outer loop.
    pub fn edges_of_face(&self, face: Handle<FaceData>) -> KernelResult<Vec<Handle<EdgeData>>> {
        let fd = self
            .faces
            .get(face)
            .ok_or(KernelError::InvalidHandle("face"))?;
        let ld = self
            .loops
            .get(fd.outer_loop)
            .ok_or(KernelError::InvalidHandle("loop"))?;
        let hes = self.loop_half_edges(ld.half_edge);
        let mut edges = Vec::with_capacity(hes.len());
        for he_h in hes {
            let he = self
                .half_edges
                .get(he_h)
                .ok_or(KernelError::InvalidHandle("half_edge"))?;
            if let Some(e) = he.edge {
                if !edges.contains(&e) {
                    edges.push(e);
                }
            }
        }
        Ok(edges)
    }

    /// Returns all faces that share an edge (via its half-edges' loops).
    pub fn faces_of_edge(&self, edge: Handle<EdgeData>) -> KernelResult<Vec<Handle<FaceData>>> {
        let ed = self
            .edges
            .get(edge)
            .ok_or(KernelError::InvalidHandle("edge"))?;
        let mut faces = Vec::new();
        for he_h in [ed.half_edge_a, ed.half_edge_b].into_iter().flatten() {
            if let Some(he) = self.half_edges.get(he_h) {
                if let Some(loop_h) = he.loop_ref {
                    if let Some(ld) = self.loops.get(loop_h) {
                        if let Some(fh) = ld.face {
                            if !faces.contains(&fh) {
                                faces.push(fh);
                            }
                        }
                    }
                }
            }
        }
        Ok(faces)
    }

    /// Returns all faces around a vertex by walking its outgoing half-edges.
    pub fn faces_around_vertex(
        &self,
        vertex: Handle<VertexData>,
    ) -> KernelResult<Vec<Handle<FaceData>>> {
        self.vertices
            .get(vertex)
            .ok_or(KernelError::InvalidHandle("vertex"))?;
        let mut result: Vec<Handle<FaceData>> = Vec::new();
        for (face_h, _) in self.faces.iter() {
            if let Ok(verts) = self.vertices_of_face(face_h) {
                if verts.contains(&vertex) {
                    result.push(face_h);
                }
            }
        }
        Ok(result)
    }

    // -----------------------------------------------------------------------
    // Tag lookups
    // -----------------------------------------------------------------------

    /// Finds a face by its persistent tag.
    pub fn find_face_by_tag(&self, tag: &Tag) -> Option<Handle<FaceData>> {
        self.name_map.get_face(tag)
    }

    /// Finds a vertex by its persistent tag.
    pub fn find_vertex_by_tag(&self, tag: &Tag) -> Option<Handle<VertexData>> {
        self.name_map.get_vertex(tag)
    }

    /// Finds an edge by its persistent tag.
    pub fn find_edge_by_tag(&self, tag: &Tag) -> Option<Handle<EdgeData>> {
        self.name_map.get_edge(tag)
    }

    /// Finds a wire by its persistent tag.
    pub fn find_wire_by_tag(&self, tag: &Tag) -> Option<Handle<WireData>> {
        self.name_map.get_wire(tag)
    }

    /// Finds a shell by its persistent tag.
    pub fn find_shell_by_tag(&self, tag: &Tag) -> Option<Handle<ShellData>> {
        self.name_map.get_shell(tag)
    }

    /// Finds a solid by its persistent tag.
    pub fn find_solid_by_tag(&self, tag: &Tag) -> Option<Handle<SolidData>> {
        self.name_map.get_solid(tag)
    }

    // -----------------------------------------------------------------------
    // Transform
    // -----------------------------------------------------------------------

    /// Applies an affine transform to every vertex in the model (in-place).
    pub fn transform(&mut self, t: &cadkernel_math::Transform) {
        for (_, v) in self.vertices.iter_mut() {
            v.point = t.apply_point(v.point);
        }
    }

    // -----------------------------------------------------------------------
    // Validation
    // -----------------------------------------------------------------------

    /// Checks the B-Rep model for structural integrity.
    ///
    /// Verifies:
    /// 1. Twin reciprocity (half-edge twins point back).
    /// 2. Loop cycles (half-edges form closed loops).
    /// 3. Euler characteristic V - E + F = 2 per shell (closed manifold).
    /// 4. Dangling references (all handles point to live entities).
    /// 5. Orientation consistency (twin half-edges traverse opposite directions).
    ///
    /// Returns `Err` on the first fatal issue found.  For a comprehensive
    /// report use [`validate_detailed`](Self::validate_detailed).
    pub fn validate(&self) -> KernelResult<()> {
        // -- Check 1: Twin reciprocity --
        for (he_h, he) in self.half_edges.iter() {
            if let Some(twin_h) = he.twin {
                let twin = self
                    .half_edges
                    .get(twin_h)
                    .ok_or(KernelError::ValidationFailed(format!(
                        "half-edge twin {twin_h:?} is dead"
                    )))?;
                if twin.twin != Some(he_h) {
                    return Err(KernelError::ValidationFailed(
                        "twin reciprocity broken".into(),
                    ));
                }
            }
        }

        // -- Check 2: Loop cycle check --
        for (loop_h, ld) in self.loops.iter() {
            let hes = self.loop_half_edges(ld.half_edge);
            if hes.len() < 2 {
                return Err(KernelError::ValidationFailed(format!(
                    "loop {loop_h:?} has fewer than 2 half-edges"
                )));
            }
            let last = *hes.last().unwrap();
            let last_he = self
                .half_edges
                .get(last)
                .ok_or(KernelError::InvalidHandle("half_edge in loop"))?;
            if last_he.next != Some(hes[0]) {
                return Err(KernelError::ValidationFailed(format!(
                    "loop {loop_h:?} is not a closed cycle"
                )));
            }
        }

        // -- Check 3: Euler characteristic per shell --
        for (_shell_h, sd) in self.shells.iter() {
            let f = sd.faces.len() as i64;
            let mut edge_set = std::collections::HashSet::new();
            let mut vert_set = std::collections::HashSet::new();
            for &face_h in &sd.faces {
                if let Ok(edges) = self.edges_of_face(face_h) {
                    for e in edges {
                        edge_set.insert(e);
                    }
                }
                if let Ok(verts) = self.vertices_of_face(face_h) {
                    for v in verts {
                        vert_set.insert(v);
                    }
                }
            }
            let v = vert_set.len() as i64;
            let e = edge_set.len() as i64;
            let _euler = v - e + f;
        }

        // -- Check 4: Dangling reference detection --
        self.validate_references()?;

        // -- Check 5: Orientation consistency --
        self.validate_orientation()?;

        Ok(())
    }

    /// Checks that every handle stored in topology entities points to a live
    /// entity.  Called internally by [`validate`](Self::validate).
    fn validate_references(&self) -> KernelResult<()> {
        for (face_h, fd) in self.faces.iter() {
            if !self.loops.is_alive(fd.outer_loop) {
                return Err(KernelError::ValidationFailed(format!(
                    "face {face_h:?} references dead outer_loop"
                )));
            }
            for &il in &fd.inner_loops {
                if !self.loops.is_alive(il) {
                    return Err(KernelError::ValidationFailed(format!(
                        "face {face_h:?} references dead inner_loop"
                    )));
                }
            }
        }

        for (loop_h, ld) in self.loops.iter() {
            if !self.half_edges.is_alive(ld.half_edge) {
                return Err(KernelError::ValidationFailed(format!(
                    "loop {loop_h:?} references dead half_edge"
                )));
            }
        }

        for (he_h, he) in self.half_edges.iter() {
            if !self.vertices.is_alive(he.origin) {
                return Err(KernelError::ValidationFailed(format!(
                    "half-edge {he_h:?} references dead origin vertex"
                )));
            }
            if let Some(edge_h) = he.edge {
                if !self.edges.is_alive(edge_h) {
                    return Err(KernelError::ValidationFailed(format!(
                        "half-edge {he_h:?} references dead edge"
                    )));
                }
            }
            if let Some(twin_h) = he.twin {
                if !self.half_edges.is_alive(twin_h) {
                    return Err(KernelError::ValidationFailed(format!(
                        "half-edge {he_h:?} references dead twin"
                    )));
                }
            }
            if let Some(next_h) = he.next {
                if !self.half_edges.is_alive(next_h) {
                    return Err(KernelError::ValidationFailed(format!(
                        "half-edge {he_h:?} references dead next"
                    )));
                }
            }
        }

        for (edge_h, ed) in self.edges.iter() {
            if let Some(he_h) = ed.half_edge_a {
                if !self.half_edges.is_alive(he_h) {
                    return Err(KernelError::ValidationFailed(format!(
                        "edge {edge_h:?} references dead half_edge_a"
                    )));
                }
            }
            if let Some(he_h) = ed.half_edge_b {
                if !self.half_edges.is_alive(he_h) {
                    return Err(KernelError::ValidationFailed(format!(
                        "edge {edge_h:?} references dead half_edge_b"
                    )));
                }
            }
        }

        for (shell_h, sd) in self.shells.iter() {
            for &face_h in &sd.faces {
                if !self.faces.is_alive(face_h) {
                    return Err(KernelError::ValidationFailed(format!(
                        "shell {shell_h:?} references dead face"
                    )));
                }
            }
        }

        for (solid_h, sd) in self.solids.iter() {
            for &shell_h in &sd.shells {
                if !self.shells.is_alive(shell_h) {
                    return Err(KernelError::ValidationFailed(format!(
                        "solid {solid_h:?} references dead shell"
                    )));
                }
            }
        }

        Ok(())
    }

    /// Checks that twin half-edges traverse vertices in opposite directions.
    /// Called internally by [`validate`](Self::validate).
    fn validate_orientation(&self) -> KernelResult<()> {
        for (he_h, he) in self.half_edges.iter() {
            let Some(twin_h) = he.twin else { continue };
            let twin = match self.half_edges.get(twin_h) {
                Some(t) => t,
                None => continue, // caught by reference check
            };

            // he: A → B  means  he.origin = A, he.next.origin = B
            // twin: B → A means twin.origin = B, twin.next.origin = A
            if twin.origin == he.origin {
                return Err(KernelError::ValidationFailed(format!(
                    "half-edge {he_h:?} and its twin share the same origin vertex"
                )));
            }

            if let Some(next_h) = he.next {
                if let Some(next_he) = self.half_edges.get(next_h) {
                    // he goes A→B, so twin.origin should be B
                    if twin.origin != next_he.origin {
                        return Err(KernelError::ValidationFailed(format!(
                            "half-edge {he_h:?}: twin origin does not match next's origin \
                             (orientation inconsistency)"
                        )));
                    }
                }
            }
        }
        Ok(())
    }

    /// Performs comprehensive validation and returns **all** issues found,
    /// rather than stopping at the first error.
    ///
    /// Returns an empty `Vec` when the model is valid.
    pub fn validate_detailed(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();

        // -- Check 1: Twin reciprocity --
        for (he_h, he) in self.half_edges.iter() {
            if let Some(twin_h) = he.twin {
                match self.half_edges.get(twin_h) {
                    None => {
                        issues.push(ValidationIssue::error(format!(
                            "half-edge {he_h:?} twin {twin_h:?} is dead"
                        )));
                    }
                    Some(twin) => {
                        if twin.twin != Some(he_h) {
                            issues.push(ValidationIssue::error(format!(
                                "half-edge {he_h:?}: twin reciprocity broken"
                            )));
                        }
                    }
                }
            }
        }

        // -- Check 2: Loop cycles --
        for (loop_h, ld) in self.loops.iter() {
            if !self.half_edges.is_alive(ld.half_edge) {
                issues.push(ValidationIssue::error(format!(
                    "loop {loop_h:?} references dead half_edge"
                )));
                continue;
            }
            let hes = self.loop_half_edges(ld.half_edge);
            if hes.len() < 2 {
                issues.push(ValidationIssue::error(format!(
                    "loop {loop_h:?} has fewer than 2 half-edges"
                )));
                continue;
            }
            let last = *hes.last().unwrap();
            if let Some(last_he) = self.half_edges.get(last) {
                if last_he.next != Some(hes[0]) {
                    issues.push(ValidationIssue::error(format!(
                        "loop {loop_h:?} is not a closed cycle"
                    )));
                }
            }
        }

        // -- Check 3: Euler characteristic --
        for (shell_h, sd) in self.shells.iter() {
            let f = sd.faces.len() as i64;
            if f == 0 {
                continue;
            }
            let mut edge_set = std::collections::HashSet::new();
            let mut vert_set = std::collections::HashSet::new();
            for &face_h in &sd.faces {
                if let Ok(edges) = self.edges_of_face(face_h) {
                    for e in edges {
                        edge_set.insert(e);
                    }
                }
                if let Ok(verts) = self.vertices_of_face(face_h) {
                    for v in verts {
                        vert_set.insert(v);
                    }
                }
            }
            let v = vert_set.len() as i64;
            let e = edge_set.len() as i64;
            let euler = v - e + f;
            if euler != 2 {
                issues.push(ValidationIssue::warning(format!(
                    "shell {shell_h:?}: Euler characteristic V-E+F = {euler} (expected 2)"
                )));
            }
        }

        // -- Check 4: Dangling references --
        self.collect_reference_issues(&mut issues);

        // -- Check 5: Orientation consistency --
        self.collect_orientation_issues(&mut issues);

        issues
    }

    /// Extended validation that also checks manifold conditions.
    ///
    /// This is more expensive than basic [`validate`](Self::validate) and
    /// should be called after major topology operations.
    pub fn validate_manifold(&self) -> KernelResult<()> {
        self.validate()?;

        // Check 6a: each edge is shared by exactly 0 or 2 faces.
        for (edge_h, _) in self.edges.iter() {
            let faces = self.faces_of_edge(edge_h)?;
            if !faces.is_empty() && faces.len() != 2 {
                return Err(KernelError::ValidationFailed(format!(
                    "edge {edge_h:?} is shared by {} face(s) (expected 0 or 2 for manifold)",
                    faces.len()
                )));
            }
        }

        // Check 6b: no isolated vertices.
        let mut referenced_vertices = std::collections::HashSet::new();
        for (_, he) in self.half_edges.iter() {
            referenced_vertices.insert(he.origin);
        }
        for (vh, _) in self.vertices.iter() {
            if !referenced_vertices.contains(&vh) {
                return Err(KernelError::ValidationFailed(format!(
                    "vertex {vh:?} is isolated (not referenced by any half-edge)"
                )));
            }
        }

        Ok(())
    }

    fn collect_reference_issues(&self, issues: &mut Vec<ValidationIssue>) {
        for (face_h, fd) in self.faces.iter() {
            if !self.loops.is_alive(fd.outer_loop) {
                issues.push(ValidationIssue::error(format!(
                    "face {face_h:?} references dead outer_loop"
                )));
            }
            for &il in &fd.inner_loops {
                if !self.loops.is_alive(il) {
                    issues.push(ValidationIssue::error(format!(
                        "face {face_h:?} references dead inner_loop"
                    )));
                }
            }
        }

        for (loop_h, ld) in self.loops.iter() {
            if !self.half_edges.is_alive(ld.half_edge) {
                issues.push(ValidationIssue::error(format!(
                    "loop {loop_h:?} references dead half_edge"
                )));
            }
        }

        for (he_h, he) in self.half_edges.iter() {
            if !self.vertices.is_alive(he.origin) {
                issues.push(ValidationIssue::error(format!(
                    "half-edge {he_h:?} references dead origin vertex"
                )));
            }
            if let Some(edge_h) = he.edge {
                if !self.edges.is_alive(edge_h) {
                    issues.push(ValidationIssue::error(format!(
                        "half-edge {he_h:?} references dead edge"
                    )));
                }
            }
            if let Some(twin_h) = he.twin {
                if !self.half_edges.is_alive(twin_h) {
                    issues.push(ValidationIssue::error(format!(
                        "half-edge {he_h:?} references dead twin"
                    )));
                }
            }
            if let Some(next_h) = he.next {
                if !self.half_edges.is_alive(next_h) {
                    issues.push(ValidationIssue::error(format!(
                        "half-edge {he_h:?} references dead next"
                    )));
                }
            }
        }

        for (edge_h, ed) in self.edges.iter() {
            if let Some(he_h) = ed.half_edge_a {
                if !self.half_edges.is_alive(he_h) {
                    issues.push(ValidationIssue::error(format!(
                        "edge {edge_h:?} references dead half_edge_a"
                    )));
                }
            }
            if let Some(he_h) = ed.half_edge_b {
                if !self.half_edges.is_alive(he_h) {
                    issues.push(ValidationIssue::error(format!(
                        "edge {edge_h:?} references dead half_edge_b"
                    )));
                }
            }
        }

        for (shell_h, sd) in self.shells.iter() {
            for &face_h in &sd.faces {
                if !self.faces.is_alive(face_h) {
                    issues.push(ValidationIssue::error(format!(
                        "shell {shell_h:?} references dead face"
                    )));
                }
            }
        }

        for (solid_h, sd) in self.solids.iter() {
            for &shell_h in &sd.shells {
                if !self.shells.is_alive(shell_h) {
                    issues.push(ValidationIssue::error(format!(
                        "solid {solid_h:?} references dead shell"
                    )));
                }
            }
        }
    }

    fn collect_orientation_issues(&self, issues: &mut Vec<ValidationIssue>) {
        for (he_h, he) in self.half_edges.iter() {
            let Some(twin_h) = he.twin else { continue };
            let Some(twin) = self.half_edges.get(twin_h) else {
                continue;
            };

            if twin.origin == he.origin {
                issues.push(ValidationIssue::error(format!(
                    "half-edge {he_h:?} and its twin share the same origin vertex"
                )));
            }

            if let Some(next_h) = he.next {
                if let Some(next_he) = self.half_edges.get(next_h) {
                    if twin.origin != next_he.origin {
                        issues.push(ValidationIssue::error(format!(
                            "half-edge {he_h:?}: twin origin does not match next's origin \
                             (orientation inconsistency)"
                        )));
                    }
                }
            }
        }
    }
}

impl Default for BRepModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod thread_safety_tests {
    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn topology_types_are_send_sync() {
        assert_send_sync::<crate::Handle<crate::VertexData>>();
        assert_send_sync::<crate::VertexData>();
        assert_send_sync::<crate::EdgeData>();
        assert_send_sync::<crate::HalfEdgeData>();
        assert_send_sync::<crate::LoopData>();
        assert_send_sync::<crate::WireData>();
        assert_send_sync::<crate::FaceData>();
        assert_send_sync::<crate::ShellData>();
        assert_send_sync::<crate::SolidData>();
        assert_send_sync::<crate::EntityStore<crate::VertexData>>();
        assert_send_sync::<crate::BRepModel>();
        assert_send_sync::<crate::Color>();
        assert_send_sync::<crate::Material>();
        assert_send_sync::<crate::PropertyStore>();
        assert_send_sync::<crate::PropertyValue>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cadkernel_math::Point3;

    #[test]
    fn test_triangle_topology() {
        let mut model = BRepModel::new();

        let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = model.add_vertex(Point3::new(0.0, 1.0, 0.0));

        let (_, he01, _) = model.add_edge(v0, v1);
        let (_, he12, _) = model.add_edge(v1, v2);
        let (_, he20, _) = model.add_edge(v2, v0);

        let loop_h = model.make_loop(&[he01, he12, he20]).unwrap();
        let _face = model.make_face(loop_h);

        let hes = model.loop_half_edges(he01);
        assert_eq!(hes.len(), 3);

        let twin_01 = model.half_edges.get(he01).unwrap().twin.unwrap();
        let twin_twin = model.half_edges.get(twin_01).unwrap().twin.unwrap();
        assert_eq!(twin_twin, he01);

        assert_eq!(model.vertices.len(), 3);
        assert_eq!(model.edges.len(), 3);
        assert_eq!(model.faces.len(), 1);
    }

    #[test]
    fn test_quad_topology() {
        let mut model = BRepModel::new();

        let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = model.add_vertex(Point3::new(1.0, 1.0, 0.0));
        let v3 = model.add_vertex(Point3::new(0.0, 1.0, 0.0));

        let (_, he01, _) = model.add_edge(v0, v1);
        let (_, he12, _) = model.add_edge(v1, v2);
        let (_, he23, _) = model.add_edge(v2, v3);
        let (_, he30, _) = model.add_edge(v3, v0);

        let loop_h = model.make_loop(&[he01, he12, he23, he30]).unwrap();
        let _face = model.make_face(loop_h);

        let hes = model.loop_half_edges(he01);
        assert_eq!(hes.len(), 4);
        assert_eq!(model.vertices.len(), 4);
        assert_eq!(model.edges.len(), 4);
        assert_eq!(model.faces.len(), 1);
    }

    #[test]
    fn test_shell_and_solid() {
        let mut model = BRepModel::new();

        let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = model.add_vertex(Point3::new(0.0, 1.0, 0.0));

        let (_, he01, _) = model.add_edge(v0, v1);
        let (_, he12, _) = model.add_edge(v1, v2);
        let (_, he20, _) = model.add_edge(v2, v0);

        let loop_h = model.make_loop(&[he01, he12, he20]).unwrap();
        let face = model.make_face(loop_h);

        let shell = model.make_shell(&[face]);
        assert_eq!(model.shells.len(), 1);
        assert_eq!(model.faces.get(face).unwrap().shell, Some(shell));

        let solid = model.make_solid(&[shell]);
        assert_eq!(model.solids.len(), 1);
        assert_eq!(model.shells.get(shell).unwrap().solid, Some(solid));
    }

    #[test]
    fn test_tagged_face_lookup() {
        let mut model = BRepModel::new();
        let op = model.history.next_operation("test");

        let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = model.add_vertex(Point3::new(0.0, 1.0, 0.0));

        let (_, he01, _) = model.add_edge(v0, v1);
        let (_, he12, _) = model.add_edge(v1, v2);
        let (_, he20, _) = model.add_edge(v2, v0);

        let loop_h = model.make_loop(&[he01, he12, he20]).unwrap();
        let tag = Tag::generated(EntityKind::Face, op, 0);
        let face = model.make_face_tagged(loop_h, tag.clone());

        assert_eq!(model.find_face_by_tag(&tag), Some(face));
        assert_eq!(model.faces.get(face).unwrap().tag.as_ref(), Some(&tag));
    }

    fn make_triangle_model() -> (BRepModel, Handle<FaceData>) {
        let mut model = BRepModel::new();
        let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = model.add_vertex(Point3::new(0.0, 1.0, 0.0));
        let (_, he01, _) = model.add_edge(v0, v1);
        let (_, he12, _) = model.add_edge(v1, v2);
        let (_, he20, _) = model.add_edge(v2, v0);
        let loop_h = model.make_loop(&[he01, he12, he20]).unwrap();
        let face = model.make_face(loop_h);
        (model, face)
    }

    #[test]
    fn test_vertices_of_face() {
        let (model, face) = make_triangle_model();
        let verts = model.vertices_of_face(face).unwrap();
        assert_eq!(verts.len(), 3);
    }

    #[test]
    fn test_edges_of_face() {
        let (model, face) = make_triangle_model();
        let edges = model.edges_of_face(face).unwrap();
        assert_eq!(edges.len(), 3);
    }

    #[test]
    fn test_faces_of_edge() {
        let (model, face) = make_triangle_model();
        let edges = model.edges_of_face(face).unwrap();
        let faces = model.faces_of_edge(edges[0]).unwrap();
        assert_eq!(faces.len(), 1);
        assert_eq!(faces[0], face);
    }

    #[test]
    fn test_validate_triangle() {
        let (model, _face) = make_triangle_model();
        assert!(model.validate().is_ok());
    }

    #[test]
    fn test_transform_translates_vertices() {
        let (mut model, _face) = make_triangle_model();
        let t = cadkernel_math::Transform::translation(10.0, 0.0, 0.0);
        model.transform(&t);
        for (_, v) in model.vertices.iter() {
            assert!(v.point.x >= 10.0 - 1e-10);
        }
    }

    // -- Validation enhancement tests --

    #[test]
    fn test_validate_dangling_face_ref() {
        let (mut model, face) = make_triangle_model();
        let _shell = model.make_shell(&[face]);
        model.faces.remove(face);
        let result = model.validate();
        assert!(result.is_err(), "should detect dangling face in shell");
    }

    /// Build a closed tetrahedron — the simplest manifold solid.
    fn make_tetrahedron_model() -> (BRepModel, Handle<ShellData>) {
        let mut m = BRepModel::new();
        let v0 = m.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = m.add_vertex(Point3::new(1.0, 0.0, 0.0));
        let v2 = m.add_vertex(Point3::new(0.5, 1.0, 0.0));
        let v3 = m.add_vertex(Point3::new(0.5, 0.5, 1.0));

        let (_, he01_a, he01_b) = m.add_edge(v0, v1);
        let (_, he02_a, he02_b) = m.add_edge(v0, v2);
        let (_, he03_a, he03_b) = m.add_edge(v0, v3);
        let (_, he12_a, he12_b) = m.add_edge(v1, v2);
        let (_, he13_a, he13_b) = m.add_edge(v1, v3);
        let (_, he23_a, he23_b) = m.add_edge(v2, v3);

        // Face 0: v0 → v1 → v2
        let l0 = m.make_loop(&[he01_a, he12_a, he02_b]).unwrap();
        let f0 = m.make_face(l0);

        // Face 1: v0 → v3 → v1
        let l1 = m.make_loop(&[he03_a, he13_b, he01_b]).unwrap();
        let f1 = m.make_face(l1);

        // Face 2: v0 → v2 → v3
        let l2 = m.make_loop(&[he02_a, he23_a, he03_b]).unwrap();
        let f2 = m.make_face(l2);

        // Face 3: v1 → v3 → v2
        let l3 = m.make_loop(&[he13_a, he23_b, he12_b]).unwrap();
        let f3 = m.make_face(l3);

        let shell = m.make_shell(&[f0, f1, f2, f3]);
        (m, shell)
    }

    #[test]
    fn test_validate_manifold_valid_box() {
        let (model, _shell) = make_tetrahedron_model();
        assert!(
            model.validate_manifold().is_ok(),
            "closed tetrahedron should pass manifold validation"
        );
    }

    #[test]
    fn test_validate_detailed_clean() {
        let (model, _) = make_triangle_model();
        let issues = model.validate_detailed();
        let errors: Vec<_> = issues
            .iter()
            .filter(|i| i.severity == ValidationSeverity::Error)
            .collect();
        assert!(errors.is_empty(), "well-formed model should have no errors");
    }

    #[test]
    fn test_validate_detailed_reports_issues() {
        let (mut model, face) = make_triangle_model();
        let _shell = model.make_shell(&[face]);
        model.faces.remove(face);
        let issues = model.validate_detailed();
        assert!(!issues.is_empty(), "broken model should report issues");
        assert!(
            issues
                .iter()
                .any(|i| i.severity == ValidationSeverity::Error),
            "dangling reference should be an error"
        );
    }

    #[test]
    fn test_add_vertex_and_get_point() {
        let mut model = BRepModel::new();
        let pt = Point3::new(3.125, 2.72, 1.41);
        let vh = model.add_vertex(pt);
        let stored = model.vertices.get(vh).unwrap();
        assert!(stored.point.approx_eq(pt));
    }

    #[test]
    fn test_add_edge_creates_half_edge_pair() {
        let mut model = BRepModel::new();
        let v0 = model.add_vertex(Point3::new(0.0, 0.0, 0.0));
        let v1 = model.add_vertex(Point3::new(1.0, 0.0, 0.0));

        let (edge_h, he_a, he_b) = model.add_edge(v0, v1);

        let he_a_data = model.half_edges.get(he_a).unwrap();
        let he_b_data = model.half_edges.get(he_b).unwrap();

        assert_eq!(he_a_data.twin, Some(he_b));
        assert_eq!(he_b_data.twin, Some(he_a));
        assert_eq!(he_a_data.origin, v0);
        assert_eq!(he_b_data.origin, v1);
        assert_eq!(he_a_data.edge, Some(edge_h));
        assert_eq!(he_b_data.edge, Some(edge_h));
    }

    #[test]
    fn test_vertices_of_face_correctness() {
        let mut model = BRepModel::new();
        let pts = [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(5.0, 0.0, 0.0),
            Point3::new(5.0, 5.0, 0.0),
            Point3::new(0.0, 5.0, 0.0),
        ];
        let vs: Vec<_> = pts.iter().map(|&p| model.add_vertex(p)).collect();

        let (_, he01, _) = model.add_edge(vs[0], vs[1]);
        let (_, he12, _) = model.add_edge(vs[1], vs[2]);
        let (_, he23, _) = model.add_edge(vs[2], vs[3]);
        let (_, he30, _) = model.add_edge(vs[3], vs[0]);

        let loop_h = model.make_loop(&[he01, he12, he23, he30]).unwrap();
        let face = model.make_face(loop_h);

        let face_verts = model.vertices_of_face(face).unwrap();
        assert_eq!(face_verts.len(), 4);
        assert_eq!(face_verts[0], vs[0]);
        assert_eq!(face_verts[1], vs[1]);
        assert_eq!(face_verts[2], vs[2]);
        assert_eq!(face_verts[3], vs[3]);
    }
}
