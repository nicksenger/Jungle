use super::{DspCode, MockingBirdMctsTag, PulseCodeParadise, PulseCodeParadiseError};
use directories_next::BaseDirs;
use redb::{ReadableTable, TableDefinition};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

pub trait SearchTree<Tag = ()> {
    type Error;
    type Data: Serialize + DeserializeOwned;

    fn select(&self) -> impl Future<Output = Result<Self::Data, Self::Error>> + Send;

    fn submit(
        &self,
        data: Self::Data,
        score: f32,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

const ROOT_NODE_ID: u64 = 0;
const MCTS_TREES_TABLE: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("mockingbird_mcts_trees");
const MCTS_NODES_TABLE: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("mockingbird_mcts_nodes");
const MOCKINGBIRD_MCTS_TAG: &str = "mockingbird";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredMctsTree {
    next_node_id: u64,
    node_ids: Vec<u64>,
    pending_selected_node_id: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredMctsNode {
    id: u64,
    parent_id: Option<u64>,
    data: Vec<DspCode>,
    visits: u64,
    total_score: f64,
    children: Vec<u64>,
}

pub(crate) fn open_mcts_db(
    db_path: Option<PathBuf>,
) -> Result<(Arc<redb::Database>, PathBuf), PulseCodeParadiseError> {
    let db_path = resolve_mcts_db_path(db_path)?;
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|source| PulseCodeParadiseError::CreateDbDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let db = redb::Database::create(&db_path).map_err(|err| {
        PulseCodeParadiseError::Persistence(format!(
            "failed to open mcts database {}: {err}",
            db_path.display()
        ))
    })?;

    Ok((Arc::new(db), db_path))
}

impl PulseCodeParadise {
    fn select_mockingbird_branch(&self) -> Result<Vec<DspCode>, PulseCodeParadiseError> {
        let write_tx = self.db.begin_write().map_err(|err| {
            PulseCodeParadiseError::Persistence(format!("mcts select begin_write failed: {err}"))
        })?;
        let (mut tree_state, nodes) = load_mcts_tree(&write_tx, MOCKINGBIRD_MCTS_TAG)?;
        if tree_state.pending_selected_node_id.is_some() {
            return Err(PulseCodeParadiseError::MctsProtocol(
                "mockingbird tree already has a pending selected node; submit must follow select"
                    .to_owned(),
            ));
        }

        let selected_node_id = choose_expandable_node(&nodes, self.max_tree_depth)?;
        let selected_branch = branch_for_node(selected_node_id, &nodes, &self.initial_dsp_code)?;

        tree_state.pending_selected_node_id = Some(selected_node_id);
        save_tree_state(&write_tx, MOCKINGBIRD_MCTS_TAG, &tree_state)?;
        write_tx.commit().map_err(|err| {
            PulseCodeParadiseError::Persistence(format!("mcts select commit failed: {err}"))
        })?;

        Ok(selected_branch)
    }

    fn submit_mockingbird_branch(
        &self,
        data: Vec<DspCode>,
        score: f32,
    ) -> Result<(), PulseCodeParadiseError> {
        if !score.is_finite() {
            return Err(PulseCodeParadiseError::MctsProtocol(
                "mockingbird tree score must be finite".to_owned(),
            ));
        }
        if data.len() != 1 {
            return Err(PulseCodeParadiseError::MctsProtocol(format!(
                "mockingbird tree submit expects exactly one generated dsp code, got {}",
                data.len()
            )));
        }

        let write_tx = self.db.begin_write().map_err(|err| {
            PulseCodeParadiseError::Persistence(format!("mcts submit begin_write failed: {err}"))
        })?;
        let (mut tree_state, mut nodes) = load_mcts_tree(&write_tx, MOCKINGBIRD_MCTS_TAG)?;
        let selected_node_id = tree_state.pending_selected_node_id.take().ok_or_else(|| {
            PulseCodeParadiseError::MctsProtocol(
                "mockingbird tree has no pending selected node; select must precede submit"
                    .to_owned(),
            )
        })?;

        let new_node_id = tree_state.next_node_id;
        tree_state.next_node_id = tree_state.next_node_id.saturating_add(1);
        tree_state.node_ids.push(new_node_id);

        let selected_node = nodes.get_mut(&selected_node_id).ok_or_else(|| {
            PulseCodeParadiseError::Persistence(format!(
                "selected node {selected_node_id} missing for mockingbird tree"
            ))
        })?;
        selected_node.children.push(new_node_id);

        nodes.insert(
            new_node_id,
            StoredMctsNode {
                id: new_node_id,
                parent_id: Some(selected_node_id),
                data,
                visits: 0,
                total_score: 0.0,
                children: Vec::new(),
            },
        );

        backpropagate_mcts(&mut nodes, new_node_id, score as f64)?;
        save_tree_state(&write_tx, MOCKINGBIRD_MCTS_TAG, &tree_state)?;
        save_tree_nodes(&write_tx, MOCKINGBIRD_MCTS_TAG, &nodes)?;
        write_tx.commit().map_err(|err| {
            PulseCodeParadiseError::Persistence(format!("mcts submit commit failed: {err}"))
        })?;

        Ok(())
    }

    #[cfg(test)]
    fn load_tree_for_test(
        &self,
    ) -> Result<(StoredMctsTree, HashMap<u64, StoredMctsNode>), PulseCodeParadiseError> {
        let write_tx = self.db.begin_write().map_err(|err| {
            PulseCodeParadiseError::Persistence(format!("mcts test begin_write failed: {err}"))
        })?;
        let snapshot = load_mcts_tree(&write_tx, MOCKINGBIRD_MCTS_TAG)?;
        write_tx.commit().map_err(|err| {
            PulseCodeParadiseError::Persistence(format!("mcts test commit failed: {err}"))
        })?;
        Ok(snapshot)
    }
}

impl SearchTree<MockingBirdMctsTag> for PulseCodeParadise {
    type Error = String;
    type Data = Vec<DspCode>;

    fn select(&self) -> impl Future<Output = Result<Self::Data, Self::Error>> + Send {
        async move {
            self.select_mockingbird_branch()
                .map_err(|err| err.to_string())
        }
    }

    fn submit(
        &self,
        data: Self::Data,
        score: f32,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            self.submit_mockingbird_branch(data, score)
                .map_err(|err| err.to_string())
        }
    }
}

fn resolve_mcts_db_path(db_path: Option<PathBuf>) -> Result<PathBuf, PulseCodeParadiseError> {
    match db_path {
        Some(path) => Ok(path),
        None => {
            let base_dirs = BaseDirs::new().ok_or(PulseCodeParadiseError::HomeDirUnavailable)?;
            Ok(base_dirs
                .home_dir()
                .join(".jungle")
                .join("mockingbird")
                .join("mcts.redb"))
        }
    }
}

fn tree_key(tag: &str) -> Vec<u8> {
    format!("tree:{tag}").into_bytes()
}

fn node_key(tag: &str, node_id: u64) -> Vec<u8> {
    format!("node:{tag}:{node_id:020}").into_bytes()
}

fn initial_tree_state() -> StoredMctsTree {
    StoredMctsTree {
        next_node_id: ROOT_NODE_ID + 1,
        node_ids: vec![ROOT_NODE_ID],
        pending_selected_node_id: None,
    }
}

fn root_node() -> StoredMctsNode {
    StoredMctsNode {
        id: ROOT_NODE_ID,
        parent_id: None,
        data: Vec::new(),
        visits: 0,
        total_score: 0.0,
        children: Vec::new(),
    }
}

fn load_mcts_tree(
    write_tx: &redb::WriteTransaction,
    tag: &str,
) -> Result<(StoredMctsTree, HashMap<u64, StoredMctsNode>), PulseCodeParadiseError> {
    let tree_state = {
        let mut trees = write_tx.open_table(MCTS_TREES_TABLE).map_err(|err| {
            PulseCodeParadiseError::Persistence(format!("open mcts trees table failed: {err}"))
        })?;
        let key = tree_key(tag);
        let existing = trees
            .get(key.as_slice())
            .map_err(|err| {
                PulseCodeParadiseError::Persistence(format!(
                    "read tree state for {tag} failed: {err}"
                ))
            })?
            .map(|value| value.value().to_vec());

        match existing {
            Some(value) => serde_json::from_slice(&value)?,
            None => {
                let state = initial_tree_state();
                let encoded = serde_json::to_vec(&state)?;
                trees
                    .insert(key.as_slice(), encoded.as_slice())
                    .map_err(|err| {
                        PulseCodeParadiseError::Persistence(format!(
                            "initialize tree state for {tag} failed: {err}"
                        ))
                    })?;
                state
            }
        }
    };

    let mut nodes = HashMap::new();
    {
        let mut node_table = write_tx.open_table(MCTS_NODES_TABLE).map_err(|err| {
            PulseCodeParadiseError::Persistence(format!("open mcts nodes table failed: {err}"))
        })?;

        for node_id in &tree_state.node_ids {
            let key = node_key(tag, *node_id);
            let existing = node_table
                .get(key.as_slice())
                .map_err(|err| {
                    PulseCodeParadiseError::Persistence(format!(
                        "read node {node_id} for tree {tag} failed: {err}"
                    ))
                })?
                .map(|value| value.value().to_vec());

            match existing {
                Some(value) => {
                    nodes.insert(*node_id, serde_json::from_slice(&value)?);
                }
                None if *node_id == ROOT_NODE_ID => {
                    let root = root_node();
                    let encoded = serde_json::to_vec(&root)?;
                    node_table
                        .insert(key.as_slice(), encoded.as_slice())
                        .map_err(|err| {
                            PulseCodeParadiseError::Persistence(format!(
                                "initialize root node for {tag} failed: {err}"
                            ))
                        })?;
                    nodes.insert(ROOT_NODE_ID, root);
                }
                None => {
                    return Err(PulseCodeParadiseError::Persistence(format!(
                        "node {node_id} missing for tree {tag}"
                    )));
                }
            }
        }
    }

    Ok((tree_state, nodes))
}

fn save_tree_state(
    write_tx: &redb::WriteTransaction,
    tag: &str,
    tree_state: &StoredMctsTree,
) -> Result<(), PulseCodeParadiseError> {
    let mut trees = write_tx.open_table(MCTS_TREES_TABLE).map_err(|err| {
        PulseCodeParadiseError::Persistence(format!("open mcts trees table failed: {err}"))
    })?;
    let key = tree_key(tag);
    let encoded = serde_json::to_vec(tree_state)?;
    trees
        .insert(key.as_slice(), encoded.as_slice())
        .map_err(|err| {
            PulseCodeParadiseError::Persistence(format!("write tree state for {tag} failed: {err}"))
        })?;
    Ok(())
}

fn save_tree_nodes(
    write_tx: &redb::WriteTransaction,
    tag: &str,
    nodes: &HashMap<u64, StoredMctsNode>,
) -> Result<(), PulseCodeParadiseError> {
    let mut node_table = write_tx.open_table(MCTS_NODES_TABLE).map_err(|err| {
        PulseCodeParadiseError::Persistence(format!("open mcts nodes table failed: {err}"))
    })?;
    for (node_id, node) in nodes {
        let key = node_key(tag, *node_id);
        let encoded = serde_json::to_vec(node)?;
        node_table
            .insert(key.as_slice(), encoded.as_slice())
            .map_err(|err| {
                PulseCodeParadiseError::Persistence(format!(
                    "write node {node_id} for tree {tag} failed: {err}"
                ))
            })?;
    }
    Ok(())
}

fn choose_expandable_node(
    nodes: &HashMap<u64, StoredMctsNode>,
    max_tree_depth: usize,
) -> Result<u64, PulseCodeParadiseError> {
    nodes
        .values()
        .filter_map(|node| {
            let depth = node_depth(node.id, nodes).ok()?;
            (depth < max_tree_depth).then_some((node.id, depth))
        })
        .max_by(|(left_id, left_depth), (right_id, right_depth)| {
            compare_expandable_nodes(*left_id, *left_depth, *right_id, *right_depth, nodes)
        })
        .map(|(node_id, _)| node_id)
        .ok_or_else(|| {
            PulseCodeParadiseError::MctsProtocol(format!(
                "mockingbird tree has no selectable branches below max depth {max_tree_depth}"
            ))
        })
}

fn compare_expandable_nodes(
    left_id: u64,
    left_depth: usize,
    right_id: u64,
    right_depth: usize,
    nodes: &HashMap<u64, StoredMctsNode>,
) -> Ordering {
    let left = nodes.get(&left_id).expect("left node must exist");
    let right = nodes.get(&right_id).expect("right node must exist");
    let left_score = mcts_selection_score(left, nodes);
    let right_score = mcts_selection_score(right, nodes);
    left_score
        .partial_cmp(&right_score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left_depth.cmp(&right_depth))
        .then_with(|| left_id.cmp(&right_id))
}

fn mcts_selection_score(node: &StoredMctsNode, nodes: &HashMap<u64, StoredMctsNode>) -> f64 {
    if node.visits == 0 {
        return f64::INFINITY;
    }

    let average_score = node.total_score / node.visits as f64;
    let parent_visits = node
        .parent_id
        .and_then(|parent_id| nodes.get(&parent_id).map(|parent| parent.visits))
        .unwrap_or(node.visits);

    if node.parent_id.is_none() || parent_visits <= 1 {
        return average_score;
    }

    average_score + ((2.0 * (parent_visits as f64).ln()) / node.visits as f64).sqrt()
}

fn node_depth(
    node_id: u64,
    nodes: &HashMap<u64, StoredMctsNode>,
) -> Result<usize, PulseCodeParadiseError> {
    let mut depth = 0usize;
    let mut current_node_id = Some(node_id);
    while let Some(id) = current_node_id {
        let node = nodes.get(&id).ok_or_else(|| {
            PulseCodeParadiseError::Persistence(format!(
                "node {id} missing while computing mockingbird tree depth"
            ))
        })?;
        current_node_id = node.parent_id;
        if current_node_id.is_some() {
            depth = depth.saturating_add(1);
        }
    }
    Ok(depth)
}

fn branch_for_node(
    node_id: u64,
    nodes: &HashMap<u64, StoredMctsNode>,
    initial_dsp_code: &DspCode,
) -> Result<Vec<DspCode>, PulseCodeParadiseError> {
    let mut lineage = Vec::new();
    let mut current_node_id = Some(node_id);
    while let Some(id) = current_node_id {
        let node = nodes.get(&id).ok_or_else(|| {
            PulseCodeParadiseError::Persistence(format!(
                "node {id} missing while rebuilding mockingbird branch"
            ))
        })?;
        if id != ROOT_NODE_ID {
            lineage.push(id);
        }
        current_node_id = node.parent_id;
    }

    let mut branch = vec![initial_dsp_code.clone()];
    for id in lineage.into_iter().rev() {
        branch.extend(
            nodes
                .get(&id)
                .ok_or_else(|| {
                    PulseCodeParadiseError::Persistence(format!(
                        "node {id} missing while extending mockingbird branch"
                    ))
                })?
                .data
                .iter()
                .cloned(),
        );
    }
    Ok(branch)
}

fn backpropagate_mcts(
    nodes: &mut HashMap<u64, StoredMctsNode>,
    start_node_id: u64,
    score: f64,
) -> Result<(), PulseCodeParadiseError> {
    let mut current_node_id = Some(start_node_id);
    while let Some(node_id) = current_node_id {
        let node = nodes.get_mut(&node_id).ok_or_else(|| {
            PulseCodeParadiseError::Persistence(format!(
                "node {node_id} missing while backpropagating mockingbird tree"
            ))
        })?;
        node.visits = node.visits.saturating_add(1);
        node.total_score += score;
        current_node_id = node.parent_id;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use reqwest::Url;
    use uuid::Uuid;

    fn temp_db_path(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join("jungle-mockingbird-tests")
            .join(format!("{name}-{}.redb", Uuid::new_v4()))
    }

    fn dsp_code(iteration_id: &str, similarity: Option<f32>) -> DspCode {
        DspCode {
            iteration_id: iteration_id.to_owned(),
            source: format!("// {iteration_id}"),
            sample_path: format!("/tmp/{iteration_id}.wav"),
            spectrogram_path: format!("/tmp/{iteration_id}.png"),
            similarity,
        }
    }

    fn ecosystem(name: &str, max_tree_depth: usize) -> PulseCodeParadise {
        PulseCodeParadise::new(
            Url::parse("https://api.openai.com/v1").unwrap(),
            None,
            Some(temp_db_path(name)),
        )
        .unwrap()
        .with_mcts_config(dsp_code("initial", Some(0.1)), max_tree_depth)
    }

    #[test]
    fn resolves_default_db_path_under_home_directory() {
        let db_path = resolve_mcts_db_path(None).unwrap();
        assert!(db_path.ends_with(".jungle/mockingbird/mcts.redb"));
    }

    #[test]
    fn persists_mockingbird_branch_and_backpropagates() {
        let db_path = temp_db_path("persisted-mcts");
        let tokens_url = Url::parse("https://api.openai.com/v1").unwrap();
        let initial = dsp_code("initial", Some(0.2));

        let first = PulseCodeParadise::new(tokens_url.clone(), None, Some(db_path.clone()))
            .unwrap()
            .with_mcts_config(initial.clone(), 8);
        let selected =
            block_on(<PulseCodeParadise as SearchTree<MockingBirdMctsTag>>::select(&first))
                .unwrap();
        assert_eq!(selected, vec![initial.clone()]);
        drop(first);

        let second = PulseCodeParadise::new(tokens_url.clone(), None, Some(db_path.clone()))
            .unwrap()
            .with_mcts_config(initial.clone(), 8);
        let candidate = dsp_code("00000001", Some(0.75));
        block_on(
            <PulseCodeParadise as SearchTree<MockingBirdMctsTag>>::submit(
                &second,
                vec![candidate.clone()],
                0.75,
            ),
        )
        .unwrap();
        drop(second);

        let third = PulseCodeParadise::new(tokens_url, None, Some(db_path))
            .unwrap()
            .with_mcts_config(initial, 8);
        let (tree, nodes) = third.load_tree_for_test().unwrap();
        let root = nodes.get(&ROOT_NODE_ID).unwrap();
        let child = nodes.get(&1).unwrap();

        assert_eq!(tree.pending_selected_node_id, None);
        assert_eq!(root.children, vec![1]);
        assert_eq!(root.visits, 1);
        assert!((root.total_score - 0.75).abs() < f64::EPSILON);
        assert_eq!(child.parent_id, Some(ROOT_NODE_ID));
        assert_eq!(child.visits, 1);
        assert_eq!(child.data, vec![candidate]);
    }

    #[test]
    fn rebuilds_branch_with_initial_code_prepended() {
        let ecosystem = ecosystem("branch-select", 8);

        let root_branch =
            block_on(<PulseCodeParadise as SearchTree<MockingBirdMctsTag>>::select(&ecosystem))
                .unwrap();
        let child = dsp_code("00000001", Some(0.6));
        block_on(
            <PulseCodeParadise as SearchTree<MockingBirdMctsTag>>::submit(
                &ecosystem,
                vec![child.clone()],
                0.6,
            ),
        )
        .unwrap();

        let selected =
            block_on(<PulseCodeParadise as SearchTree<MockingBirdMctsTag>>::select(&ecosystem))
                .unwrap();

        assert_eq!(root_branch.len(), 1);
        assert!(selected.len() >= 2);
        assert_eq!(selected.first().unwrap().iteration_id, "initial");
        assert_eq!(selected.last().unwrap(), &child);
    }

    #[test]
    fn requires_select_and_submit_to_alternate() {
        let ecosystem = ecosystem("alternation", 8);

        let submit_err = block_on(
            <PulseCodeParadise as SearchTree<MockingBirdMctsTag>>::submit(
                &ecosystem,
                vec![dsp_code("00000001", Some(0.3))],
                0.3,
            ),
        )
        .unwrap_err();
        assert!(submit_err
            .to_string()
            .contains("select must precede submit"));

        let _ = block_on(<PulseCodeParadise as SearchTree<MockingBirdMctsTag>>::select(&ecosystem))
            .unwrap();
        let select_err =
            block_on(<PulseCodeParadise as SearchTree<MockingBirdMctsTag>>::select(&ecosystem))
                .unwrap_err();
        assert!(select_err.to_string().contains("submit must follow select"));
    }

    #[test]
    fn excludes_terminal_depth_from_selection() {
        let ecosystem = ecosystem("max-depth", 2);

        let _ = block_on(<PulseCodeParadise as SearchTree<MockingBirdMctsTag>>::select(&ecosystem))
            .unwrap();
        block_on(
            <PulseCodeParadise as SearchTree<MockingBirdMctsTag>>::submit(
                &ecosystem,
                vec![dsp_code("00000001", Some(0.4))],
                0.4,
            ),
        )
        .unwrap();

        let branch =
            block_on(<PulseCodeParadise as SearchTree<MockingBirdMctsTag>>::select(&ecosystem))
                .unwrap();
        block_on(
            <PulseCodeParadise as SearchTree<MockingBirdMctsTag>>::submit(
                &ecosystem,
                vec![dsp_code("00000002", Some(0.8))],
                0.8,
            ),
        )
        .unwrap();

        let next_selected =
            block_on(<PulseCodeParadise as SearchTree<MockingBirdMctsTag>>::select(&ecosystem))
                .unwrap();

        assert!(branch.len() <= 2);
        assert!(next_selected.len() <= 2);
    }
}
