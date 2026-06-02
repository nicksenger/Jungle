use super::{PulseCodeParadise, PulseCodeParadiseError};
use directories_next::BaseDirs;
use redb::{ReadableTable, TableDefinition};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::any::type_name;
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
    data: Value,
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
    fn select_mcts(&self, tag: &str) -> Result<Value, PulseCodeParadiseError> {
        let write_tx = self.db.begin_write().map_err(|err| {
            PulseCodeParadiseError::Persistence(format!("mcts select begin_write failed: {err}"))
        })?;
        let (mut tree_state, nodes) = load_mcts_tree(&write_tx, tag)?;
        if tree_state.pending_selected_node_id.is_some() {
            return Err(PulseCodeParadiseError::MctsProtocol(format!(
                "tree {tag} already has a pending selected node; submit must follow select"
            )));
        }

        let selected_node_id = choose_mcts_node(&nodes)?;
        let selected_data = nodes
            .get(&selected_node_id)
            .ok_or_else(|| {
                PulseCodeParadiseError::Persistence(format!(
                    "selected node {selected_node_id} missing for tree {tag}"
                ))
            })?
            .data
            .clone();

        tree_state.pending_selected_node_id = Some(selected_node_id);
        save_tree_state(&write_tx, tag, &tree_state)?;
        write_tx.commit().map_err(|err| {
            PulseCodeParadiseError::Persistence(format!("mcts select commit failed: {err}"))
        })?;

        Ok(selected_data)
    }

    fn select_mcts_for_type<Tag>(&self) -> Result<Value, PulseCodeParadiseError> {
        self.select_mcts(type_name::<Tag>())
    }

    fn submit_mcts(
        &self,
        tag: &str,
        data: Value,
        score: f32,
    ) -> Result<(), PulseCodeParadiseError> {
        let write_tx = self.db.begin_write().map_err(|err| {
            PulseCodeParadiseError::Persistence(format!("mcts submit begin_write failed: {err}"))
        })?;
        let (mut tree_state, mut nodes) = load_mcts_tree(&write_tx, tag)?;
        let selected_node_id = tree_state.pending_selected_node_id.take().ok_or_else(|| {
            PulseCodeParadiseError::MctsProtocol(format!(
                "tree {tag} has no pending selected node; select must precede submit"
            ))
        })?;

        let new_node_id = tree_state.next_node_id;
        tree_state.next_node_id = tree_state.next_node_id.saturating_add(1);
        tree_state.node_ids.push(new_node_id);

        let selected_node = nodes.get_mut(&selected_node_id).ok_or_else(|| {
            PulseCodeParadiseError::Persistence(format!(
                "selected node {selected_node_id} missing for tree {tag}"
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

        backpropagate_mcts(&mut nodes, new_node_id, score as f64, tag)?;
        save_tree_state(&write_tx, tag, &tree_state)?;
        save_tree_nodes(&write_tx, tag, &nodes)?;
        write_tx.commit().map_err(|err| {
            PulseCodeParadiseError::Persistence(format!("mcts submit commit failed: {err}"))
        })?;

        Ok(())
    }

    fn submit_mcts_for_type<Tag>(
        &self,
        data: Value,
        score: f32,
    ) -> Result<(), PulseCodeParadiseError> {
        self.submit_mcts(type_name::<Tag>(), data, score)
    }

    #[cfg(test)]
    fn load_tree_for_test(
        &self,
        tag: &str,
    ) -> Result<(StoredMctsTree, HashMap<u64, StoredMctsNode>), PulseCodeParadiseError> {
        let write_tx = self.db.begin_write().map_err(|err| {
            PulseCodeParadiseError::Persistence(format!("mcts test begin_write failed: {err}"))
        })?;
        let snapshot = load_mcts_tree(&write_tx, tag)?;
        write_tx.commit().map_err(|err| {
            PulseCodeParadiseError::Persistence(format!("mcts test commit failed: {err}"))
        })?;
        Ok(snapshot)
    }
}

impl<Tag> SearchTree<Tag> for PulseCodeParadise {
    type Error = PulseCodeParadiseError;
    type Data = Value;

    fn select(&self) -> impl Future<Output = Result<Self::Data, Self::Error>> + Send {
        async move { self.select_mcts_for_type::<Tag>() }
    }

    fn submit(
        &self,
        data: Self::Data,
        score: f32,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move { self.submit_mcts_for_type::<Tag>(data, score) }
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
        data: Value::Null,
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

fn choose_mcts_node(nodes: &HashMap<u64, StoredMctsNode>) -> Result<u64, PulseCodeParadiseError> {
    nodes
        .values()
        .max_by(|left, right| compare_mcts_nodes(left, right, nodes))
        .map(|node| node.id)
        .ok_or_else(|| PulseCodeParadiseError::Persistence("mcts tree has no nodes".to_owned()))
}

fn compare_mcts_nodes(
    left: &StoredMctsNode,
    right: &StoredMctsNode,
    nodes: &HashMap<u64, StoredMctsNode>,
) -> Ordering {
    let left_score = mcts_selection_score(left, nodes);
    let right_score = mcts_selection_score(right, nodes);
    left_score
        .partial_cmp(&right_score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| right.id.cmp(&left.id))
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

    if parent_visits <= 1 {
        return average_score;
    }

    average_score + ((2.0 * (parent_visits as f64).ln()) / node.visits as f64).sqrt()
}

fn backpropagate_mcts(
    nodes: &mut HashMap<u64, StoredMctsNode>,
    start_node_id: u64,
    score: f64,
    tag: &str,
) -> Result<(), PulseCodeParadiseError> {
    let mut current_node_id = Some(start_node_id);
    while let Some(node_id) = current_node_id {
        let node = nodes.get_mut(&node_id).ok_or_else(|| {
            PulseCodeParadiseError::Persistence(format!(
                "node {node_id} missing while backpropagating tree {tag}"
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
    use serde_json::json;
    use uuid::Uuid;

    struct BasslineTag;
    struct DrumsTag;

    fn temp_db_path(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join("jungle-mockingbird-tests")
            .join(format!("{name}-{}.redb", Uuid::new_v4()))
    }

    #[test]
    fn resolves_default_db_path_under_home_directory() {
        let db_path = resolve_mcts_db_path(None).unwrap();
        assert!(db_path.ends_with(".jungle/mockingbird/mcts.redb"));
    }

    #[test]
    fn persists_mcts_selection_between_processes_and_backprops() {
        let db_path = temp_db_path("persisted-mcts");
        let tokens_url = Url::parse("https://api.openai.com/v1").unwrap();

        let first =
            PulseCodeParadise::new(tokens_url.clone(), None, Some(db_path.clone())).unwrap();
        let selected = block_on(<PulseCodeParadise as SearchTree<BasslineTag>>::select(
            &first,
        ))
        .unwrap();
        assert_eq!(selected, Value::Null);
        drop(first);

        let second =
            PulseCodeParadise::new(tokens_url.clone(), None, Some(db_path.clone())).unwrap();
        block_on(<PulseCodeParadise as SearchTree<BasslineTag>>::submit(
            &second,
            json!({ "prompt": "add syncopation" }),
            0.75,
        ))
        .unwrap();
        drop(second);

        let third = PulseCodeParadise::new(tokens_url, None, Some(db_path.clone())).unwrap();
        let (tree, nodes) = third
            .load_tree_for_test(type_name::<BasslineTag>())
            .unwrap();
        let root = nodes.get(&ROOT_NODE_ID).unwrap();
        let child = nodes.get(&1).unwrap();

        assert_eq!(tree.pending_selected_node_id, None);
        assert_eq!(root.children, vec![1]);
        assert_eq!(root.visits, 1);
        assert!((root.total_score - 0.75).abs() < f64::EPSILON);
        assert_eq!(child.parent_id, Some(ROOT_NODE_ID));
        assert_eq!(child.visits, 1);
        assert_eq!(child.data, json!({ "prompt": "add syncopation" }));
    }

    #[test]
    fn requires_select_and_submit_to_alternate() {
        let db_path = temp_db_path("alternation");
        let ecosystem = PulseCodeParadise::new(
            Url::parse("https://api.openai.com/v1").unwrap(),
            None,
            Some(db_path),
        )
        .unwrap();

        let submit_err = block_on(<PulseCodeParadise as SearchTree<DrumsTag>>::submit(
            &ecosystem,
            json!({ "prompt": "fill" }),
            0.3,
        ))
        .unwrap_err();
        assert!(submit_err
            .to_string()
            .contains("select must precede submit"));

        let _ = block_on(<PulseCodeParadise as SearchTree<DrumsTag>>::select(
            &ecosystem,
        ))
        .unwrap();
        let select_err = block_on(<PulseCodeParadise as SearchTree<DrumsTag>>::select(
            &ecosystem,
        ))
        .unwrap_err();
        assert!(select_err.to_string().contains("submit must follow select"));
    }
}
