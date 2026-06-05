use super::{
    DspCode, LyrebirdBranchNode, LyrebirdInstrument, LyrebirdInstrumentTag, PulseCodePurgatory,
    PulseCodePurgatoryError,
};
use directories_next::BaseDirs;
use redb::{ReadableTable, TableDefinition};
use serde::{Deserialize, Serialize};
use std::any::type_name;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs;
use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::warn;

const ROOT_NODE_ID: u64 = 0;
const MCTS_SCHEMA_VERSION: &str = "v2-score-metrics";
const MCTS_TREES_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("lyrebird_mcts_trees");
const MCTS_NODES_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("lyrebird_mcts_nodes");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Submission<Data> {
    pub data: Data,
    pub score: f32,
}

pub trait SearchTree<Tag = ()> {
    type Error;
    type Data: Clone + Serialize + for<'de> Deserialize<'de>;

    fn select(&self) -> impl Future<Output = Result<Self::Data, Self::Error>> + Send;

    fn submit(
        &self,
        submissions: Vec<Submission<Self::Data>>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredMctsTree {
    next_node_id: u64,
    node_ids: Vec<u64>,
    pending_selected_node_id: Option<u64>,
    #[serde(default)]
    pending_session_id: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct StoredMctsNode {
    id: u64,
    parent_id: Option<u64>,
    data: Vec<LyrebirdBranchNode>,
    visits: u64,
    total_score: f64,
    children: Vec<u64>,
}

pub(crate) fn open_mcts_db(
    db_path: Option<PathBuf>,
) -> Result<(Arc<redb::Database>, PathBuf), PulseCodePurgatoryError> {
    let db_path = resolve_mcts_db_path(db_path)?;
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|source| PulseCodePurgatoryError::CreateDbDir {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    let db = redb::Database::create(&db_path).map_err(|err| {
        PulseCodePurgatoryError::Persistence(format!(
            "failed to open mcts database {}: {err}",
            db_path.display()
        ))
    })?;

    Ok((Arc::new(db), db_path))
}

impl PulseCodePurgatory {
    pub(crate) fn select_lyrebird_branch_for_tag<Tag>(
        &self,
    ) -> Result<Vec<LyrebirdBranchNode>, PulseCodePurgatoryError>
    where
        Tag: LyrebirdInstrumentTag,
    {
        let instrument = Tag::INSTRUMENT;
        let write_tx = self.db.begin_write().map_err(|err| {
            PulseCodePurgatoryError::Persistence(format!("mcts select begin_write failed: {err}"))
        })?;
        let tag = type_name::<Tag>();
        let (mut tree_state, nodes) = load_mcts_tree(&write_tx, tag)?;
        if let Some(pending_selected_node_id) = tree_state.pending_selected_node_id {
            if tree_state.pending_session_id.as_deref() == Some(self.runtime_session_id.as_str()) {
                return Err(PulseCodePurgatoryError::MctsProtocol(format!(
                    "{} tree already has a pending selected node; submit must follow select",
                    instrument.slug()
                )));
            }

            warn!(
                instrument = instrument.slug(),
                pending_selected_node_id,
                pending_session_id = tree_state.pending_session_id.as_deref().unwrap_or("unknown"),
                runtime_session_id = %self.runtime_session_id,
                "recovering stale lyrebird mcts selection from a previous runtime"
            );
            tree_state.pending_selected_node_id = None;
            tree_state.pending_session_id = None;
        }

        let selected_node_id = choose_expandable_node(&nodes, self.max_tree_depth, instrument)?;
        let selected_branch = branch_for_node(
            selected_node_id,
            &nodes,
            self.initial_dsp_codes.get(&instrument).ok_or_else(|| {
                PulseCodePurgatoryError::MctsProtocol(format!(
                    "missing initial dsp code for {}",
                    instrument.slug()
                ))
            })?,
            instrument,
        )?;

        tree_state.pending_selected_node_id = Some(selected_node_id);
        tree_state.pending_session_id = Some(self.runtime_session_id.clone());
        save_tree_state(&write_tx, tag, &tree_state)?;
        write_tx.commit().map_err(|err| {
            PulseCodePurgatoryError::Persistence(format!("mcts select commit failed: {err}"))
        })?;

        Ok(selected_branch)
    }

    pub(crate) fn submit_lyrebird_branch_for_tag<Tag>(
        &self,
        submissions: Vec<Submission<Vec<LyrebirdBranchNode>>>,
    ) -> Result<(), PulseCodePurgatoryError>
    where
        Tag: LyrebirdInstrumentTag,
    {
        let instrument = Tag::INSTRUMENT;
        for submission in &submissions {
            if !submission.score.is_finite() {
                return Err(PulseCodePurgatoryError::MctsProtocol(format!(
                    "{} tree score must be finite",
                    instrument.slug()
                )));
            }
            if submission.data.len() != 1 {
                return Err(PulseCodePurgatoryError::MctsProtocol(format!(
                    "{} tree submit expects exactly one generated branch node per submission, got {}",
                    instrument.slug(),
                    submission.data.len()
                )));
            }
        }

        let write_tx = self.db.begin_write().map_err(|err| {
            PulseCodePurgatoryError::Persistence(format!("mcts submit begin_write failed: {err}"))
        })?;
        let tag = type_name::<Tag>();
        let (mut tree_state, mut nodes) = load_mcts_tree(&write_tx, tag)?;
        if tree_state.pending_session_id.as_deref() != Some(self.runtime_session_id.as_str()) {
            return Err(PulseCodePurgatoryError::MctsProtocol(format!(
                "{} tree pending selection does not belong to this runtime; select must precede submit",
                instrument.slug()
            )));
        }
        let selected_node_id = tree_state.pending_selected_node_id.take().ok_or_else(|| {
            PulseCodePurgatoryError::MctsProtocol(format!(
                "{} tree has no pending selected node; select must precede submit",
                instrument.slug()
            ))
        })?;
        tree_state.pending_session_id = None;

        let new_nodes = {
            let selected_node = nodes.get_mut(&selected_node_id).ok_or_else(|| {
                PulseCodePurgatoryError::Persistence(format!(
                    "selected node {selected_node_id} missing for {} tree",
                    instrument.slug()
                ))
            })?;
            let mut new_nodes = Vec::with_capacity(submissions.len());
            for submission in submissions {
                let new_node_id = tree_state.next_node_id;
                tree_state.next_node_id = tree_state.next_node_id.saturating_add(1);
                tree_state.node_ids.push(new_node_id);
                selected_node.children.push(new_node_id);
                new_nodes.push((new_node_id, submission));
            }
            new_nodes
        };

        for (new_node_id, submission) in new_nodes {
            nodes.insert(
                new_node_id,
                StoredMctsNode {
                    id: new_node_id,
                    parent_id: Some(selected_node_id),
                    data: submission.data,
                    visits: 0,
                    total_score: 0.0,
                    children: Vec::new(),
                },
            );

            backpropagate_mcts(&mut nodes, new_node_id, submission.score as f64, instrument)?;
        }

        save_tree_state(&write_tx, tag, &tree_state)?;
        save_tree_nodes(&write_tx, tag, &nodes)?;
        write_tx.commit().map_err(|err| {
            PulseCodePurgatoryError::Persistence(format!("mcts submit commit failed: {err}"))
        })?;

        Ok(())
    }

    #[cfg(test)]
    fn load_tree_for_test<Tag>(
        &self,
    ) -> Result<(StoredMctsTree, HashMap<u64, StoredMctsNode>), PulseCodePurgatoryError>
    where
        Tag: LyrebirdInstrumentTag,
    {
        let write_tx = self.db.begin_write().map_err(|err| {
            PulseCodePurgatoryError::Persistence(format!("mcts test begin_write failed: {err}"))
        })?;
        let snapshot = load_mcts_tree(&write_tx, type_name::<Tag>())?;
        write_tx.commit().map_err(|err| {
            PulseCodePurgatoryError::Persistence(format!("mcts test commit failed: {err}"))
        })?;
        Ok(snapshot)
    }
}

impl<Tag> SearchTree<Tag> for PulseCodePurgatory
where
    Tag: LyrebirdInstrumentTag + Send + Sync + 'static,
{
    type Error = String;
    type Data = Vec<LyrebirdBranchNode>;

    fn select(&self) -> impl Future<Output = Result<Self::Data, Self::Error>> + Send {
        async move {
            self.select_lyrebird_branch_for_tag::<Tag>()
                .map_err(|err| err.to_string())
        }
    }

    fn submit(
        &self,
        submissions: Vec<Submission<Self::Data>>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        async move {
            self.submit_lyrebird_branch_for_tag::<Tag>(submissions)
                .map_err(|err| err.to_string())
        }
    }
}

fn resolve_mcts_db_path(db_path: Option<PathBuf>) -> Result<PathBuf, PulseCodePurgatoryError> {
    match db_path {
        Some(path) => Ok(path),
        None => {
            let base_dirs = BaseDirs::new().ok_or(PulseCodePurgatoryError::HomeDirUnavailable)?;
            Ok(base_dirs
                .home_dir()
                .join(".jungle")
                .join("lyrebird")
                .join("mcts.redb"))
        }
    }
}

fn tree_key(tag: &str) -> Vec<u8> {
    format!("tree:{MCTS_SCHEMA_VERSION}:{tag}").into_bytes()
}

fn node_key(tag: &str, node_id: u64) -> Vec<u8> {
    format!("node:{MCTS_SCHEMA_VERSION}:{tag}:{node_id:020}").into_bytes()
}

fn initial_tree_state() -> StoredMctsTree {
    StoredMctsTree {
        next_node_id: ROOT_NODE_ID + 1,
        node_ids: vec![ROOT_NODE_ID],
        pending_selected_node_id: None,
        pending_session_id: None,
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
) -> Result<(StoredMctsTree, HashMap<u64, StoredMctsNode>), PulseCodePurgatoryError> {
    let tree_state = {
        let mut trees = write_tx.open_table(MCTS_TREES_TABLE).map_err(|err| {
            PulseCodePurgatoryError::Persistence(format!("open mcts trees table failed: {err}"))
        })?;
        let key = tree_key(tag);
        let existing = trees
            .get(key.as_slice())
            .map_err(|err| {
                PulseCodePurgatoryError::Persistence(format!(
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
                        PulseCodePurgatoryError::Persistence(format!(
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
            PulseCodePurgatoryError::Persistence(format!("open mcts nodes table failed: {err}"))
        })?;

        for node_id in &tree_state.node_ids {
            let key = node_key(tag, *node_id);
            let existing = node_table
                .get(key.as_slice())
                .map_err(|err| {
                    PulseCodePurgatoryError::Persistence(format!(
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
                            PulseCodePurgatoryError::Persistence(format!(
                                "initialize root node for {tag} failed: {err}"
                            ))
                        })?;
                    nodes.insert(ROOT_NODE_ID, root);
                }
                None => {
                    return Err(PulseCodePurgatoryError::Persistence(format!(
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
) -> Result<(), PulseCodePurgatoryError> {
    let mut trees = write_tx.open_table(MCTS_TREES_TABLE).map_err(|err| {
        PulseCodePurgatoryError::Persistence(format!("open mcts trees table failed: {err}"))
    })?;
    let key = tree_key(tag);
    let encoded = serde_json::to_vec(tree_state)?;
    trees
        .insert(key.as_slice(), encoded.as_slice())
        .map_err(|err| {
            PulseCodePurgatoryError::Persistence(format!(
                "write tree state for {tag} failed: {err}"
            ))
        })?;
    Ok(())
}

fn save_tree_nodes(
    write_tx: &redb::WriteTransaction,
    tag: &str,
    nodes: &HashMap<u64, StoredMctsNode>,
) -> Result<(), PulseCodePurgatoryError> {
    let mut node_table = write_tx.open_table(MCTS_NODES_TABLE).map_err(|err| {
        PulseCodePurgatoryError::Persistence(format!("open mcts nodes table failed: {err}"))
    })?;
    for (node_id, node) in nodes {
        let key = node_key(tag, *node_id);
        let encoded = serde_json::to_vec(node)?;
        node_table
            .insert(key.as_slice(), encoded.as_slice())
            .map_err(|err| {
                PulseCodePurgatoryError::Persistence(format!(
                    "write node {node_id} for tree {tag} failed: {err}"
                ))
            })?;
    }
    Ok(())
}

fn choose_expandable_node(
    nodes: &HashMap<u64, StoredMctsNode>,
    max_tree_depth: usize,
    instrument: LyrebirdInstrument,
) -> Result<u64, PulseCodePurgatoryError> {
    let mut best: Option<(&StoredMctsNode, usize)> = None;

    for node in nodes.values() {
        let depth = node_depth(node.id, nodes, instrument)?;
        if depth >= max_tree_depth {
            continue;
        }

        match best {
            Some((best_node, best_depth))
                if compare_expandable_nodes(node, depth, best_node, best_depth, nodes)
                    != Ordering::Greater => {}
            _ => best = Some((node, depth)),
        }
    }

    best.map(|(node, _)| node.id).ok_or_else(|| {
        PulseCodePurgatoryError::MctsProtocol(format!(
            "{} tree has no selectable branches below max depth {max_tree_depth}",
            instrument.slug()
        ))
    })
}

fn compare_expandable_nodes(
    left: &StoredMctsNode,
    left_depth: usize,
    right: &StoredMctsNode,
    right_depth: usize,
    nodes: &HashMap<u64, StoredMctsNode>,
) -> Ordering {
    let left_score = mcts_selection_score(left, nodes);
    let right_score = mcts_selection_score(right, nodes);
    left_score
        .partial_cmp(&right_score)
        .unwrap_or(Ordering::Equal)
        .then_with(|| left_depth.cmp(&right_depth))
        .then_with(|| left.id.cmp(&right.id))
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
    instrument: LyrebirdInstrument,
) -> Result<usize, PulseCodePurgatoryError> {
    let mut depth = 0usize;
    let mut current_node_id = Some(node_id);
    while let Some(id) = current_node_id {
        let node = nodes.get(&id).ok_or_else(|| {
            PulseCodePurgatoryError::Persistence(format!(
                "node {id} missing while computing {} tree depth",
                instrument.slug()
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
    instrument: LyrebirdInstrument,
) -> Result<Vec<LyrebirdBranchNode>, PulseCodePurgatoryError> {
    let mut lineage = Vec::new();
    let mut current_node_id = Some(node_id);
    while let Some(id) = current_node_id {
        let node = nodes.get(&id).ok_or_else(|| {
            PulseCodePurgatoryError::Persistence(format!(
                "node {id} missing while rebuilding {} branch",
                instrument.slug()
            ))
        })?;
        if id != ROOT_NODE_ID {
            lineage.push(id);
        }
        current_node_id = node.parent_id;
    }

    let mut branch = vec![initial_dsp_code.clone().into()];
    for id in lineage.into_iter().rev() {
        branch.extend(
            nodes
                .get(&id)
                .ok_or_else(|| {
                    PulseCodePurgatoryError::Persistence(format!(
                        "node {id} missing while extending {} branch",
                        instrument.slug()
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
    instrument: LyrebirdInstrument,
) -> Result<(), PulseCodePurgatoryError> {
    let mut current_node_id = Some(start_node_id);
    while let Some(node_id) = current_node_id {
        let node = nodes.get_mut(&node_id).ok_or_else(|| {
            PulseCodePurgatoryError::Persistence(format!(
                "node {node_id} missing while backpropagating {} tree",
                instrument.slug()
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
    use crate::{
        BackupVocalsMarker, BassMarker, DrumsMarker, GuitarSoloMarker, LyrebirdPatch,
        RhythmGuitarMarker, VocalsMarker,
    };
    use reqwest::Url;
    use uuid::Uuid;

    fn temp_db_path(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join("jungle-lyrebird-tests")
            .join(format!("{name}-{}.redb", Uuid::new_v4()))
    }

    fn dsp_code(iteration_id: &str, similarity: Option<f32>) -> DspCode {
        DspCode {
            iteration_id: iteration_id.to_owned(),
            source: format!("// {iteration_id}"),
            sample_path: format!("/tmp/{iteration_id}.wav").into(),
            spectrogram_path: format!("/tmp/{iteration_id}.png").into(),
            mel_similarity: similarity,
            score: similarity,
            audio_metrics: None,
            audio_metric_errors: None,
        }
    }

    fn branch_node(iteration_id: &str, similarity: Option<f32>) -> LyrebirdBranchNode {
        LyrebirdBranchNode::from_generated(
            dsp_code(iteration_id, similarity),
            LyrebirdPatch {
                search: format!("old-{iteration_id}"),
                replacement: format!("new-{iteration_id}"),
                note: format!("patch {iteration_id}"),
            },
        )
    }

    fn submission(iteration_id: &str, score: f32) -> Submission<Vec<LyrebirdBranchNode>> {
        Submission {
            data: vec![branch_node(iteration_id, Some(score))],
            score,
        }
    }

    fn ecosystem(name: &str, max_tree_depth: usize) -> PulseCodePurgatory {
        let initial = LyrebirdInstrument::ALL.into_iter().map(|instrument| {
            (
                instrument,
                dsp_code(&format!("initial-{}", instrument.slug()), Some(0.1)),
            )
        });
        PulseCodePurgatory::new(
            Url::parse("https://api.openai.com/v1").unwrap(),
            None,
            Some(temp_db_path(name)),
        )
        .unwrap()
        .with_mcts_config(initial, max_tree_depth)
    }

    fn select_for_instrument(
        ecosystem: &PulseCodePurgatory,
        instrument: LyrebirdInstrument,
    ) -> Result<Vec<LyrebirdBranchNode>, PulseCodePurgatoryError> {
        match instrument {
            LyrebirdInstrument::RhythmGuitar => {
                ecosystem.select_lyrebird_branch_for_tag::<RhythmGuitarMarker>()
            }
            LyrebirdInstrument::Vocals => {
                ecosystem.select_lyrebird_branch_for_tag::<VocalsMarker>()
            }
            LyrebirdInstrument::BackupVocals => {
                ecosystem.select_lyrebird_branch_for_tag::<BackupVocalsMarker>()
            }
            LyrebirdInstrument::Bass => ecosystem.select_lyrebird_branch_for_tag::<BassMarker>(),
            LyrebirdInstrument::GuitarSolo => {
                ecosystem.select_lyrebird_branch_for_tag::<GuitarSoloMarker>()
            }
            LyrebirdInstrument::Drums => ecosystem.select_lyrebird_branch_for_tag::<DrumsMarker>(),
        }
    }

    fn submit_for_instrument(
        ecosystem: &PulseCodePurgatory,
        instrument: LyrebirdInstrument,
        submissions: Vec<Submission<Vec<LyrebirdBranchNode>>>,
    ) -> Result<(), PulseCodePurgatoryError> {
        match instrument {
            LyrebirdInstrument::RhythmGuitar => {
                ecosystem.submit_lyrebird_branch_for_tag::<RhythmGuitarMarker>(submissions)
            }
            LyrebirdInstrument::Vocals => {
                ecosystem.submit_lyrebird_branch_for_tag::<VocalsMarker>(submissions)
            }
            LyrebirdInstrument::BackupVocals => {
                ecosystem.submit_lyrebird_branch_for_tag::<BackupVocalsMarker>(submissions)
            }
            LyrebirdInstrument::Bass => {
                ecosystem.submit_lyrebird_branch_for_tag::<BassMarker>(submissions)
            }
            LyrebirdInstrument::GuitarSolo => {
                ecosystem.submit_lyrebird_branch_for_tag::<GuitarSoloMarker>(submissions)
            }
            LyrebirdInstrument::Drums => {
                ecosystem.submit_lyrebird_branch_for_tag::<DrumsMarker>(submissions)
            }
        }
    }

    fn load_tree_for_instrument(
        ecosystem: &PulseCodePurgatory,
        instrument: LyrebirdInstrument,
    ) -> Result<(StoredMctsTree, HashMap<u64, StoredMctsNode>), PulseCodePurgatoryError> {
        match instrument {
            LyrebirdInstrument::RhythmGuitar => {
                ecosystem.load_tree_for_test::<RhythmGuitarMarker>()
            }
            LyrebirdInstrument::Vocals => ecosystem.load_tree_for_test::<VocalsMarker>(),
            LyrebirdInstrument::BackupVocals => {
                ecosystem.load_tree_for_test::<BackupVocalsMarker>()
            }
            LyrebirdInstrument::Bass => ecosystem.load_tree_for_test::<BassMarker>(),
            LyrebirdInstrument::GuitarSolo => ecosystem.load_tree_for_test::<GuitarSoloMarker>(),
            LyrebirdInstrument::Drums => ecosystem.load_tree_for_test::<DrumsMarker>(),
        }
    }

    #[test]
    fn resolves_default_db_path_under_home_directory() {
        let db_path = resolve_mcts_db_path(None).unwrap();
        assert!(db_path.ends_with(".jungle/lyrebird/mcts.redb"));
    }

    #[test]
    fn persists_lyrebird_branch_and_backpropagates() {
        let db_path = temp_db_path("persisted-mcts");
        let tokens_url = Url::parse("https://api.openai.com/v1").unwrap();
        let initial = dsp_code("initial", Some(0.2));

        let first = PulseCodePurgatory::new(tokens_url.clone(), None, Some(db_path.clone()))
            .unwrap()
            .with_mcts_config([(LyrebirdInstrument::RhythmGuitar, initial.clone())], 8);
        let selected = select_for_instrument(&first, LyrebirdInstrument::RhythmGuitar).unwrap();
        assert_eq!(selected, vec![initial.clone().into()]);
        drop(first);

        let second = PulseCodePurgatory::new(tokens_url.clone(), None, Some(db_path.clone()))
            .unwrap()
            .with_mcts_config([(LyrebirdInstrument::RhythmGuitar, initial.clone())], 8);
        let recovered = select_for_instrument(&second, LyrebirdInstrument::RhythmGuitar).unwrap();
        assert_eq!(recovered, vec![initial.clone().into()]);
        let candidate = dsp_code("00000001", Some(0.75));
        submit_for_instrument(
            &second,
            LyrebirdInstrument::RhythmGuitar,
            vec![Submission {
                data: vec![candidate.clone().into()],
                score: 0.75,
            }],
        )
        .unwrap();
        drop(second);

        let third = PulseCodePurgatory::new(tokens_url, None, Some(db_path))
            .unwrap()
            .with_mcts_config([(LyrebirdInstrument::RhythmGuitar, initial)], 8);
        let (tree, nodes) =
            load_tree_for_instrument(&third, LyrebirdInstrument::RhythmGuitar).unwrap();
        let root = nodes.get(&ROOT_NODE_ID).unwrap();
        let child = nodes.get(&1).unwrap();

        assert_eq!(tree.pending_selected_node_id, None);
        assert_eq!(root.children, vec![1]);
        assert_eq!(root.visits, 1);
        assert!((root.total_score - 0.75).abs() < f64::EPSILON);
        assert_eq!(child.parent_id, Some(ROOT_NODE_ID));
        assert_eq!(child.visits, 1);
        assert_eq!(child.data, vec![candidate.into()]);
    }

    #[test]
    fn submit_can_add_multiple_children_for_one_selection() {
        let ecosystem = ecosystem("parallel-submit", 8);

        let _ = select_for_instrument(&ecosystem, LyrebirdInstrument::Bass).unwrap();
        submit_for_instrument(
            &ecosystem,
            LyrebirdInstrument::Bass,
            vec![submission("00000001", 0.6), submission("00000002", 0.8)],
        )
        .unwrap();

        let (tree, nodes) = load_tree_for_instrument(&ecosystem, LyrebirdInstrument::Bass).unwrap();
        let root = nodes.get(&ROOT_NODE_ID).unwrap();

        assert_eq!(tree.pending_selected_node_id, None);
        assert_eq!(root.children, vec![1, 2]);
        assert_eq!(root.visits, 2);
        assert!((root.total_score - 1.4).abs() < 1e-6);
        assert_eq!(nodes.get(&1).unwrap().visits, 1);
        assert_eq!(nodes.get(&2).unwrap().visits, 1);
    }

    #[test]
    fn instrument_trees_are_disambiguated() {
        let ecosystem = ecosystem("instrument-split", 8);
        let rhythm_branch =
            select_for_instrument(&ecosystem, LyrebirdInstrument::RhythmGuitar).unwrap();
        submit_for_instrument(
            &ecosystem,
            LyrebirdInstrument::RhythmGuitar,
            vec![submission("rhythm", 0.5)],
        )
        .unwrap();

        let vocals_branch = select_for_instrument(&ecosystem, LyrebirdInstrument::Vocals).unwrap();

        assert_eq!(rhythm_branch.len(), 1);
        assert_eq!(vocals_branch.len(), 1);
        assert_ne!(
            rhythm_branch[0].code.iteration_id,
            vocals_branch[0].code.iteration_id
        );
    }

    #[test]
    fn recovers_stale_pending_selection_after_restart() {
        let db_path = temp_db_path("stale-pending");
        let tokens_url = Url::parse("https://api.openai.com/v1").unwrap();
        let initial = dsp_code("initial", Some(0.2));

        let first = PulseCodePurgatory::new(tokens_url.clone(), None, Some(db_path.clone()))
            .unwrap()
            .with_mcts_config([(LyrebirdInstrument::Bass, initial.clone())], 8);
        let selected = select_for_instrument(&first, LyrebirdInstrument::Bass).unwrap();
        assert_eq!(selected, vec![initial.clone().into()]);
        drop(first);

        let second = PulseCodePurgatory::new(tokens_url, None, Some(db_path))
            .unwrap()
            .with_mcts_config([(LyrebirdInstrument::Bass, initial.clone())], 8);
        let recovered = select_for_instrument(&second, LyrebirdInstrument::Bass).unwrap();

        assert_eq!(recovered, vec![initial.into()]);
    }

    #[test]
    fn requires_select_and_submit_to_alternate() {
        let ecosystem = ecosystem("alternation", 8);

        let submit_err = submit_for_instrument(
            &ecosystem,
            LyrebirdInstrument::GuitarSolo,
            vec![submission("00000001", 0.3)],
        )
        .unwrap_err();
        assert!(
            submit_err
                .to_string()
                .contains("select must precede submit")
        );

        let _ = select_for_instrument(&ecosystem, LyrebirdInstrument::GuitarSolo).unwrap();
        let select_err =
            select_for_instrument(&ecosystem, LyrebirdInstrument::GuitarSolo).unwrap_err();
        assert!(select_err.to_string().contains("submit must follow select"));
    }

    #[test]
    fn empty_submission_clears_pending_selection_for_next_select() {
        let ecosystem = ecosystem("empty-submit-clears-pending", 8);

        let initial = select_for_instrument(&ecosystem, LyrebirdInstrument::Vocals).unwrap();
        assert_eq!(initial.len(), 1);

        submit_for_instrument(&ecosystem, LyrebirdInstrument::Vocals, Vec::new()).unwrap();

        let selected_again = select_for_instrument(&ecosystem, LyrebirdInstrument::Vocals).unwrap();

        assert_eq!(selected_again.len(), 1);
        let (tree, _nodes) =
            load_tree_for_instrument(&ecosystem, LyrebirdInstrument::Vocals).unwrap();
        assert!(tree.pending_selected_node_id.is_some());
    }

    #[test]
    fn excludes_terminal_depth_from_selection() {
        let ecosystem = ecosystem("max-depth", 2);

        let _ = select_for_instrument(&ecosystem, LyrebirdInstrument::BackupVocals).unwrap();
        submit_for_instrument(
            &ecosystem,
            LyrebirdInstrument::BackupVocals,
            vec![submission("00000001", 0.4)],
        )
        .unwrap();

        let branch = select_for_instrument(&ecosystem, LyrebirdInstrument::BackupVocals).unwrap();
        submit_for_instrument(
            &ecosystem,
            LyrebirdInstrument::BackupVocals,
            vec![submission("00000002", 0.8)],
        )
        .unwrap();

        let next_selected =
            select_for_instrument(&ecosystem, LyrebirdInstrument::BackupVocals).unwrap();

        assert!(branch.len() <= 2);
        assert!(next_selected.len() <= 2);
    }

    #[test]
    fn choose_expandable_node_bubbles_up_corrupt_tree_depth_errors() {
        let mut nodes = HashMap::new();
        nodes.insert(ROOT_NODE_ID, root_node());
        nodes.insert(
            1,
            StoredMctsNode {
                id: 1,
                parent_id: Some(999),
                data: vec![branch_node("00000001", Some(0.4))],
                visits: 1,
                total_score: 0.4,
                children: Vec::new(),
            },
        );

        let err = choose_expandable_node(&nodes, 8, LyrebirdInstrument::Vocals).unwrap_err();

        assert!(
            err.to_string()
                .contains("node 999 missing while computing vocals tree depth")
        );
    }

    #[test]
    fn selected_branch_retains_node_mel_paths() {
        let ecosystem = ecosystem("branch-mel-paths", 8);

        let initial_branch = select_for_instrument(&ecosystem, LyrebirdInstrument::Bass).unwrap();
        assert_eq!(initial_branch.len(), 1);
        assert_eq!(
            initial_branch[0].mel_spectrogram_path,
            "/tmp/initial-bass.png".to_owned()
        );

        submit_for_instrument(
            &ecosystem,
            LyrebirdInstrument::Bass,
            vec![submission("00000001", 0.6)],
        )
        .unwrap();

        let selected_branch = select_for_instrument(&ecosystem, LyrebirdInstrument::Bass).unwrap();

        assert_eq!(selected_branch.len(), 2);
        assert_eq!(
            selected_branch[0].mel_spectrogram_path,
            "/tmp/initial-bass.png".to_owned()
        );
        assert_eq!(
            selected_branch[1].mel_spectrogram_path,
            "/tmp/00000001.png".to_owned()
        );
        assert_eq!(
            selected_branch[1]
                .patch
                .as_ref()
                .map(|patch| patch.note.as_str()),
            Some("patch 00000001")
        );
    }
}
