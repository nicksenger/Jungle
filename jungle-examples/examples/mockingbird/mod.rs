use base64::prelude::{Engine as _, BASE64_STANDARD};
use clap::Parser;
use directories_next::BaseDirs;
use jungle_sdk::prelude::*;
use redb::{ReadableTable, TableDefinition};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;

mod effect;
pub mod mcts;
pub mod tokens;

use crate::mcts::SearchTree;
use crate::tokens::{Content, Message, TokenPredictor, Tool, ToolCall};

const DEFAULT_TOKENS_MODEL: &str = "qwen/qwen3.6-27b";
const ROOT_NODE_ID: u64 = 0;
const MCTS_TREES_TABLE: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("mockingbird_mcts_trees");
const MCTS_NODES_TABLE: TableDefinition<&[u8], &[u8]> =
    TableDefinition::new("mockingbird_mcts_nodes");

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub struct MockingBirdState;

pub type MockingBirdSeed = MockingBirdState;

pub struct MockingBirdIdle;
#[jungle::action]
impl Action for MockingBirdIdle {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(_state: &MockingBirdState, _input: Self::Input) -> Duration {
        Duration::from_millis(0)
    }

    fn absorb(
        _state: &mut MockingBirdState,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|_err| Failure::from("mockingbird idle stub should succeed"))?;
        Ok(())
    }
}

#[derive(Flow)]
pub struct MockingBirdJourney(Step<MockingBirdIdle>);

pub struct MockingBird;
#[jungle::animal(id = 0, generation = 0)]
impl Animal for MockingBird {
    type State = MockingBirdState;
    type Seed = MockingBirdSeed;
    type Flow = MockingBirdJourney;
}

#[derive(Animals)]
pub struct PulseCodeParadiseAnimals(MockingBird);

pub struct PulseCodeParadise {
    client: reqwest::Client,
    db: Arc<redb::Database>,
    db_path: PathBuf,
    tokens_model: String,
    tokens_url: Url,
    tools: Vec<Tool>,
}

impl PulseCodeParadise {
    pub fn new(
        tokens_url: Url,
        tokens_token: Option<String>,
        db_path: Option<PathBuf>,
    ) -> Result<Self, PulseCodeParadiseError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        if let Some(tokens_token) = tokens_token {
            let mut authorization = HeaderValue::from_str(&format!("Bearer {tokens_token}"))?;
            authorization.set_sensitive(true);
            headers.insert(AUTHORIZATION, authorization);
        }

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()?;
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

        Ok(Self {
            client,
            db: Arc::new(db),
            db_path,
            tokens_model: std::env::var("MOCKINGBIRD_TOKENS_MODEL")
                .unwrap_or_else(|_| DEFAULT_TOKENS_MODEL.to_owned()),
            tokens_url,
            tools: Vec::new(),
        })
    }

    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = tools;
        self
    }

    fn chat_completions_endpoint(&self) -> String {
        format!(
            "{}/chat/completions",
            self.tokens_url.as_str().trim_end_matches('/')
        )
    }

    fn chat_completions_request(
        &self,
        messages: &[Message],
    ) -> Result<Value, PulseCodeParadiseError> {
        let mut request = json!({
            "model": self.tokens_model,
            "messages": messages
                .iter()
                .map(openai_message)
                .collect::<Result<Vec<_>, _>>()?,
        });

        if !self.tools.is_empty() {
            request["tool_choice"] = Value::String("auto".to_owned());
            request["tools"] = Value::Array(self.tools.iter().map(openai_tool).collect());
        }

        Ok(request)
    }

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

impl Ecosystem for PulseCodeParadise {
    const NAME: &'static str = "pulse-code-paradise";
    type Animals = PulseCodeParadiseAnimals;
}

impl TokenPredictor for PulseCodeParadise {
    type Error = PulseCodeParadiseError;

    fn predict(
        &self,
        messages: Vec<Message>,
    ) -> impl std::future::Future<Output = Result<Vec<ToolCall>, Self::Error>> {
        async move {
            let request = self.chat_completions_request(&messages)?;
            let response = self
                .client
                .post(self.chat_completions_endpoint())
                .json(&request)
                .send()
                .await?
                .error_for_status()?
                .json::<OpenAiChatCompletionsResponse>()
                .await?;

            extract_tool_calls(response)
        }
    }
}

impl<Tag> SearchTree<Tag> for PulseCodeParadise
where
    Tag: Display,
{
    type Error = PulseCodeParadiseError;
    type Data = Value;

    fn select(
        &self,
        tag: Tag,
    ) -> impl std::future::Future<Output = Result<Self::Data, Self::Error>> {
        async move { self.select_mcts(&tag.to_string()) }
    }

    fn submit(
        &self,
        tag: Tag,
        data: Self::Data,
        score: f32,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> {
        async move { self.submit_mcts(&tag.to_string(), data, score) }
    }
}

#[derive(Debug, Error)]
pub enum PulseCodeParadiseError {
    #[error("failed to construct tokens client: {0}")]
    Client(#[from] reqwest::Error),
    #[error("invalid bearer token header: {0}")]
    InvalidHeader(#[from] reqwest::header::InvalidHeaderValue),
    #[error("failed to read image content from {path}: {source}")]
    ReadImage {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to create database directory {path}: {source}")]
    CreateDbDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("home directory is unavailable")]
    HomeDirUnavailable,
    #[error("mcts protocol error: {0}")]
    MctsProtocol(String),
    #[error("mcts persistence error: {0}")]
    Persistence(String),
    #[error("failed to parse tool-call arguments: {0}")]
    ToolArguments(#[from] serde_json::Error),
}

#[derive(Debug, Parser)]
#[command(name = "mockingbird")]
struct Cli {
    #[arg(long = "tokens-url")]
    tokens_url: Url,
    #[arg(long = "tokens-token")]
    tokens_token: Option<String>,
    #[arg(long = "db-path")]
    db_path: Option<PathBuf>,
}

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

#[derive(Debug, Deserialize)]
struct OpenAiChatCompletionsResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    #[serde(default)]
    tool_calls: Vec<OpenAiToolCall>,
}

#[derive(Debug, Deserialize)]
struct OpenAiToolCall {
    #[serde(default)]
    id: Option<String>,
    function: OpenAiFunctionCall,
}

#[derive(Debug, Deserialize)]
struct OpenAiFunctionCall {
    name: String,
    #[serde(default)]
    arguments: Option<OpenAiArguments>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum OpenAiArguments {
    String(String),
    Json(Value),
}

impl OpenAiArguments {
    fn into_json_value(self) -> Result<Value, serde_json::Error> {
        match self {
            Self::Json(value) => Ok(value),
            Self::String(arguments) if arguments.trim().is_empty() => {
                Ok(Value::Object(serde_json::Map::new()))
            }
            Self::String(arguments) => serde_json::from_str(&arguments),
        }
    }
}

fn openai_message(message: &Message) -> Result<Value, PulseCodeParadiseError> {
    Ok(json!({
        "role": message.role,
        "content": message.contents.iter().map(openai_content).collect::<Result<Vec<_>, _>>()?,
    }))
}

fn openai_content(content: &Content) -> Result<Value, PulseCodeParadiseError> {
    match content {
        Content::Text(text) => Ok(json!({
            "type": "text",
            "text": text,
        })),
        Content::Image(path) => Ok(json!({
            "type": "image_url",
            "image_url": {
                "url": image_data_url(path)?,
            },
        })),
    }
}

fn openai_tool(tool: &Tool) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description,
            "parameters": tool.parameters,
        },
    })
}

fn image_data_url(path: &Path) -> Result<String, PulseCodeParadiseError> {
    let bytes = std::fs::read(path).map_err(|source| PulseCodeParadiseError::ReadImage {
        path: path.to_path_buf(),
        source,
    })?;

    Ok(format!(
        "data:{};base64,{}",
        mime_type(path),
        BASE64_STANDARD.encode(bytes)
    ))
}

fn mime_type(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        _ => "application/octet-stream",
    }
}

fn extract_tool_calls(
    response: OpenAiChatCompletionsResponse,
) -> Result<Vec<ToolCall>, PulseCodeParadiseError> {
    response
        .choices
        .into_iter()
        .flat_map(|choice| choice.message.tool_calls)
        .map(|tool_call| {
            Ok(ToolCall {
                id: tool_call.id,
                name: tool_call.function.name,
                arguments: match tool_call.function.arguments {
                    Some(arguments) => arguments.into_json_value()?,
                    None => Value::Object(serde_json::Map::new()),
                },
            })
        })
        .collect()
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

fn main() -> Result<(), PulseCodeParadiseError> {
    let cli = Cli::parse();
    let ecosystem = PulseCodeParadise::new(cli.tokens_url, cli.tokens_token, cli.db_path)?;

    eprintln!(
        "mockingbird example stub configured for {} with mcts db {}",
        ecosystem.chat_completions_endpoint(),
        ecosystem.db_path.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::executor::block_on;
    use uuid::Uuid;

    fn temp_db_path(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join("jungle-mockingbird-tests")
            .join(format!("{name}-{}.redb", Uuid::new_v4()))
    }

    #[test]
    fn appends_chat_completions_path() {
        let ecosystem = PulseCodeParadise::new(
            Url::parse("https://api.openai.com/v1").unwrap(),
            None,
            Some(temp_db_path("endpoint")),
        )
        .unwrap();

        assert_eq!(
            ecosystem.chat_completions_endpoint(),
            "https://api.openai.com/v1/chat/completions"
        );
    }

    #[test]
    fn parses_tool_calls_from_chat_completions_response() {
        let response: OpenAiChatCompletionsResponse = serde_json::from_value(json!({
            "choices": [
                {
                    "message": {
                        "tool_calls": [
                            {
                                "id": "call_123",
                                "function": {
                                    "name": "insert_node",
                                    "arguments": "{\"score\":0.8}"
                                }
                            }
                        ]
                    }
                }
            ]
        }))
        .unwrap();

        let tool_calls = extract_tool_calls(response).unwrap();

        assert_eq!(
            tool_calls,
            vec![ToolCall {
                id: Some("call_123".to_owned()),
                name: "insert_node".to_owned(),
                arguments: json!({ "score": 0.8 }),
            }]
        );
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
        let tag = "bassline";

        let first =
            PulseCodeParadise::new(tokens_url.clone(), None, Some(db_path.clone())).unwrap();
        let selected = block_on(SearchTree::select(&first, tag)).unwrap();
        assert_eq!(selected, Value::Null);
        drop(first);

        let second =
            PulseCodeParadise::new(tokens_url.clone(), None, Some(db_path.clone())).unwrap();
        block_on(SearchTree::submit(
            &second,
            tag,
            json!({ "prompt": "add syncopation" }),
            0.75,
        ))
        .unwrap();
        drop(second);

        let third = PulseCodeParadise::new(tokens_url, None, Some(db_path.clone())).unwrap();
        let (tree, nodes) = third.load_tree_for_test(tag).unwrap();
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

        let submit_err = block_on(SearchTree::submit(
            &ecosystem,
            "drums",
            json!({ "prompt": "fill" }),
            0.3,
        ))
        .unwrap_err();
        assert!(submit_err
            .to_string()
            .contains("select must precede submit"));

        let _ = block_on(SearchTree::select(&ecosystem, "drums")).unwrap();
        let select_err = block_on(SearchTree::select(&ecosystem, "drums")).unwrap_err();
        assert!(select_err.to_string().contains("submit must follow select"));
    }
}
