use jungle_types::{JourneyAst, JourneyUpdateEvent, NodeLifecyclePhase, RunnerUpdateOut};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase<T> {
    Static,
    Live(T),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeState {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepKind {
    Step,
    Conditional,
    Select,
    Join,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClusterKind {
    While,
    Join,
    Transparent,
    Attempt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConditionalSide {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClusterLive {
    pub has_running: bool,
    pub has_failed: bool,
    pub has_completed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClusterSpec {
    pub nodes: Vec<u32>,
    pub parent: Option<usize>,
    pub padding: Option<f32>,
}

impl ClusterSpec {
    fn new(nodes: Vec<u32>) -> Self {
        Self {
            nodes,
            parent: None,
            padding: None,
        }
    }

    fn padding(mut self, padding: f32) -> Self {
        self.padding = Some(padding);
        self
    }

    fn parent(mut self, parent: usize) -> Self {
        self.parent = Some(parent);
        self
    }
}

#[derive(Clone)]
pub struct Dag {
    pub nodes: Vec<NodeDisplay>,
    pub node_map: HashMap<u32, NodeDisplay>,
    pub edges: Vec<(u32, u32)>,
    pub clusters: Vec<ClusterSpec>,
    pub derived: DagDerived,
    pub cluster_info: Vec<ClusterInfo>,
}

impl Dag {
    pub fn from_ast(ast: JourneyAst) -> Self {
        let mut builder = GraphBuilder::default();
        builder.flatten(&ast);

        let descendant_runtime_ids_by_runtime_id =
            builder.descendant_runtime_ids_by_runtime_id.clone();
        let nodes = std::mem::take(&mut builder.nodes);
        let edges = std::mem::take(&mut builder.edges);
        let cluster_info = std::mem::take(&mut builder.cluster_info);
        let node_map = nodes
            .iter()
            .map(|node| (node.id, node.clone()))
            .collect::<HashMap<_, _>>();
        let derived = DagDerived::build(
            &nodes,
            &node_map,
            &edges,
            &cluster_info,
            &builder.conditional_branches,
            &descendant_runtime_ids_by_runtime_id,
        );

        Self {
            nodes,
            node_map,
            edges,
            clusters: builder.clusters,
            derived,
            cluster_info,
        }
    }

    pub fn cluster_node_id(&self, index: usize) -> Option<u32> {
        let offset = u32::try_from(index).ok()?;
        Some(
            self.derived
                .max_node_id
                .saturating_add(1)
                .saturating_add(offset),
        )
    }
}

#[derive(Clone)]
pub struct DagDerived {
    pub condition_successor_runtime_ids: HashMap<u32, Vec<u32>>,
    pub conditional_branches: Vec<ConditionalBranchInfo>,
    pub cluster_member_runtime_ids: Vec<Vec<u32>>,
    pub cluster_successor_runtime_ids: Vec<Vec<u32>>,
    pub cluster_entry_runtime_ids: Vec<Vec<u32>>,
    pub memberships: HashMap<u32, Vec<(usize, usize)>>,
    pub nearest_attempt_boundary_cluster_index_by_display_id: HashMap<u32, usize>,
    pub max_node_id: u32,
    pub runtime_by_display_id: HashMap<u32, Option<u32>>,
    pub proxy_runtime_ids_by_display_id: HashMap<u32, Vec<u32>>,
    pub descendant_runtime_ids_by_runtime_id: HashMap<u32, Vec<u32>>,
}

impl DagDerived {
    fn build(
        nodes: &[NodeDisplay],
        node_map: &HashMap<u32, NodeDisplay>,
        edges: &[(u32, u32)],
        cluster_info: &[ClusterInfo],
        conditional_branches: &[ConditionalBranchInfo],
        descendant_runtime_ids_by_runtime_id: &HashMap<u32, Vec<u32>>,
    ) -> Self {
        let mut condition_successor_runtime_ids = HashMap::<u32, Vec<u32>>::new();
        let mut condition_successor_seen = HashMap::<u32, BTreeSet<u32>>::new();
        for (from, to) in edges {
            let Some(source) = node_map.get(from) else {
                continue;
            };
            if !source.is_conditional_branch {
                continue;
            }
            let Some(target) = node_map.get(to) else {
                continue;
            };
            let Some(runtime_id) = target.runtime_node_id else {
                continue;
            };
            let seen = condition_successor_seen.entry(*from).or_default();
            if seen.insert(runtime_id) {
                condition_successor_runtime_ids
                    .entry(*from)
                    .or_default()
                    .push(runtime_id);
            }
        }

        let mut cluster_member_runtime_ids = vec![Vec::<u32>::new(); cluster_info.len()];
        for (index, cluster) in cluster_info.iter().enumerate() {
            cluster_member_runtime_ids[index] = cluster.member_runtime_ids.clone();
        }

        let mut cluster_entry_runtime_ids = vec![Vec::<u32>::new(); cluster_info.len()];
        for (index, cluster) in cluster_info.iter().enumerate() {
            cluster_entry_runtime_ids[index] = cluster.root_runtime_ids.clone();
        }

        let mut memberships = HashMap::<u32, Vec<(usize, usize)>>::new();
        for (index, cluster) in cluster_info.iter().enumerate() {
            for node_id in &cluster.nodes {
                memberships
                    .entry(*node_id)
                    .or_default()
                    .push((cluster.depth, index));
            }
        }
        for entry in memberships.values_mut() {
            entry.sort_by_key(|(depth, _)| *depth);
        }

        let mut nearest_attempt_boundary_cluster_index_by_display_id = HashMap::<u32, usize>::new();
        for (index, cluster) in cluster_info.iter().enumerate() {
            if !matches!(cluster.kind, ClusterKind::Attempt) {
                continue;
            }
            for node_id in &cluster.nodes {
                let should_replace = nearest_attempt_boundary_cluster_index_by_display_id
                    .get(node_id)
                    .copied()
                    .map(|current_index| cluster_info[current_index].depth < cluster.depth)
                    .unwrap_or(true);
                if should_replace {
                    nearest_attempt_boundary_cluster_index_by_display_id.insert(*node_id, index);
                }
            }
        }

        let runtime_by_display_id = node_map
            .iter()
            .map(|(display_id, node)| (*display_id, node.runtime_node_id))
            .collect::<HashMap<_, _>>();
        let proxy_runtime_ids_by_display_id = node_map
            .iter()
            .map(|(display_id, node)| (*display_id, node.proxy_runtime_ids.clone()))
            .collect::<HashMap<_, _>>();

        Self {
            condition_successor_runtime_ids,
            conditional_branches: conditional_branches.to_vec(),
            cluster_member_runtime_ids,
            cluster_successor_runtime_ids: compute_cluster_successor_runtime_ids(
                edges,
                node_map,
                cluster_info,
            ),
            cluster_entry_runtime_ids,
            memberships,
            nearest_attempt_boundary_cluster_index_by_display_id,
            max_node_id: nodes.iter().map(|node| node.id).max().unwrap_or(0),
            runtime_by_display_id,
            proxy_runtime_ids_by_display_id,
            descendant_runtime_ids_by_runtime_id: descendant_runtime_ids_by_runtime_id.clone(),
        }
    }
}

#[derive(Clone)]
pub struct NodeDisplay {
    pub id: u32,
    pub label: String,
    pub metadata: Option<String>,
    pub runtime_node_id: Option<u32>,
    pub proxy_runtime_ids: Vec<u32>,
    pub is_conditional_branch: bool,
    pub is_select: bool,
    pub is_join: bool,
}

impl NodeDisplay {
    pub fn kind(&self) -> StepKind {
        if self.is_conditional_branch {
            StepKind::Conditional
        } else if self.is_select {
            StepKind::Select
        } else if self.is_join {
            StepKind::Join
        } else {
            StepKind::Step
        }
    }
}

#[derive(Clone)]
pub struct ClusterInfo {
    pub id: u32,
    pub kind: ClusterKind,
    pub runtime_node_id: u32,
    pub label: String,
    pub metadata: Option<String>,
    pub parent: Option<usize>,
    pub nodes: Vec<u32>,
    pub root_nodes: Vec<u32>,
    pub member_runtime_ids: Vec<u32>,
    pub root_runtime_ids: Vec<u32>,
    pub depth: usize,
}

#[derive(Clone)]
pub struct ConditionalBranchInfo {
    pub condition_display_id: u32,
    pub left_root_display_ids: Vec<u32>,
    pub right_root_display_ids: Vec<u32>,
    pub left_member_display_ids: Vec<u32>,
    pub right_member_display_ids: Vec<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct LiveDagState {
    pub active_runtime_ids: BTreeSet<u32>,
    pub finished_runtime_ids: BTreeSet<u32>,
    pub failed_runtime_ids: BTreeSet<u32>,
    pub lifecycle_runtime_ids: BTreeSet<u32>,
    pub descendant_runtime_ids_by_runtime_id: HashMap<u32, Vec<u32>>,
    pub runtime_activation_paths: HashMap<u32, Vec<u64>>,
    pub runtime_update_sequence: HashMap<u32, usize>,
    pub latest_event_count: usize,
}

impl LiveDagState {
    pub fn bind_model(&mut self, model: &Dag) {
        self.descendant_runtime_ids_by_runtime_id =
            model.derived.descendant_runtime_ids_by_runtime_id.clone();
    }

    pub fn apply_update(&mut self, update: JourneyUpdateEvent) -> bool {
        let mut highlight_changed = false;
        let sequence = update.sequence_id as usize;
        self.latest_event_count = sequence;
        match update.event {
            RunnerUpdateOut::EffectInput { node_id, .. } => {
                if self.lifecycle_runtime_ids.contains(&node_id) {
                    return highlight_changed;
                }
                highlight_changed |= self.finished_runtime_ids.remove(&node_id);
                highlight_changed |= self.failed_runtime_ids.remove(&node_id);
                highlight_changed |= self.active_runtime_ids.insert(node_id);
                self.runtime_update_sequence.insert(node_id, sequence);
            }
            RunnerUpdateOut::EffectSuccessOutput { node_id, .. } => {
                if self.lifecycle_runtime_ids.contains(&node_id) {
                    return highlight_changed;
                }
                highlight_changed |= self.active_runtime_ids.remove(&node_id);
                highlight_changed |= self.finished_runtime_ids.insert(node_id);
                self.runtime_update_sequence.insert(node_id, sequence);
            }
            RunnerUpdateOut::EffectFailureOutput { node_id, .. } => {
                if self.lifecycle_runtime_ids.contains(&node_id) {
                    return highlight_changed;
                }
                highlight_changed |= self.active_runtime_ids.remove(&node_id);
                highlight_changed |= self.failed_runtime_ids.insert(node_id);
                self.runtime_update_sequence.insert(node_id, sequence);
            }
            RunnerUpdateOut::NodeLifecycle(node) => {
                if matches!(node.phase, NodeLifecyclePhase::Entered) {
                    highlight_changed |=
                        self.clear_stale_descendants(node.node_id, &node.activation_path);
                }
                self.lifecycle_runtime_ids.insert(node.node_id);
                highlight_changed |= self.active_runtime_ids.remove(&node.node_id);
                highlight_changed |= self.finished_runtime_ids.remove(&node.node_id);
                highlight_changed |= self.failed_runtime_ids.remove(&node.node_id);
                match node.phase {
                    NodeLifecyclePhase::Entered => {
                        highlight_changed |= self.active_runtime_ids.insert(node.node_id);
                    }
                    NodeLifecyclePhase::Succeeded => {
                        highlight_changed |= self.finished_runtime_ids.insert(node.node_id);
                    }
                    NodeLifecyclePhase::Failed => {
                        highlight_changed |= self.failed_runtime_ids.insert(node.node_id);
                    }
                }
                self.runtime_update_sequence.insert(node.node_id, sequence);
                self.runtime_activation_paths
                    .insert(node.node_id, node.activation_path);
            }
            RunnerUpdateOut::SleepScheduled { .. } | RunnerUpdateOut::SleepFired { .. } => {}
        }
        highlight_changed
    }

    fn clear_stale_descendants(
        &mut self,
        ancestor_runtime_id: u32,
        activation_path: &[u64],
    ) -> bool {
        let Some(descendants) = self
            .descendant_runtime_ids_by_runtime_id
            .get(&ancestor_runtime_id)
            .cloned()
        else {
            return false;
        };

        let mut changed = false;
        for runtime_id in descendants {
            if self
                .runtime_activation_paths
                .get(&runtime_id)
                .map(|path| {
                    path.len() <= activation_path.len() || path.starts_with(activation_path)
                })
                .unwrap_or(false)
            {
                continue;
            }
            changed |= self.active_runtime_ids.remove(&runtime_id);
            changed |= self.finished_runtime_ids.remove(&runtime_id);
            changed |= self.failed_runtime_ids.remove(&runtime_id);
            self.lifecycle_runtime_ids.remove(&runtime_id);
            changed |= self.runtime_update_sequence.remove(&runtime_id).is_some();
            changed |= self.runtime_activation_paths.remove(&runtime_id).is_some();
        }
        changed
    }
}

#[derive(Debug, Clone)]
pub struct DagSnapshot {
    pub has_live_data: bool,
    pub repaired_node_states: HashMap<u32, RuntimeState>,
    pub node_states: HashMap<u32, RuntimeState>,
    pub active_conditional_sides: HashMap<u32, ConditionalSide>,
    pub skipped_conditional_branch_nodes: HashSet<u32>,
    pub runtime_sequence_floors: HashMap<u32, usize>,
    pub runtime_activation_prefixes: HashMap<u32, Vec<u64>>,
    pub cluster_live_states: Vec<ClusterLive>,
}

impl DagSnapshot {
    pub fn new(dag: &Dag, live_data: Option<&LiveDagState>) -> Self {
        let Some(live) = live_data else {
            return Self {
                has_live_data: false,
                repaired_node_states: HashMap::new(),
                node_states: HashMap::new(),
                active_conditional_sides: HashMap::new(),
                skipped_conditional_branch_nodes: HashSet::new(),
                runtime_sequence_floors: HashMap::new(),
                runtime_activation_prefixes: HashMap::new(),
                cluster_live_states: Vec::new(),
            };
        };

        let runtime_sequence_floors = runtime_sequence_floors_for_display(dag, live);
        let runtime_activation_prefixes = runtime_activation_prefixes_for_display(dag, live);
        let conditional_branch_membership = conditional_branch_membership(dag);
        let active_conditional_sides = active_conditional_branch_sides(
            dag,
            live,
            &runtime_sequence_floors,
            &runtime_activation_prefixes,
        );
        let skipped_conditional_branch_nodes =
            skipped_conditional_branch_nodes(dag, &active_conditional_sides);

        let repaired_node_states = repaired_live_states_for_display(
            dag,
            live,
            &runtime_sequence_floors,
            &runtime_activation_prefixes,
            &conditional_branch_membership,
            &active_conditional_sides,
            &skipped_conditional_branch_nodes,
        );

        let mut node_states = repaired_node_states.clone();
        let branch_root_has_activity = |display_ids: &[u32]| {
            display_ids.iter().any(|display_id| {
                dag.node_map
                    .get(display_id)
                    .and_then(|node| node.runtime_node_id)
                    .map(|runtime_id| {
                        runtime_observed_in_current_iteration(
                            live,
                            runtime_id,
                            &runtime_sequence_floors,
                            &runtime_activation_prefixes,
                        )
                    })
                    .unwrap_or(false)
            })
        };

        for branch in &dag.derived.conditional_branches {
            if !branch_root_has_activity(&branch.left_root_display_ids)
                && branch_root_has_activity(&branch.right_root_display_ids)
            {
                node_states.insert(branch.condition_display_id, RuntimeState::Pending);
            }
        }

        let cluster_live_states = dag
            .cluster_info
            .iter()
            .enumerate()
            .map(|(cluster_index, _)| {
                cluster_live_from_states(
                    live,
                    dag,
                    cluster_index,
                    &node_states,
                    &runtime_sequence_floors,
                    &runtime_activation_prefixes,
                )
            })
            .collect();

        Self {
            has_live_data: true,
            repaired_node_states,
            node_states,
            active_conditional_sides,
            skipped_conditional_branch_nodes,
            runtime_sequence_floors,
            runtime_activation_prefixes,
            cluster_live_states,
        }
    }

    pub fn node_phase(&self, display_id: u32) -> Phase<RuntimeState> {
        if !self.has_live_data {
            Phase::Static
        } else {
            Phase::Live(
                self.node_states
                    .get(&display_id)
                    .copied()
                    .unwrap_or(RuntimeState::Pending),
            )
        }
    }

    pub fn cluster_phase(&self, cluster_index: usize) -> Phase<ClusterLive> {
        if !self.has_live_data {
            Phase::Static
        } else {
            Phase::Live(
                self.cluster_live_states
                    .get(cluster_index)
                    .copied()
                    .unwrap_or(ClusterLive {
                        has_running: false,
                        has_failed: false,
                        has_completed: false,
                    }),
            )
        }
    }
}

#[derive(Debug, Clone)]
pub struct DagProjection {
    pub nodes: Vec<u32>,
    pub edges: Vec<(u32, u32)>,
    pub visible_real_nodes: HashSet<u32>,
    pub collapsed_cluster_by_display: HashMap<u32, usize>,
    pub visible_clusters: Vec<VisibleCluster>,
}

#[derive(Debug, Clone)]
pub struct VisibleCluster {
    pub source_index: usize,
    pub member_nodes: Vec<u32>,
    pub parent_visible_index: Option<usize>,
    pub padding: f32,
}

impl DagProjection {
    pub fn new(dag: &Dag, collapsed_clusters: &HashSet<usize>) -> Self {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum VisibleOwner {
            Node(u32),
            Cluster(usize),
        }

        let cluster_hidden_by_collapsed_ancestor = |cluster_index: usize| -> bool {
            let mut parent = dag.cluster_info[cluster_index].parent;
            while let Some(parent_index) = parent {
                if collapsed_clusters.contains(&parent_index) {
                    return true;
                }
                parent = dag.cluster_info[parent_index].parent;
            }
            false
        };

        let owner_for_node = |node_id: u32| -> VisibleOwner {
            if let Some(candidates) = dag.derived.memberships.get(&node_id) {
                for (_, index) in candidates {
                    if collapsed_clusters.contains(index) {
                        return VisibleOwner::Cluster(*index);
                    }
                }
            }
            VisibleOwner::Node(node_id)
        };

        let owner_to_display = |owner: VisibleOwner| -> Option<u32> {
            match owner {
                VisibleOwner::Node(node_id) => Some(node_id),
                VisibleOwner::Cluster(index) => dag.cluster_node_id(index),
            }
        };

        let mut visible_ids = BTreeSet::new();
        let mut visible_real_nodes = HashSet::<u32>::new();
        let mut collapsed_cluster_by_display = HashMap::<u32, usize>::new();

        for node in &dag.nodes {
            let owner = owner_for_node(node.id);
            if owner != VisibleOwner::Node(node.id) {
                continue;
            }
            visible_ids.insert(node.id);
            visible_real_nodes.insert(node.id);
        }

        for (index, _) in dag.cluster_info.iter().enumerate() {
            if !collapsed_clusters.contains(&index) {
                continue;
            }
            if cluster_hidden_by_collapsed_ancestor(index) {
                continue;
            }
            let Some(display_id) = dag.cluster_node_id(index) else {
                continue;
            };
            visible_ids.insert(display_id);
            collapsed_cluster_by_display.insert(display_id, index);
        }

        let mut edges = Vec::<(u32, u32)>::new();
        let mut edge_set = HashSet::<(u32, u32)>::new();
        for (from, to) in &dag.edges {
            let from_display = owner_to_display(owner_for_node(*from));
            let to_display = owner_to_display(owner_for_node(*to));
            let (Some(from_display), Some(to_display)) = (from_display, to_display) else {
                continue;
            };
            if from_display == to_display {
                continue;
            }
            if edge_set.insert((from_display, to_display)) {
                edges.push((from_display, to_display));
            }
        }

        let mut visible_clusters = Vec::<VisibleCluster>::new();
        let mut visible_cluster_index_by_source = HashMap::<usize, usize>::new();
        for (source_index, cluster) in dag.cluster_info.iter().enumerate() {
            if collapsed_clusters.contains(&source_index) {
                continue;
            }
            let member_nodes = cluster
                .nodes
                .iter()
                .copied()
                .filter(|node_id| {
                    matches!(owner_for_node(*node_id), VisibleOwner::Node(id) if id == *node_id)
                })
                .collect::<Vec<_>>();
            if member_nodes.is_empty() {
                continue;
            }
            let parent_visible_index = cluster.parent.and_then(|parent_source| {
                visible_cluster_index_by_source.get(&parent_source).copied()
            });
            let visible_index = visible_clusters.len();
            visible_clusters.push(VisibleCluster {
                source_index,
                member_nodes,
                parent_visible_index,
                padding: dag.clusters[source_index].padding.unwrap_or(24.0),
            });
            visible_cluster_index_by_source.insert(source_index, visible_index);
        }

        Self {
            nodes: visible_ids.into_iter().collect(),
            edges,
            visible_real_nodes,
            collapsed_cluster_by_display,
            visible_clusters,
        }
    }
}

#[derive(Default)]
struct GraphBuilder {
    nodes: Vec<NodeDisplay>,
    edges: Vec<(u32, u32)>,
    clusters: Vec<ClusterSpec>,
    cluster_info: Vec<ClusterInfo>,
    cluster_stack: Vec<usize>,
    cluster_next_id: u32,
    runtime_next_id: u32,
    display_next_id: u32,
    label_occurrences: HashMap<String, u32>,
    conditional_branches: Vec<ConditionalBranchInfo>,
    descendant_runtime_ids_by_runtime_id: HashMap<u32, Vec<u32>>,
}

#[derive(Default)]
struct Flattened {
    roots: Vec<u32>,
    exits: Vec<u32>,
    members: Vec<u32>,
    root_runtime_ids: Vec<u32>,
    member_runtime_ids: Vec<u32>,
}

impl GraphBuilder {
    fn flatten(&mut self, ast: &JourneyAst) -> Flattened {
        match ast {
            JourneyAst::Empty => Flattened::default(),
            JourneyAst::Sequence(items) => {
                let mut acc = Flattened::default();
                let mut previous_exits = Vec::<u32>::new();
                for item in items {
                    let current = self.flatten(item);
                    if current.roots.is_empty() {
                        continue;
                    }

                    if acc.roots.is_empty() {
                        acc.roots = current.roots.clone();
                    }

                    for from in &previous_exits {
                        for to in &current.roots {
                            self.edges.push((*from, *to));
                        }
                    }

                    previous_exits = current.exits.clone();
                    acc.exits = current.exits.clone();
                    acc.members.extend(current.members);
                    if acc.root_runtime_ids.is_empty() {
                        acc.root_runtime_ids = current.root_runtime_ids.clone();
                    }
                    acc.member_runtime_ids
                        .extend(current.member_runtime_ids.iter().copied());
                }
                acc.member_runtime_ids = dedup(acc.member_runtime_ids);
                acc
            }
            JourneyAst::Step { label } => {
                let runtime_id = self.runtime_next_id;
                self.runtime_next_id = self.runtime_next_id.saturating_add(1);
                let label = self.unique_label(*label);
                let node = self.push_runtime_node(label, runtime_id);
                Flattened {
                    roots: vec![node],
                    exits: vec![node],
                    members: vec![node],
                    root_runtime_ids: vec![runtime_id],
                    member_runtime_ids: vec![runtime_id],
                }
            }
            JourneyAst::Conditional {
                label,
                metadata,
                left,
                right,
            } => {
                let runtime_id = self.runtime_next_id;
                self.runtime_next_id = self.runtime_next_id.saturating_add(1);
                let branch_label = if metadata.trim().is_empty() {
                    short_type_name_str(label).to_string()
                } else {
                    format!("{} :: {}", short_type_name_str(label), metadata)
                };
                let branch = self.push_layout_node(branch_label, Some(runtime_id), |node| {
                    node.is_conditional_branch = true;
                });
                if !metadata.trim().is_empty() {
                    self.mark(branch, |node| node.metadata = Some((*metadata).to_string()));
                }
                let left_flow = self.flatten(left);
                let right_flow = self.flatten(right);

                for target in &left_flow.roots {
                    self.edges.push((branch, *target));
                }
                for target in &right_flow.roots {
                    self.edges.push((branch, *target));
                }

                let mut members = vec![branch];
                members.extend(left_flow.members.iter().copied());
                members.extend(right_flow.members.iter().copied());
                let mut member_runtime_ids = vec![runtime_id];
                member_runtime_ids.extend(left_flow.member_runtime_ids.iter().copied());
                member_runtime_ids.extend(right_flow.member_runtime_ids.iter().copied());
                let member_runtime_ids = dedup(member_runtime_ids);

                let mut exits = left_flow.exits;
                exits.extend(right_flow.exits);
                exits = dedup(exits);
                let mut descendant_runtime_ids = left_flow.member_runtime_ids.clone();
                descendant_runtime_ids.extend(right_flow.member_runtime_ids.iter().copied());
                self.descendant_runtime_ids_by_runtime_id
                    .insert(runtime_id, dedup(descendant_runtime_ids));
                self.conditional_branches.push(ConditionalBranchInfo {
                    condition_display_id: branch,
                    left_root_display_ids: dedup(left_flow.roots.clone()),
                    right_root_display_ids: dedup(right_flow.roots.clone()),
                    left_member_display_ids: dedup(left_flow.members.clone()),
                    right_member_display_ids: dedup(right_flow.members.clone()),
                });

                Flattened {
                    roots: vec![branch],
                    exits,
                    members,
                    root_runtime_ids: vec![runtime_id],
                    member_runtime_ids,
                }
            }
            JourneyAst::While {
                label,
                metadata,
                body,
            } => {
                let runtime_id = self.runtime_next_id;
                self.runtime_next_id = self.runtime_next_id.saturating_add(1);
                let parent_cluster = self.cluster_stack.last().copied();
                let cluster_index = self.clusters.len();
                let cluster_id = self.cluster_next_id;
                self.cluster_next_id = self.cluster_next_id.saturating_add(1);
                let depth = self.cluster_stack.len();
                let cluster = ClusterSpec::new(Vec::new()).padding(24.0);
                let cluster = if let Some(parent) = parent_cluster {
                    cluster.parent(parent)
                } else {
                    cluster
                };
                self.clusters.push(cluster);
                let cluster_label = if metadata.trim().is_empty() {
                    format!("while: {}", short_type_name_str(label))
                } else {
                    format!("while: {} :: {}", short_type_name_str(label), metadata)
                };
                self.cluster_info.push(ClusterInfo {
                    id: cluster_id,
                    kind: ClusterKind::While,
                    runtime_node_id: runtime_id,
                    label: cluster_label,
                    metadata: if metadata.trim().is_empty() {
                        None
                    } else {
                        Some((*metadata).to_string())
                    },
                    parent: parent_cluster,
                    nodes: Vec::new(),
                    root_nodes: Vec::new(),
                    member_runtime_ids: Vec::new(),
                    root_runtime_ids: Vec::new(),
                    depth,
                });
                self.cluster_stack.push(cluster_index);
                let body_flow = self.flatten(body);
                let _ = self.cluster_stack.pop();

                for exit in &body_flow.exits {
                    for root in &body_flow.roots {
                        self.edges.push((*exit, *root));
                    }
                }

                let cluster_nodes = dedup(body_flow.members.clone());
                if !cluster_nodes.is_empty() {
                    self.clusters[cluster_index].nodes = cluster_nodes.clone();
                    self.cluster_info[cluster_index].nodes = cluster_nodes;
                }
                self.cluster_info[cluster_index].root_nodes = dedup(body_flow.roots.clone());
                let mut member_runtime_ids = vec![runtime_id];
                member_runtime_ids.extend(body_flow.member_runtime_ids.iter().copied());
                member_runtime_ids = dedup(member_runtime_ids);
                self.cluster_info[cluster_index].member_runtime_ids = member_runtime_ids.clone();
                self.cluster_info[cluster_index].root_runtime_ids =
                    dedup(body_flow.root_runtime_ids.clone());
                self.descendant_runtime_ids_by_runtime_id
                    .insert(runtime_id, dedup(body_flow.member_runtime_ids.clone()));

                Flattened {
                    roots: body_flow.roots.clone(),
                    exits: body_flow.exits,
                    members: body_flow.members,
                    root_runtime_ids: body_flow.root_runtime_ids,
                    member_runtime_ids,
                }
            }
            JourneyAst::Transparent {
                label,
                metadata,
                body,
            } => {
                let runtime_id = self.runtime_next_id;
                self.runtime_next_id = self.runtime_next_id.saturating_add(1);
                let parent_cluster = self.cluster_stack.last().copied();
                let cluster_index = self.clusters.len();
                let cluster_id = self.cluster_next_id;
                self.cluster_next_id = self.cluster_next_id.saturating_add(1);
                let depth = self.cluster_stack.len();
                let cluster = ClusterSpec::new(Vec::new()).padding(24.0);
                let cluster = if let Some(parent) = parent_cluster {
                    cluster.parent(parent)
                } else {
                    cluster
                };
                self.clusters.push(cluster);

                let cluster_label = if metadata.trim().is_empty() {
                    format!("transparent: {}", short_type_name_str(label))
                } else {
                    format!(
                        "transparent: {} :: {}",
                        short_type_name_str(label),
                        metadata
                    )
                };
                self.cluster_info.push(ClusterInfo {
                    id: cluster_id,
                    kind: ClusterKind::Transparent,
                    runtime_node_id: runtime_id,
                    label: cluster_label,
                    metadata: if metadata.trim().is_empty() {
                        None
                    } else {
                        Some((*metadata).to_string())
                    },
                    parent: parent_cluster,
                    nodes: Vec::new(),
                    root_nodes: Vec::new(),
                    member_runtime_ids: Vec::new(),
                    root_runtime_ids: Vec::new(),
                    depth,
                });

                self.cluster_stack.push(cluster_index);
                let body_flow = self.flatten(body);
                let _ = self.cluster_stack.pop();

                let cluster_nodes = dedup(body_flow.members.clone());
                if !cluster_nodes.is_empty() {
                    self.clusters[cluster_index].nodes = cluster_nodes.clone();
                    self.cluster_info[cluster_index].nodes = cluster_nodes;
                }
                self.cluster_info[cluster_index].root_nodes = dedup(body_flow.roots.clone());
                let mut member_runtime_ids = vec![runtime_id];
                member_runtime_ids.extend(body_flow.member_runtime_ids.iter().copied());
                member_runtime_ids = dedup(member_runtime_ids);
                self.cluster_info[cluster_index].member_runtime_ids = member_runtime_ids.clone();
                self.cluster_info[cluster_index].root_runtime_ids =
                    dedup(body_flow.root_runtime_ids.clone());
                self.descendant_runtime_ids_by_runtime_id
                    .insert(runtime_id, dedup(body_flow.member_runtime_ids.clone()));

                Flattened {
                    roots: body_flow.roots.clone(),
                    exits: body_flow.exits,
                    members: body_flow.members,
                    root_runtime_ids: body_flow.root_runtime_ids,
                    member_runtime_ids,
                }
            }
            JourneyAst::Attempt {
                label,
                metadata,
                body,
            } => {
                let runtime_id = self.runtime_next_id;
                self.runtime_next_id = self.runtime_next_id.saturating_add(1);
                let parent_cluster = self.cluster_stack.last().copied();
                let cluster_index = self.clusters.len();
                let cluster_id = self.cluster_next_id;
                self.cluster_next_id = self.cluster_next_id.saturating_add(1);
                let depth = self.cluster_stack.len();
                let cluster = ClusterSpec::new(Vec::new()).padding(24.0);
                let cluster = if let Some(parent) = parent_cluster {
                    cluster.parent(parent)
                } else {
                    cluster
                };
                self.clusters.push(cluster);

                let cluster_label = if metadata.trim().is_empty() {
                    format!("attempt: {}", short_type_name_str(label))
                } else {
                    format!("attempt: {} :: {}", short_type_name_str(label), metadata)
                };
                self.cluster_info.push(ClusterInfo {
                    id: cluster_id,
                    kind: ClusterKind::Attempt,
                    runtime_node_id: runtime_id,
                    label: cluster_label,
                    metadata: if metadata.trim().is_empty() {
                        None
                    } else {
                        Some((*metadata).to_string())
                    },
                    parent: parent_cluster,
                    nodes: Vec::new(),
                    root_nodes: Vec::new(),
                    member_runtime_ids: Vec::new(),
                    root_runtime_ids: Vec::new(),
                    depth,
                });

                self.cluster_stack.push(cluster_index);
                let body_flow = self.flatten(body);
                let _ = self.cluster_stack.pop();

                let cluster_nodes = dedup(body_flow.members.clone());
                if !cluster_nodes.is_empty() {
                    self.clusters[cluster_index].nodes = cluster_nodes.clone();
                    self.cluster_info[cluster_index].nodes = cluster_nodes;
                }
                self.cluster_info[cluster_index].root_nodes = dedup(body_flow.roots.clone());
                let mut member_runtime_ids = vec![runtime_id];
                member_runtime_ids.extend(body_flow.member_runtime_ids.iter().copied());
                member_runtime_ids = dedup(member_runtime_ids);
                self.cluster_info[cluster_index].member_runtime_ids = member_runtime_ids.clone();
                self.cluster_info[cluster_index].root_runtime_ids =
                    dedup(body_flow.root_runtime_ids.clone());
                self.descendant_runtime_ids_by_runtime_id
                    .insert(runtime_id, dedup(body_flow.member_runtime_ids.clone()));

                Flattened {
                    roots: body_flow.roots.clone(),
                    exits: body_flow.exits,
                    members: body_flow.members,
                    root_runtime_ids: body_flow.root_runtime_ids,
                    member_runtime_ids,
                }
            }
            JourneyAst::Select {
                label,
                metadata,
                left,
                right,
            } => {
                let runtime_id = self.runtime_next_id;
                self.runtime_next_id = self.runtime_next_id.saturating_add(1);
                let _ = (label, metadata);
                let left_flow = self.flatten(left);
                let right_flow = self.flatten(right);
                let mut roots = left_flow.roots;
                roots.extend(right_flow.roots.iter().copied());
                roots = dedup(roots);
                let mut exits = left_flow.exits;
                exits.extend(right_flow.exits.iter().copied());
                exits = dedup(exits);
                let mut members = Vec::new();
                members.extend(left_flow.members.iter().copied());
                members.extend(right_flow.members.iter().copied());
                members = dedup(members);
                let mut root_runtime_ids = left_flow.root_runtime_ids;
                root_runtime_ids.extend(right_flow.root_runtime_ids.iter().copied());
                root_runtime_ids = dedup(root_runtime_ids);
                let mut member_runtime_ids = vec![runtime_id];
                member_runtime_ids.extend(left_flow.member_runtime_ids.iter().copied());
                member_runtime_ids.extend(right_flow.member_runtime_ids.iter().copied());
                member_runtime_ids = dedup(member_runtime_ids);
                for member_id in &exits {
                    self.mark(*member_id, |node| {
                        if !node.proxy_runtime_ids.contains(&runtime_id) {
                            node.proxy_runtime_ids.push(runtime_id);
                        }
                    });
                }

                Flattened {
                    roots,
                    exits,
                    members,
                    root_runtime_ids,
                    member_runtime_ids,
                }
            }
            JourneyAst::Join {
                label,
                metadata,
                left,
                right,
            } => {
                let runtime_id = self.runtime_next_id;
                self.runtime_next_id = self.runtime_next_id.saturating_add(1);
                let parent_cluster = self.cluster_stack.last().copied();
                let cluster_index = self.clusters.len();
                let cluster_id = self.cluster_next_id;
                self.cluster_next_id = self.cluster_next_id.saturating_add(1);
                let depth = self.cluster_stack.len();
                let cluster = ClusterSpec::new(Vec::new()).padding(24.0);
                let cluster = if let Some(parent) = parent_cluster {
                    cluster.parent(parent)
                } else {
                    cluster
                };
                self.clusters.push(cluster);

                let cluster_label = if metadata.trim().is_empty() {
                    format!("join: {}", short_type_name_str(label))
                } else {
                    format!("join: {} :: {}", short_type_name_str(label), metadata)
                };
                self.cluster_info.push(ClusterInfo {
                    id: cluster_id,
                    kind: ClusterKind::Join,
                    runtime_node_id: runtime_id,
                    label: cluster_label,
                    metadata: if metadata.trim().is_empty() {
                        None
                    } else {
                        Some((*metadata).to_string())
                    },
                    parent: parent_cluster,
                    nodes: Vec::new(),
                    root_nodes: Vec::new(),
                    member_runtime_ids: Vec::new(),
                    root_runtime_ids: Vec::new(),
                    depth,
                });

                self.cluster_stack.push(cluster_index);
                let left_flow = self.flatten(left);
                let right_flow = self.flatten(right);
                let _ = self.cluster_stack.pop();
                let mut roots = left_flow.roots;
                roots.extend(right_flow.roots.iter().copied());
                roots = dedup(roots);
                let mut exits = left_flow.exits;
                exits.extend(right_flow.exits.iter().copied());
                exits = dedup(exits);
                let mut members = Vec::new();
                members.extend(left_flow.members.iter().copied());
                members.extend(right_flow.members.iter().copied());
                members = dedup(members);
                let mut root_runtime_ids = left_flow.root_runtime_ids;
                root_runtime_ids.extend(right_flow.root_runtime_ids.iter().copied());
                root_runtime_ids = dedup(root_runtime_ids);
                let mut member_runtime_ids = vec![runtime_id];
                member_runtime_ids.extend(left_flow.member_runtime_ids.iter().copied());
                member_runtime_ids.extend(right_flow.member_runtime_ids.iter().copied());
                member_runtime_ids = dedup(member_runtime_ids);
                let cluster_nodes = dedup(members.clone());
                if !cluster_nodes.is_empty() {
                    self.clusters[cluster_index].nodes = cluster_nodes.clone();
                    self.cluster_info[cluster_index].nodes = cluster_nodes;
                }
                self.cluster_info[cluster_index].root_nodes = dedup(roots.clone());
                self.cluster_info[cluster_index].member_runtime_ids = member_runtime_ids.clone();
                self.cluster_info[cluster_index].root_runtime_ids = dedup(root_runtime_ids.clone());
                for member_id in &exits {
                    self.mark(*member_id, |node| {
                        if !node.proxy_runtime_ids.contains(&runtime_id) {
                            node.proxy_runtime_ids.push(runtime_id);
                        }
                    });
                }

                Flattened {
                    roots,
                    exits,
                    members,
                    root_runtime_ids,
                    member_runtime_ids,
                }
            }
        }
    }

    fn push_runtime_node(&mut self, label: impl Into<String>, runtime_id: u32) -> u32 {
        let node_id = self.next_display_id();
        let display = NodeDisplay {
            id: node_id,
            label: label.into(),
            metadata: None,
            runtime_node_id: Some(runtime_id),
            proxy_runtime_ids: Vec::new(),
            is_conditional_branch: false,
            is_select: false,
            is_join: false,
        };
        self.nodes.push(display);
        node_id
    }

    fn push_layout_node(
        &mut self,
        label: impl Into<String>,
        runtime_node_id: Option<u32>,
        apply: impl FnOnce(&mut NodeDisplay),
    ) -> u32 {
        let node_id = self.next_display_id();
        let mut display = NodeDisplay {
            id: node_id,
            label: label.into(),
            metadata: None,
            runtime_node_id,
            proxy_runtime_ids: Vec::new(),
            is_conditional_branch: false,
            is_select: false,
            is_join: false,
        };
        apply(&mut display);
        self.nodes.push(display);
        node_id
    }

    fn mark(&mut self, node_id: u32, apply: impl FnOnce(&mut NodeDisplay)) {
        if let Some(node) = self
            .nodes
            .iter_mut()
            .find(|candidate| candidate.id == node_id)
        {
            apply(node);
        }
    }

    fn next_display_id(&mut self) -> u32 {
        let id = self.display_next_id;
        self.display_next_id = self.display_next_id.saturating_add(1);
        id
    }

    fn unique_label(&mut self, raw: impl Into<String>) -> String {
        let full = raw.into();
        let short = short_type_name_str(&full);
        let entry = self.label_occurrences.entry(short.clone()).or_insert(0);
        let label = if *entry == 0 {
            short
        } else {
            format!("{short} #{}", *entry + 1)
        };
        *entry = entry.saturating_add(1);
        label
    }
}

fn runtime_state_for_live_data_with_activation_prefixes(
    live: &LiveDagState,
    runtime_id: u32,
    runtime_sequence_floors: &HashMap<u32, usize>,
    runtime_activation_prefixes: &HashMap<u32, Vec<u64>>,
) -> RuntimeState {
    if live
        .runtime_update_sequence
        .get(&runtime_id)
        .copied()
        .zip(runtime_sequence_floors.get(&runtime_id).copied())
        .map(|(sequence, floor)| sequence < floor)
        .unwrap_or(false)
    {
        return RuntimeState::Pending;
    }
    if runtime_activation_prefixes
        .get(&runtime_id)
        .and_then(|required_prefix| {
            live.runtime_activation_paths
                .get(&runtime_id)
                .map(|path| !path.starts_with(required_prefix))
        })
        .unwrap_or(false)
    {
        return RuntimeState::Pending;
    }
    if live.failed_runtime_ids.contains(&runtime_id) {
        RuntimeState::Failed
    } else if live.active_runtime_ids.contains(&runtime_id) {
        RuntimeState::Running
    } else if live.finished_runtime_ids.contains(&runtime_id) {
        RuntimeState::Completed
    } else {
        RuntimeState::Pending
    }
}

fn node_phase_for_display_with_runtime_floors(
    live: &LiveDagState,
    dag: &Dag,
    display_id: u32,
    runtime_id: Option<u32>,
    runtime_sequence_floors: &HashMap<u32, usize>,
    runtime_activation_prefixes: &HashMap<u32, Vec<u64>>,
) -> RuntimeState {
    match runtime_id {
        Some(id) => runtime_state_for_live_data_with_activation_prefixes(
            live,
            id,
            runtime_sequence_floors,
            runtime_activation_prefixes,
        ),
        None => dag
            .derived
            .condition_successor_runtime_ids
            .get(&display_id)
            .map(|successors| {
                infer_condition_runtime_state_with_runtime_floors(
                    live,
                    successors,
                    runtime_sequence_floors,
                    runtime_activation_prefixes,
                )
            })
            .unwrap_or(RuntimeState::Pending),
    }
}

fn infer_condition_runtime_state_with_runtime_floors(
    live: &LiveDagState,
    successor_runtime_ids: &[u32],
    runtime_sequence_floors: &HashMap<u32, usize>,
    runtime_activation_prefixes: &HashMap<u32, Vec<u64>>,
) -> RuntimeState {
    let mut newest: Option<(usize, RuntimeState)> = None;

    for runtime_id in successor_runtime_ids {
        let Some(sequence) = live.runtime_update_sequence.get(runtime_id).copied() else {
            continue;
        };
        if newest
            .map(|(best_sequence, _)| sequence > best_sequence)
            .unwrap_or(true)
        {
            newest = Some((
                sequence,
                runtime_state_for_live_data_with_activation_prefixes(
                    live,
                    *runtime_id,
                    runtime_sequence_floors,
                    runtime_activation_prefixes,
                ),
            ));
        }
    }

    newest
        .map(|(_, state)| state)
        .unwrap_or(RuntimeState::Pending)
}

fn runtime_observed_in_current_iteration(
    live: &LiveDagState,
    runtime_id: u32,
    runtime_sequence_floors: &HashMap<u32, usize>,
    runtime_activation_prefixes: &HashMap<u32, Vec<u64>>,
) -> bool {
    live.runtime_update_sequence
        .get(&runtime_id)
        .copied()
        .map(|sequence| {
            sequence
                >= runtime_sequence_floors
                    .get(&runtime_id)
                    .copied()
                    .unwrap_or_default()
                && runtime_activation_prefixes
                    .get(&runtime_id)
                    .and_then(|required_prefix| {
                        live.runtime_activation_paths
                            .get(&runtime_id)
                            .map(|path| path.starts_with(required_prefix))
                    })
                    .unwrap_or(true)
        })
        .unwrap_or(false)
}

fn node_has_current_iteration_activity(
    live: &LiveDagState,
    node: &NodeDisplay,
    runtime_sequence_floors: &HashMap<u32, usize>,
    runtime_activation_prefixes: &HashMap<u32, Vec<u64>>,
) -> bool {
    node.runtime_node_id.into_iter().any(|runtime_id| {
        runtime_observed_in_current_iteration(
            live,
            runtime_id,
            runtime_sequence_floors,
            runtime_activation_prefixes,
        )
    })
}

fn conditional_branch_membership(dag: &Dag) -> HashMap<u32, (u32, ConditionalSide)> {
    let mut membership = HashMap::new();
    for branch in &dag.derived.conditional_branches {
        for display_id in &branch.left_member_display_ids {
            membership.insert(
                *display_id,
                (branch.condition_display_id, ConditionalSide::Left),
            );
        }
        for display_id in &branch.right_member_display_ids {
            membership.insert(
                *display_id,
                (branch.condition_display_id, ConditionalSide::Right),
            );
        }
    }
    membership
}

fn active_conditional_branch_sides(
    dag: &Dag,
    live: &LiveDagState,
    runtime_sequence_floors: &HashMap<u32, usize>,
    runtime_activation_prefixes: &HashMap<u32, Vec<u64>>,
) -> HashMap<u32, ConditionalSide> {
    #[derive(Clone, Copy)]
    struct BranchSignal {
        best_priority: u8,
        observed_count: usize,
        latest_sequence: usize,
    }

    let mut active = HashMap::new();

    for branch in &dag.derived.conditional_branches {
        let branch_signal_for_side = |display_ids: &[u32]| {
            let mut latest_sequence = None::<usize>;
            let mut best_priority = 0_u8;
            let mut observed_count = 0_usize;
            for display_id in display_ids {
                let Some(node) = dag.node_map.get(display_id) else {
                    continue;
                };
                let Some(runtime_id) = node.runtime_node_id else {
                    continue;
                };
                if !runtime_observed_in_current_iteration(
                    live,
                    runtime_id,
                    runtime_sequence_floors,
                    runtime_activation_prefixes,
                ) {
                    continue;
                }
                let Some(sequence) = live.runtime_update_sequence.get(&runtime_id).copied() else {
                    continue;
                };
                let priority = match runtime_state_for_live_data_with_activation_prefixes(
                    live,
                    runtime_id,
                    runtime_sequence_floors,
                    runtime_activation_prefixes,
                ) {
                    RuntimeState::Failed => 3_u8,
                    RuntimeState::Running => 2_u8,
                    RuntimeState::Completed => 1_u8,
                    RuntimeState::Pending => 0_u8,
                };
                best_priority = best_priority.max(priority);
                observed_count = observed_count.saturating_add(1);
                latest_sequence = Some(
                    latest_sequence
                        .map(|current| current.max(sequence))
                        .unwrap_or(sequence),
                );
            }
            latest_sequence.map(|latest_sequence| BranchSignal {
                best_priority,
                observed_count,
                latest_sequence,
            })
        };

        let left_root_signal = branch_signal_for_side(&branch.left_root_display_ids);
        let right_root_signal = branch_signal_for_side(&branch.right_root_display_ids);
        let left_signal = branch_signal_for_side(&branch.left_member_display_ids);
        let right_signal = branch_signal_for_side(&branch.right_member_display_ids);

        let selected_side = match (left_root_signal, right_root_signal) {
            (Some(_), Some(_)) => Some(ConditionalSide::Left),
            (Some(_), None) => Some(ConditionalSide::Left),
            (None, Some(_)) => Some(ConditionalSide::Right),
            (None, None) => None,
        }
        .or_else(|| match (left_signal, right_signal) {
            (Some(_), None) => Some(ConditionalSide::Left),
            (None, Some(_)) => Some(ConditionalSide::Right),
            (Some(left_signal), Some(right_signal)) => {
                if left_signal.best_priority > right_signal.best_priority
                    || (left_signal.best_priority == right_signal.best_priority
                        && left_signal.observed_count > right_signal.observed_count)
                    || (left_signal.best_priority == right_signal.best_priority
                        && left_signal.observed_count == right_signal.observed_count
                        && left_signal.latest_sequence > right_signal.latest_sequence)
                {
                    Some(ConditionalSide::Left)
                } else if right_signal.best_priority > left_signal.best_priority
                    || (right_signal.best_priority == left_signal.best_priority
                        && right_signal.observed_count > left_signal.observed_count)
                    || (right_signal.best_priority == left_signal.best_priority
                        && right_signal.observed_count == left_signal.observed_count
                        && right_signal.latest_sequence > left_signal.latest_sequence)
                {
                    Some(ConditionalSide::Right)
                } else {
                    None
                }
            }
            (None, None) => None,
        });

        if let Some(side) = selected_side {
            active.insert(branch.condition_display_id, side);
        }
    }

    active
}

fn skipped_conditional_branch_nodes(
    dag: &Dag,
    active_conditional_sides: &HashMap<u32, ConditionalSide>,
) -> HashSet<u32> {
    let mut skipped = HashSet::new();

    for branch in &dag.derived.conditional_branches {
        match active_conditional_sides.get(&branch.condition_display_id) {
            Some(ConditionalSide::Left) => {
                skipped.extend(branch.right_member_display_ids.iter().copied());
            }
            Some(ConditionalSide::Right) => {
                skipped.extend(branch.left_member_display_ids.iter().copied());
            }
            None => {}
        }
    }

    skipped
}

fn runtime_activation_prefixes_for_display(
    dag: &Dag,
    live: &LiveDagState,
) -> HashMap<u32, Vec<u64>> {
    let mut prefixes = HashMap::<u32, Vec<u64>>::new();
    for (index, cluster) in dag.cluster_info.iter().enumerate() {
        if !matches!(cluster.kind, ClusterKind::While) {
            continue;
        }
        let cluster_path_len = live
            .runtime_activation_paths
            .get(&cluster.runtime_node_id)
            .map(Vec::len);
        let current_iteration = std::iter::once(cluster.runtime_node_id)
            .chain(dag.derived.cluster_entry_runtime_ids[index].iter().copied())
            .filter_map(|runtime_id| {
                Some((
                    live.runtime_update_sequence.get(&runtime_id).copied()?,
                    live.runtime_activation_paths.get(&runtime_id)?.clone(),
                ))
            })
            .max_by_key(|(sequence, _)| *sequence)
            .map(|(_, path)| {
                let prefix_len = cluster_path_len
                    .unwrap_or_else(|| path.len().saturating_sub(1))
                    .min(path.len());
                path[..prefix_len].to_vec()
            });
        let Some(current_iteration) = current_iteration else {
            continue;
        };
        for runtime_id in &cluster.member_runtime_ids {
            prefixes
                .entry(*runtime_id)
                .and_modify(|current| {
                    if current_iteration.len() > current.len() {
                        *current = current_iteration.clone();
                    }
                })
                .or_insert_with(|| current_iteration.clone());
        }
    }
    prefixes
}

fn repaired_live_states_for_display(
    dag: &Dag,
    live: &LiveDagState,
    runtime_sequence_floors: &HashMap<u32, usize>,
    runtime_activation_prefixes: &HashMap<u32, Vec<u64>>,
    conditional_branch_membership: &HashMap<u32, (u32, ConditionalSide)>,
    active_conditional_sides: &HashMap<u32, ConditionalSide>,
    skipped_conditional_branch_nodes: &HashSet<u32>,
) -> HashMap<u32, RuntimeState> {
    let mut states = HashMap::<u32, RuntimeState>::new();
    for node in &dag.nodes {
        let state = node_phase_for_display_with_runtime_floors(
            live,
            dag,
            node.id,
            node.runtime_node_id,
            runtime_sequence_floors,
            runtime_activation_prefixes,
        );
        states.insert(node.id, state);
    }
    for node_id in skipped_conditional_branch_nodes {
        states.insert(*node_id, RuntimeState::Pending);
    }

    let mut loop_back_edges = HashSet::<(u32, u32)>::new();
    for cluster in &dag.cluster_info {
        if !matches!(cluster.kind, ClusterKind::While) {
            continue;
        }
        let root_nodes = cluster.root_nodes.iter().copied().collect::<HashSet<_>>();
        let member_nodes = cluster.nodes.iter().copied().collect::<HashSet<_>>();
        for (from, to) in &dag.edges {
            if member_nodes.contains(from) && root_nodes.contains(to) {
                loop_back_edges.insert((*from, *to));
            }
        }
    }

    let mut incoming = HashMap::<u32, Vec<u32>>::new();
    for (from, to) in &dag.edges {
        if loop_back_edges.contains(&(*from, *to)) {
            continue;
        }
        incoming.entry(*to).or_default().push(*from);
    }

    let mut queue = std::collections::VecDeque::<u32>::new();
    let mut queued = HashSet::<u32>::new();
    for (node_id, state) in &states {
        if matches!(state, RuntimeState::Running | RuntimeState::Completed) {
            queue.push_back(*node_id);
            queued.insert(*node_id);
        }
    }

    while let Some(node_id) = queue.pop_front() {
        queued.remove(&node_id);
        let Some(predecessors) = incoming.get(&node_id) else {
            continue;
        };
        for predecessor in predecessors {
            if skipped_conditional_branch_nodes.contains(predecessor) {
                continue;
            }
            if let Some((condition_display_id, side)) =
                conditional_branch_membership.get(predecessor).copied()
            {
                let predecessor_has_activity = dag
                    .node_map
                    .get(predecessor)
                    .map(|node| {
                        node_has_current_iteration_activity(
                            live,
                            node,
                            runtime_sequence_floors,
                            runtime_activation_prefixes,
                        )
                    })
                    .unwrap_or(false);
                if !predecessor_has_activity
                    && active_conditional_sides.get(&condition_display_id).copied() != Some(side)
                {
                    continue;
                }
            }
            let predecessor_state = states
                .get(predecessor)
                .copied()
                .unwrap_or(RuntimeState::Pending);
            if !matches!(
                predecessor_state,
                RuntimeState::Pending | RuntimeState::Running
            ) {
                continue;
            }
            states.insert(*predecessor, RuntimeState::Completed);
            if queued.insert(*predecessor) {
                queue.push_back(*predecessor);
            }
        }
    }

    if !states
        .values()
        .any(|state| matches!(state, RuntimeState::Running | RuntimeState::Failed))
    {
        let ready_pending_nodes = dag
            .nodes
            .iter()
            .filter_map(|node| {
                if skipped_conditional_branch_nodes.contains(&node.id) {
                    return None;
                }
                if !matches!(states.get(&node.id), Some(RuntimeState::Pending)) {
                    return None;
                }
                let predecessors = incoming.get(&node.id)?;
                if predecessors.is_empty() {
                    return None;
                }
                predecessors
                    .iter()
                    .all(|predecessor| {
                        if skipped_conditional_branch_nodes.contains(predecessor) {
                            return true;
                        }
                        matches!(
                            states.get(predecessor),
                            Some(RuntimeState::Completed | RuntimeState::Failed)
                        )
                    })
                    .then_some(node.id)
            })
            .collect::<Vec<_>>();

        if ready_pending_nodes.len() == 1 {
            states.insert(ready_pending_nodes[0], RuntimeState::Running);
        }
    }

    for node_id in skipped_conditional_branch_nodes {
        states.insert(*node_id, RuntimeState::Pending);
    }

    states
}

fn runtime_sequence_floors_for_display(dag: &Dag, live: &LiveDagState) -> HashMap<u32, usize> {
    let mut floors = HashMap::<u32, usize>::new();
    for (index, cluster) in dag.cluster_info.iter().enumerate() {
        if !matches!(cluster.kind, ClusterKind::While) {
            continue;
        }

        let iteration_start_sequence = std::iter::once(cluster.runtime_node_id)
            .chain(dag.derived.cluster_entry_runtime_ids[index].iter().copied())
            .filter_map(|runtime_id| live.runtime_update_sequence.get(&runtime_id).copied())
            .max();
        let Some(iteration_start_sequence) = iteration_start_sequence else {
            continue;
        };

        for runtime_id in &dag.derived.cluster_member_runtime_ids[index] {
            floors
                .entry(*runtime_id)
                .and_modify(|current| *current = (*current).max(iteration_start_sequence))
                .or_insert(iteration_start_sequence);
        }
    }

    floors
}

fn cluster_live_from_states(
    live: &LiveDagState,
    dag: &Dag,
    cluster_index: usize,
    node_states: &HashMap<u32, RuntimeState>,
    runtime_sequence_floors: &HashMap<u32, usize>,
    runtime_activation_prefixes: &HashMap<u32, Vec<u64>>,
) -> ClusterLive {
    let cluster = &dag.cluster_info[cluster_index];
    let cluster_state = runtime_state_for_live_data_with_activation_prefixes(
        live,
        cluster.runtime_node_id,
        runtime_sequence_floors,
        runtime_activation_prefixes,
    );
    let mut has_running = matches!(cluster_state, RuntimeState::Running);
    let mut has_failed = matches!(cluster_state, RuntimeState::Failed);
    let mut has_completed = matches!(cluster_state, RuntimeState::Completed);

    for node_id in &cluster.nodes {
        let state = node_states
            .get(node_id)
            .copied()
            .unwrap_or(RuntimeState::Pending);
        match state {
            RuntimeState::Pending => {}
            RuntimeState::Running => {
                has_running = true;
            }
            RuntimeState::Completed => {
                has_completed = true;
            }
            RuntimeState::Failed => {
                let masked_by_attempt_boundary = dag
                    .derived
                    .nearest_attempt_boundary_cluster_index_by_display_id
                    .get(node_id)
                    .copied()
                    .map(|boundary_index| {
                        cluster_is_strict_ancestor_of_cluster(
                            &dag.cluster_info,
                            cluster_index,
                            boundary_index,
                        )
                    })
                    .unwrap_or(false);
                if !masked_by_attempt_boundary {
                    has_failed = true;
                }
            }
        }
    }

    ClusterLive {
        has_running,
        has_failed,
        has_completed,
    }
}

fn cluster_is_strict_ancestor_of_cluster(
    cluster_info: &[ClusterInfo],
    ancestor_index: usize,
    descendant_index: usize,
) -> bool {
    let mut current = cluster_info[descendant_index].parent;
    while let Some(parent_index) = current {
        if parent_index == ancestor_index {
            return true;
        }
        current = cluster_info[parent_index].parent;
    }
    false
}

fn compute_cluster_successor_runtime_ids(
    edges: &[(u32, u32)],
    node_map: &HashMap<u32, NodeDisplay>,
    cluster_info: &[ClusterInfo],
) -> Vec<Vec<u32>> {
    let mut outgoing_by_node = HashMap::<u32, Vec<u32>>::new();
    for (from, to) in edges {
        outgoing_by_node.entry(*from).or_default().push(*to);
    }

    let mut cluster_successors = vec![Vec::<u32>::new(); cluster_info.len()];
    for (index, cluster) in cluster_info.iter().enumerate() {
        let cluster_nodes = cluster.nodes.iter().copied().collect::<HashSet<_>>();
        let mut queue = std::collections::VecDeque::<u32>::new();
        let mut visited = HashSet::<u32>::new();

        for (from, to) in edges {
            if !cluster_nodes.contains(from) || cluster_nodes.contains(to) {
                continue;
            }
            if visited.insert(*to) {
                queue.push_back(*to);
            }
        }

        let mut seen_runtime_ids = BTreeSet::new();
        while let Some(node_id) = queue.pop_front() {
            if cluster_nodes.contains(&node_id) {
                continue;
            }
            if let Some(node) = node_map.get(&node_id) {
                if let Some(runtime_id) = node.runtime_node_id {
                    if seen_runtime_ids.insert(runtime_id) {
                        cluster_successors[index].push(runtime_id);
                    }
                }
            }
            if let Some(neighbors) = outgoing_by_node.get(&node_id) {
                for neighbor in neighbors {
                    if visited.insert(*neighbor) {
                        queue.push_back(*neighbor);
                    }
                }
            }
        }
    }

    cluster_successors
}

fn short_type_name_str(value: &str) -> String {
    value
        .split("::")
        .filter(|part| !part.is_empty())
        .last()
        .unwrap_or(value)
        .to_string()
}

fn dedup(values: Vec<u32>) -> Vec<u32> {
    let mut seen = BTreeSet::new();
    let mut output = Vec::new();
    for value in values {
        if seen.insert(value) {
            output.push(value);
        }
    }
    output
}
