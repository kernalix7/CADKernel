//! PartDesign Body container for feature tree management.
//!
//! The index-based methods are the legacy container API: suppression removes
//! entries from the list and the current tip is exposed through
//! [`Body::tip_solid`]. The feature-id methods are the non-destructive body
//! chain API used by `cadkernel-api` for replayable PartDesign features.

use cadkernel_core::{KernelError, KernelResult};
use cadkernel_topology::{Handle, SolidData};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

/// Directed dependency graph between body feature indices.
///
/// Edges point from an upstream feature to a dependent downstream feature.
/// `reverse_edges` is kept in sync to make parent lookup and dirty
/// propagation cheap for recompute scheduling.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeatureGraph {
    edges: HashMap<u32, Vec<u32>>,
    reverse_edges: HashMap<u32, Vec<u32>>,
}

impl FeatureGraph {
    /// Creates an empty feature dependency graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true when no dependency edges are stored.
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty() && self.reverse_edges.is_empty()
    }

    /// Number of unique dependency edges.
    pub fn edge_count(&self) -> usize {
        self.edges.values().map(Vec::len).sum()
    }

    /// Adds a dependency edge `from -> to`.
    pub fn add_edge(&mut self, from: u32, to: u32) -> KernelResult<()> {
        let dependents = self.edges.entry(from).or_default();
        if !dependents.contains(&to) {
            dependents.push(to);
            dependents.sort_unstable();
        }

        let parents = self.reverse_edges.entry(to).or_default();
        if !parents.contains(&from) {
            parents.push(from);
            parents.sort_unstable();
        }

        self.edges.entry(to).or_default();
        self.reverse_edges.entry(from).or_default();
        Ok(())
    }

    /// Returns direct downstream dependents of `feature_index`.
    pub fn dependents_of(&self, feature_index: u32) -> Vec<u32> {
        self.edges.get(&feature_index).cloned().unwrap_or_default()
    }

    /// Returns direct upstream parents of `feature_index`.
    pub fn parents_of(&self, feature_index: u32) -> Vec<u32> {
        self.reverse_edges
            .get(&feature_index)
            .cloned()
            .unwrap_or_default()
    }

    /// Returns all nodes currently present in the graph.
    pub fn nodes(&self) -> Vec<u32> {
        let mut nodes = BTreeSet::new();
        for (&from, tos) in &self.edges {
            nodes.insert(from);
            for &to in tos {
                nodes.insert(to);
            }
        }
        for (&to, froms) in &self.reverse_edges {
            nodes.insert(to);
            for &from in froms {
                nodes.insert(from);
            }
        }
        nodes.into_iter().collect()
    }

    /// Returns a Kahn topological order, or an invalid-argument error on cycles.
    pub fn topological_sort(&self) -> KernelResult<Vec<u32>> {
        if let Some(cycle) = self.detect_cycles() {
            return Err(KernelError::InvalidArgument(format!(
                "cyclic feature graph: {cycle:?}"
            )));
        }

        let nodes = self.nodes();
        let mut indegree: HashMap<u32, usize> =
            nodes.iter().copied().map(|node| (node, 0)).collect();
        for tos in self.edges.values() {
            for &to in tos {
                let entry = indegree.entry(to).or_insert(0);
                *entry += 1;
            }
        }

        let mut ready: BTreeSet<u32> = indegree
            .iter()
            .filter_map(|(&node, &degree)| (degree == 0).then_some(node))
            .collect();
        let mut ordered = Vec::with_capacity(indegree.len());

        while let Some(node) = ready.pop_first() {
            ordered.push(node);
            if let Some(dependents) = self.edges.get(&node) {
                for &dependent in dependents {
                    if let Some(degree) = indegree.get_mut(&dependent) {
                        *degree = degree.saturating_sub(1);
                        if *degree == 0 {
                            ready.insert(dependent);
                        }
                    }
                }
            }
        }

        if ordered.len() != indegree.len() {
            return Err(KernelError::InvalidArgument(
                "cyclic feature graph".to_string(),
            ));
        }

        Ok(ordered)
    }

    /// Detects a directed cycle using Tarjan strongly connected components.
    pub fn detect_cycles(&self) -> Option<Vec<u32>> {
        let mut tarjan = Tarjan::new(self);
        for node in self.nodes() {
            if !tarjan.indices.contains_key(&node) {
                tarjan.strong_connect(node);
                if tarjan.cycle.is_some() {
                    return tarjan.cycle;
                }
            }
        }
        None
    }

    /// Returns `from` and every downstream dependent reached by BFS.
    pub fn dirty_propagate(&self, from: u32) -> Vec<u32> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut dirty = Vec::new();

        visited.insert(from);
        queue.push_back(from);

        while let Some(node) = queue.pop_front() {
            dirty.push(node);
            let mut dependents = self.dependents_of(node);
            dependents.sort_unstable();
            for dependent in dependents {
                if visited.insert(dependent) {
                    queue.push_back(dependent);
                }
            }
        }

        dirty
    }

    fn linear(feature_count: usize) -> Self {
        let mut graph = Self::new();
        for index in 1..feature_count {
            let _ = graph.add_edge((index - 1) as u32, index as u32);
        }
        graph
    }
}

struct Tarjan<'a> {
    graph: &'a FeatureGraph,
    next_index: usize,
    indices: HashMap<u32, usize>,
    lowlinks: HashMap<u32, usize>,
    stack: Vec<u32>,
    on_stack: HashSet<u32>,
    cycle: Option<Vec<u32>>,
}

impl<'a> Tarjan<'a> {
    fn new(graph: &'a FeatureGraph) -> Self {
        Self {
            graph,
            next_index: 0,
            indices: HashMap::new(),
            lowlinks: HashMap::new(),
            stack: Vec::new(),
            on_stack: HashSet::new(),
            cycle: None,
        }
    }

    fn strong_connect(&mut self, node: u32) {
        self.indices.insert(node, self.next_index);
        self.lowlinks.insert(node, self.next_index);
        self.next_index += 1;
        self.stack.push(node);
        self.on_stack.insert(node);

        let mut dependents = self.graph.dependents_of(node);
        dependents.sort_unstable();
        for dependent in dependents {
            if self.cycle.is_some() {
                return;
            }
            if !self.indices.contains_key(&dependent) {
                self.strong_connect(dependent);
                let Some(child_low) = self.lowlinks.get(&dependent).copied() else {
                    continue;
                };
                if let Some(node_low) = self.lowlinks.get_mut(&node) {
                    *node_low = (*node_low).min(child_low);
                }
            } else if self.on_stack.contains(&dependent) {
                let Some(dep_index) = self.indices.get(&dependent).copied() else {
                    continue;
                };
                if let Some(node_low) = self.lowlinks.get_mut(&node) {
                    *node_low = (*node_low).min(dep_index);
                }
            }
        }

        if self.indices.get(&node) == self.lowlinks.get(&node) {
            let mut component = Vec::new();
            while let Some(member) = self.stack.pop() {
                self.on_stack.remove(&member);
                component.push(member);
                if member == node {
                    break;
                }
            }
            component.sort_unstable();
            let self_loop = component.len() == 1
                && component
                    .first()
                    .is_some_and(|member| self.graph.dependents_of(*member).contains(member));
            if component.len() > 1 || self_loop {
                self.cycle = Some(component);
            }
        }
    }
}

/// A PartDesign body that maintains a feature tree and tip solid.
///
/// Features are appended sequentially via [`add_feature`](Self::add_feature).
/// The `tip` points to an index in `features`; [`tip_solid`](Self::tip_solid)
/// returns the cached solid handle for legacy callers.
#[derive(Debug, Clone)]
pub struct Body {
    pub id: u64,
    pub name: String,
    pub features: Vec<BodyFeature>,
    pub graph: FeatureGraph,
    pub tip: Option<usize>,
    pub base_plane_origin: [f64; 3],
    pub base_plane_normal: [f64; 3],
    pub current_solid: Option<u32>,
}

/// A single feature entry in a [`Body`]'s feature tree.
///
/// Stores the feature name, kind, replay spec metadata, and the last cached
/// solid handle. `solid` is retained for the legacy index-based API.
#[derive(Debug, Clone)]
pub struct BodyFeature {
    pub feature_id: u64,
    pub name: String,
    pub kind: FeatureKind,
    pub spec: Option<serde_json::Value>,
    pub spec_kind: String,
    pub suppressed: bool,
    pub solid: Handle<SolidData>,
    pub cached_solid: Option<Handle<SolidData>>,
}

/// The kind of feature operation that produced a [`BodyFeature`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureKind {
    Pad,
    Pocket,
    Revolve,
    Groove,
    Hole,
    Sweep,
    Loft,
    Helix,
    Fillet,
    Chamfer,
    Shell,
    Draft,
    Mirror,
    Pattern,
}

impl FeatureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pad => "pad",
            Self::Pocket => "pocket",
            Self::Revolve => "revolve",
            Self::Groove => "groove",
            Self::Hole => "hole",
            Self::Sweep => "sweep",
            Self::Loft => "loft",
            Self::Helix => "helix",
            Self::Fillet => "fillet",
            Self::Chamfer => "chamfer",
            Self::Shell => "shell",
            Self::Draft => "draft",
            Self::Mirror => "mirror",
            Self::Pattern => "pattern",
        }
    }
}

impl Body {
    /// Creates a new empty body with the given name.
    pub fn new(name: &str) -> Self {
        Self::new_with_plane(0, name, [0.0; 3], [0.0, 0.0, 1.0])
    }

    /// Creates a new empty body with explicit persistent id and base plane.
    pub fn new_with_plane(
        id: u64,
        name: &str,
        base_plane_origin: [f64; 3],
        base_plane_normal: [f64; 3],
    ) -> Self {
        Self {
            id,
            name: name.to_string(),
            features: Vec::new(),
            graph: FeatureGraph::new(),
            tip: None,
            base_plane_origin,
            base_plane_normal,
            current_solid: None,
        }
    }

    /// Adds a feature to the body, updating the tip to the new solid.
    pub fn add_feature(&mut self, name: &str, kind: FeatureKind, solid: Handle<SolidData>) {
        let index = self.features.len();
        self.features.push(BodyFeature {
            feature_id: 0,
            name: name.to_string(),
            kind,
            spec: None,
            spec_kind: String::new(),
            suppressed: false,
            solid,
            cached_solid: Some(solid),
        });
        self.add_default_dependency(index);
        self.tip = self.features.len().checked_sub(1);
    }

    /// Adds a replayable feature entry with an opaque serialized spec.
    pub fn add_feature_with_spec(
        &mut self,
        feature_id: u64,
        name: &str,
        kind: FeatureKind,
        spec_kind: &str,
        spec_json: serde_json::Value,
    ) {
        let index = self.features.len();
        self.features.push(BodyFeature {
            feature_id,
            name: name.to_string(),
            kind,
            spec: Some(spec_json),
            spec_kind: spec_kind.to_string(),
            suppressed: false,
            solid: Handle::from_raw_parts(0, 0),
            cached_solid: None,
        });
        self.add_default_dependency(index);
        self.tip = self.features.len().checked_sub(1);
    }

    /// Returns the tip solid (last feature result), if any.
    pub fn tip_solid(&self) -> Option<Handle<SolidData>> {
        self.tip
            .and_then(|index| self.features.get(index))
            .map(|feature| feature.solid)
    }

    /// Returns the number of features in this body.
    pub fn feature_count(&self) -> usize {
        self.features.len()
    }

    /// Suppresses (skips) a feature at the given index during rebuild.
    ///
    /// Suppressed features are removed from the feature list. If the
    /// suppressed feature was the tip, the tip moves to the previous feature.
    pub fn suppress_feature(&mut self, index: usize) -> Option<BodyFeature> {
        if index >= self.features.len() {
            return None;
        }
        let removed = self.features.remove(index);
        if self.features.is_empty() {
            self.tip = None;
        } else if let Some(tip) = self.tip {
            if index <= tip {
                self.tip = Some(tip.saturating_sub(1).min(self.features.len() - 1));
            }
        }
        self.rebuild_linear_graph();
        Some(removed)
    }

    /// Sets the rebuild tip to the feature at the given index.
    ///
    /// Features after the tip index are kept but ignored when querying
    /// the current solid state. The tip solid is updated to the indexed feature.
    pub fn set_tip(&mut self, index: usize) -> bool {
        if index >= self.features.len() {
            return false;
        }
        self.tip = Some(index);
        true
    }

    /// Transfers a feature from another body into this body.
    ///
    /// Removes the feature at `feature_index` from `source_body` and appends
    /// it to the end of this body's feature list, updating tips in both bodies.
    pub fn move_object_to_body(
        &mut self,
        source_body: &mut Body,
        feature_index: usize,
    ) -> KernelResult<()> {
        if feature_index >= source_body.features.len() {
            return Err(KernelError::InvalidArgument(format!(
                "feature_index {} out of range (source has {} features)",
                feature_index,
                source_body.features.len(),
            )));
        }
        let feature = source_body.features.remove(feature_index);
        if source_body.features.is_empty() {
            source_body.tip = None;
        } else if let Some(tip) = source_body.tip {
            if feature_index <= tip {
                source_body.tip = Some(tip.saturating_sub(1).min(source_body.features.len() - 1));
            }
        }
        source_body.rebuild_linear_graph();
        self.features.push(feature);
        self.tip = self.features.len().checked_sub(1);
        self.rebuild_linear_graph();
        Ok(())
    }

    /// Reorders a feature from one position to another.
    ///
    /// Moves the feature at `from` to the `to` position, shifting other
    /// features accordingly. Updates the tip to the last feature.
    pub fn move_feature(&mut self, from: usize, to: usize) -> bool {
        if from >= self.features.len() || to >= self.features.len() {
            return false;
        }
        let feature = self.features.remove(from);
        self.features.insert(to, feature);
        self.tip = self.features.len().checked_sub(1);
        self.rebuild_linear_graph();
        true
    }

    /// Returns the index of a feature by persistent feature id.
    pub fn feature_index_of(&self, feature_id: u64) -> Option<usize> {
        self.features
            .iter()
            .position(|feature| feature.feature_id == feature_id)
    }

    /// Non-destructively toggles suppression for a feature id.
    pub fn suppress_feature_by_id(&mut self, feature_id: u64, suppressed: bool) -> bool {
        let Some(index) = self.feature_index_of(feature_id) else {
            return false;
        };
        self.features[index].suppressed = suppressed;
        true
    }

    /// Sets the active tip to the feature with the given id.
    pub fn set_tip_by_feature_id(&mut self, feature_id: u64) -> bool {
        let Some(index) = self.feature_index_of(feature_id) else {
            return false;
        };
        self.tip = Some(index);
        true
    }

    /// Moves a feature identified by persistent id to `to_position`.
    pub fn move_feature_by_feature_id(&mut self, feature_id: u64, to_position: usize) -> bool {
        let Some(from) = self.feature_index_of(feature_id) else {
            return false;
        };
        if to_position >= self.features.len() {
            return false;
        }
        let feature = self.features.remove(from);
        self.features.insert(to_position, feature);
        self.tip = self.features.len().checked_sub(1);
        true
    }

    /// Transfers a feature by persistent id from another body into this one.
    pub fn move_object_to_body_by_feature_id(
        &mut self,
        source_body: &mut Body,
        feature_id: u64,
    ) -> KernelResult<()> {
        let Some(index) = source_body.feature_index_of(feature_id) else {
            return Err(KernelError::InvalidArgument(format!(
                "feature_id {feature_id} not found in source body"
            )));
        };
        self.move_object_to_body(source_body, index)
    }

    /// Iterates active, non-suppressed features through the current tip.
    pub fn active_features(&self) -> impl Iterator<Item = &BodyFeature> {
        let limit = self.tip.map(|tip| tip + 1).unwrap_or(0);
        self.features
            .iter()
            .take(limit)
            .filter(|feature| !feature.suppressed)
    }

    /// Returns the stored graph, or a linear fallback for legacy bodies.
    pub fn dependency_graph(&self) -> FeatureGraph {
        if self.graph.is_empty() && self.features.len() > 1 {
            FeatureGraph::linear(self.features.len())
        } else {
            self.graph.clone()
        }
    }

    fn add_default_dependency(&mut self, index: usize) {
        if index > 0 {
            let _ = self.graph.add_edge((index - 1) as u32, index as u32);
        }
    }

    fn rebuild_linear_graph(&mut self) {
        self.graph = FeatureGraph::linear(self.features.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::make_box;
    use cadkernel_math::Point3;
    use cadkernel_topology::BRepModel;

    #[test]
    fn test_body_new() {
        let body = Body::new("Body1");
        assert_eq!(body.name, "Body1");
        assert_eq!(body.feature_count(), 0);
        assert!(body.tip_solid().is_none());
    }

    #[test]
    fn test_body_add_features() {
        let mut model = BRepModel::new();
        let r1 = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let r2 = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut body = Body::new("Body1");
        body.add_feature("Base Pad", FeatureKind::Pad, r1.solid);
        assert_eq!(body.feature_count(), 1);
        assert_eq!(body.tip_solid(), Some(r1.solid));

        body.add_feature("Pocket", FeatureKind::Pocket, r2.solid);
        assert_eq!(body.feature_count(), 2);
        assert_eq!(body.tip_solid(), Some(r2.solid));
    }

    #[test]
    fn test_body_suppress_feature() {
        let mut model = BRepModel::new();
        let r1 = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let r2 = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
        let r3 = make_box(&mut model, Point3::ORIGIN, 3.0, 3.0, 3.0).unwrap();

        let mut body = Body::new("Body1");
        body.add_feature("Pad1", FeatureKind::Pad, r1.solid);
        body.add_feature("Pad2", FeatureKind::Pad, r2.solid);
        body.add_feature("Pad3", FeatureKind::Pad, r3.solid);

        assert_eq!(body.feature_count(), 3);
        assert_eq!(body.tip_solid(), Some(r3.solid));

        // Suppress middle feature
        let removed = body.suppress_feature(1);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "Pad2");
        assert_eq!(body.feature_count(), 2);
        assert_eq!(body.tip_solid(), Some(r3.solid));

        // Suppress last feature (tip)
        let removed = body.suppress_feature(1);
        assert!(removed.is_some());
        assert_eq!(body.tip_solid(), Some(r1.solid));
    }

    #[test]
    fn test_body_set_tip() {
        let mut model = BRepModel::new();
        let r1 = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let r2 = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut body = Body::new("Body1");
        body.add_feature("Pad1", FeatureKind::Pad, r1.solid);
        body.add_feature("Pad2", FeatureKind::Pad, r2.solid);

        assert!(body.set_tip(0));
        assert_eq!(body.tip_solid(), Some(r1.solid));

        assert!(body.set_tip(1));
        assert_eq!(body.tip_solid(), Some(r2.solid));

        assert!(!body.set_tip(5));
    }

    #[test]
    fn test_body_move_feature() {
        let mut model = BRepModel::new();
        let r1 = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let r2 = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();
        let r3 = make_box(&mut model, Point3::ORIGIN, 3.0, 3.0, 3.0).unwrap();

        let mut body = Body::new("Body1");
        body.add_feature("Pad1", FeatureKind::Pad, r1.solid);
        body.add_feature("Pad2", FeatureKind::Pad, r2.solid);
        body.add_feature("Pad3", FeatureKind::Pad, r3.solid);

        assert!(body.move_feature(0, 2));
        assert_eq!(body.features[0].name, "Pad2");
        assert_eq!(body.features[1].name, "Pad3");
        assert_eq!(body.features[2].name, "Pad1");
        assert_eq!(body.tip_solid(), Some(r1.solid));

        assert!(!body.move_feature(0, 5));
    }

    #[test]
    fn test_body_feature_kinds() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut body = Body::new("TestBody");
        body.add_feature("pad", FeatureKind::Pad, r.solid);
        body.add_feature("fillet", FeatureKind::Fillet, r.solid);
        body.add_feature("chamfer", FeatureKind::Chamfer, r.solid);

        assert_eq!(body.features[0].kind, FeatureKind::Pad);
        assert_eq!(body.features[1].kind, FeatureKind::Fillet);
        assert_eq!(body.features[2].kind, FeatureKind::Chamfer);
    }

    #[test]
    fn test_move_object_to_body() {
        let mut model = BRepModel::new();
        let r1 = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let r2 = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut source = Body::new("Source");
        source.add_feature("Pad1", FeatureKind::Pad, r1.solid);
        source.add_feature("Pad2", FeatureKind::Pad, r2.solid);

        let mut target = Body::new("Target");

        target.move_object_to_body(&mut source, 0).unwrap();
        assert_eq!(source.feature_count(), 1);
        assert_eq!(target.feature_count(), 1);
        assert_eq!(target.features[0].name, "Pad1");
        assert_eq!(target.tip_solid(), Some(r1.solid));
        assert_eq!(source.tip_solid(), Some(r2.solid));
    }

    #[test]
    fn test_move_object_to_body_out_of_range() {
        let mut source = Body::new("Source");
        let mut target = Body::new("Target");
        assert!(target.move_object_to_body(&mut source, 0).is_err());
    }

    #[test]
    fn test_body_suppress_first_feature() {
        let mut model = BRepModel::new();
        let r1 = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let r2 = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut body = Body::new("Body1");
        body.add_feature("Pad1", FeatureKind::Pad, r1.solid);
        body.add_feature("Pad2", FeatureKind::Pad, r2.solid);

        let removed = body.suppress_feature(0);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "Pad1");
        assert_eq!(body.feature_count(), 1);
        assert_eq!(body.tip_solid(), Some(r2.solid));
    }

    #[test]
    fn test_body_suppress_out_of_range() {
        let mut body = Body::new("Body1");
        let removed = body.suppress_feature(0);
        assert!(removed.is_none());
    }

    #[test]
    fn test_body_set_tip_first() {
        let mut model = BRepModel::new();
        let r1 = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let r2 = make_box(&mut model, Point3::ORIGIN, 2.0, 2.0, 2.0).unwrap();

        let mut body = Body::new("Body1");
        body.add_feature("Pad1", FeatureKind::Pad, r1.solid);
        body.add_feature("Pad2", FeatureKind::Pad, r2.solid);

        assert!(body.set_tip(0));
        assert_eq!(body.tip_solid(), Some(r1.solid));
    }

    #[test]
    fn test_body_feature_revolve_groove() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut body = Body::new("Body1");
        body.add_feature("rev", FeatureKind::Revolve, r.solid);
        body.add_feature("groove", FeatureKind::Groove, r.solid);

        assert_eq!(body.features[0].kind, FeatureKind::Revolve);
        assert_eq!(body.features[1].kind, FeatureKind::Groove);
    }

    #[test]
    fn test_body_move_feature_invalid_from() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let mut body = Body::new("Body1");
        body.add_feature("Pad1", FeatureKind::Pad, r.solid);
        assert!(!body.move_feature(5, 0));
    }

    #[test]
    fn test_body_move_feature_invalid_to() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();
        let mut body = Body::new("Body1");
        body.add_feature("Pad1", FeatureKind::Pad, r.solid);
        assert!(!body.move_feature(0, 5));
    }

    #[test]
    fn test_body_multiple_kinds() {
        let mut model = BRepModel::new();
        let r = make_box(&mut model, Point3::ORIGIN, 1.0, 1.0, 1.0).unwrap();

        let mut body = Body::new("Body");
        for &kind in &[
            FeatureKind::Pad,
            FeatureKind::Pocket,
            FeatureKind::Revolve,
            FeatureKind::Groove,
            FeatureKind::Fillet,
            FeatureKind::Chamfer,
            FeatureKind::Mirror,
            FeatureKind::Pattern,
        ] {
            body.add_feature("f", kind, r.solid);
        }
        assert_eq!(body.feature_count(), 8);
    }
}
