use crate::condition::FlattenEither;
use jungle_sdk::prelude::*;
use jungle_sdk::typosaurus::collections::list;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::future::Future;
use std::marker::PhantomData;
use std::time::Duration;

const DEFAULT_IDLE_POLL_MS: u64 = 250;
const DEFAULT_MAX_ROUNDS_PER_TURN: u32 = 16;
const DEFAULT_MAX_TOOL_CALLS_PER_ROUND: u32 = 32;

pub trait Tool {
    const NAME: &'static str;
    type Effect: EffectMember + EffectSchema<In = Self::Args, Out = Self::Out, Err = Self::Err>;
    type Args: Serialize + DeserializeOwned + Send + 'static;
    type Out: Serialize + DeserializeOwned + Send + 'static;
    type Err: Serialize + DeserializeOwned + Send + 'static;

    fn description() -> &'static str;
    fn parameters_schema_json() -> &'static str;
}

pub trait ToolList {
    fn definitions() -> Vec<AgentToolDefinition>;
}

impl ToolList for list::Empty {
    fn definitions() -> Vec<AgentToolDefinition> {
        Vec::new()
    }
}

impl<Head, Tail> ToolList for list::List<(Head, Tail)>
where
    Head: Tool,
    Tail: ToolList,
{
    fn definitions() -> Vec<AgentToolDefinition> {
        let mut definitions = vec![AgentToolDefinition {
            name: Head::NAME.to_owned(),
            description: Head::description().to_owned(),
            parameters_schema_json: Head::parameters_schema_json().to_owned(),
        }];
        definitions.extend(Tail::definitions());
        definitions
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentInput {
    pub prompt: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentModelConfig {
    pub base_url: String,
    pub model: String,
    pub bearer_token: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSettings {
    pub idle_poll_ms: u64,
    pub max_rounds_per_turn: u32,
    pub max_tool_calls_per_round: u32,
}

impl Default for AgentSettings {
    fn default() -> Self {
        Self {
            idle_poll_ms: DEFAULT_IDLE_POLL_MS,
            max_rounds_per_turn: DEFAULT_MAX_ROUNDS_PER_TURN,
            max_tool_calls_per_round: DEFAULT_MAX_TOOL_CALLS_PER_ROUND,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelToolCall {
    pub id: String,
    pub name: String,
    pub arguments_raw: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TranscriptEntry {
    System {
        content: String,
    },
    User {
        content: String,
    },
    Assistant {
        content: Option<String>,
        tool_calls: Vec<ModelToolCall>,
    },
    Tool {
        tool_call_id: String,
        name: String,
        content: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters_schema_json: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentModelRequest {
    pub model_config: AgentModelConfig,
    pub transcript: Vec<TranscriptEntry>,
    pub tools: Vec<AgentToolDefinition>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentModelTurn {
    pub assistant_content: Option<String>,
    pub tool_calls: Vec<ModelToolCall>,
}

#[derive(Optic, Debug, Serialize, Deserialize)]
pub struct AgentState<St, Tools> {
    pub pending_inputs: VecDeque<AgentInput>,
    pub transcript: Vec<TranscriptEntry>,
    pub pending_tool_calls: VecDeque<ModelToolCall>,
    pub active_input: Option<AgentInput>,
    pub active_tool_call: Option<ModelToolCall>,
    pub awaiting_model_turn: bool,
    pub turn_complete: bool,
    pub turn_round_index: u32,
    pub tool_calls_in_round: u32,
    pub model_config: AgentModelConfig,
    pub settings: AgentSettings,
    pub last_error: Option<String>,
    #[jungle(focus)]
    pub st: St,
    #[serde(skip)]
    marker: PhantomData<Tools>,
}

impl<St, Tools> AgentState<St, Tools> {
    pub fn new(st: St, model_config: AgentModelConfig) -> Self {
        Self {
            pending_inputs: VecDeque::new(),
            transcript: Vec::new(),
            pending_tool_calls: VecDeque::new(),
            active_input: None,
            active_tool_call: None,
            awaiting_model_turn: true,
            turn_complete: false,
            turn_round_index: 0,
            tool_calls_in_round: 0,
            model_config,
            settings: AgentSettings::default(),
            last_error: None,
            st,
            marker: PhantomData,
        }
    }

    pub fn with_settings(mut self, settings: AgentSettings) -> Self {
        self.settings = settings;
        self
    }

    pub fn enqueue_input(&mut self, input: AgentInput) {
        self.pending_inputs.push_back(input);
    }
}

impl<St, Tools> Clone for AgentState<St, Tools>
where
    St: Clone,
{
    fn clone(&self) -> Self {
        Self {
            pending_inputs: self.pending_inputs.clone(),
            transcript: self.transcript.clone(),
            pending_tool_calls: self.pending_tool_calls.clone(),
            active_input: self.active_input.clone(),
            active_tool_call: self.active_tool_call.clone(),
            awaiting_model_turn: self.awaiting_model_turn,
            turn_complete: self.turn_complete,
            turn_round_index: self.turn_round_index,
            tool_calls_in_round: self.tool_calls_in_round,
            model_config: self.model_config.clone(),
            settings: self.settings,
            last_error: self.last_error.clone(),
            st: self.st.clone(),
            marker: PhantomData,
        }
    }
}

impl<St, Tools> Default for AgentState<St, Tools>
where
    St: Default,
{
    fn default() -> Self {
        Self::new(
            St::default(),
            AgentModelConfig {
                base_url: "http://localhost:11434/v1".to_owned(),
                model: "gpt-4.1".to_owned(),
                bearer_token: None,
            },
        )
    }
}

impl<St, Tools> ViewProject<AgentState<St, Tools>> for AgentState<St, Tools> {
    fn project_view(state: &mut Self) -> &mut AgentState<St, Tools> {
        state
    }
}

pub struct RequestAgentModelTurnEffect;
#[jungle::effect(id = 931)]
impl<J> Effect<J> for RequestAgentModelTurnEffect {
    type In = AgentModelRequest;
    type Out = AgentModelTurn;
    type Err = String;

    fn effect(_jungle: &J, input: Self::In) -> impl Future<Output = Result<Self::Out, Self::Err>> {
        async move { request_agent_model_turn(input).await }
    }
}

async fn request_agent_model_turn(input: AgentModelRequest) -> Result<AgentModelTurn, String> {
    let base_url = parse_openai_api_base_url(&input.model_config.base_url)?;
    let endpoint = chat_completions_endpoint(&base_url);
    let request = build_openai_chat_completions_request(&input)?;
    let client = build_openai_client(input.model_config.bearer_token.as_deref())?;

    let response = client
        .post(endpoint)
        .json(&request)
        .send()
        .await
        .map_err(|err| format!("failed to send chat completions request: {err}"))?
        .error_for_status()
        .map_err(|err| format!("chat completions returned error status: {err}"))?
        .json::<OpenAiChatCompletionsResponse>()
        .await
        .map_err(|err| format!("failed to decode chat completions response: {err}"))?;

    extract_agent_model_turn(response)
}

fn build_openai_client(token: Option<&str>) -> Result<reqwest::Client, String> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    if let Some(token) = token {
        let mut authorization =
            HeaderValue::from_str(&format!("Bearer {token}")).map_err(|err| err.to_string())?;
        authorization.set_sensitive(true);
        headers.insert(AUTHORIZATION, authorization);
    }

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(|err| format!("failed to build OpenAI client: {err}"))
}

fn build_openai_chat_completions_request(input: &AgentModelRequest) -> Result<Value, String> {
    let mut request = json!({
        "model": input.model_config.model,
        "messages": input
            .transcript
            .iter()
            .map(openai_message_from_transcript)
            .collect::<Result<Vec<_>, _>>()?,
    });

    if !input.tools.is_empty() {
        request["tool_choice"] = Value::String("auto".to_owned());
        request["tools"] = Value::Array(
            input
                .tools
                .iter()
                .map(openai_tool_from_definition)
                .collect::<Result<Vec<_>, _>>()?,
        );
    }

    Ok(request)
}

fn openai_message_from_transcript(entry: &TranscriptEntry) -> Result<Value, String> {
    match entry {
        TranscriptEntry::System { content } => Ok(json!({
            "role": "system",
            "content": content,
        })),
        TranscriptEntry::User { content } => Ok(json!({
            "role": "user",
            "content": content,
        })),
        TranscriptEntry::Assistant {
            content,
            tool_calls,
        } => {
            let mut message = json!({
                "role": "assistant",
                "content": content.as_ref().map_or(Value::Null, |value| Value::String(value.clone())),
            });
            if !tool_calls.is_empty() {
                message["tool_calls"] = Value::Array(
                    tool_calls
                        .iter()
                        .map(|tool_call| {
                            json!({
                                "id": tool_call.id,
                                "type": "function",
                                "function": {
                                    "name": tool_call.name,
                                    "arguments": tool_call.arguments_raw,
                                },
                            })
                        })
                        .collect(),
                );
            }
            Ok(message)
        }
        TranscriptEntry::Tool {
            tool_call_id,
            name,
            content,
        } => Ok(json!({
            "role": "tool",
            "tool_call_id": tool_call_id,
            "name": name,
            "content": content,
        })),
    }
}

fn openai_tool_from_definition(tool: &AgentToolDefinition) -> Result<Value, String> {
    let parameters = if tool.parameters_schema_json.trim().is_empty() {
        json!({
            "type": "object",
            "properties": {},
        })
    } else {
        serde_json::from_str::<Value>(&tool.parameters_schema_json).map_err(|err| {
            format!(
                "tool `{}` schema is not valid JSON ({}): {}",
                tool.name, tool.parameters_schema_json, err
            )
        })?
    };

    Ok(json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description,
            "parameters": parameters,
        }
    }))
}

fn parse_openai_api_base_url(value: &str) -> Result<reqwest::Url, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err("OpenAI API base URL is empty".to_owned());
    }

    let candidate = if value.contains("://") {
        value.to_owned()
    } else {
        format!("http://{value}")
    };

    let base_url = reqwest::Url::parse(&candidate)
        .map_err(|_| format!("failed to parse OpenAI API base URL `{value}`"))?;
    validate_openai_api_base_url(&base_url)?;
    Ok(base_url)
}

fn validate_openai_api_base_url(base_url: &reqwest::Url) -> Result<(), String> {
    match base_url.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(format!(
                "unsupported URL scheme `{scheme}` for OpenAI API base URL `{base_url}`"
            ));
        }
    }

    if base_url.query().is_some() {
        return Err(format!(
            "OpenAI API base URL `{base_url}` must not include query parameters"
        ));
    }
    if base_url.fragment().is_some() {
        return Err(format!(
            "OpenAI API base URL `{base_url}` must not include fragments"
        ));
    }

    let path_segments: Vec<_> = base_url
        .path_segments()
        .map(|segments| {
            segments
                .filter(|segment| !segment.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if path_segments.ends_with(&["chat", "completions"]) {
        return Err(format!(
            "OpenAI API base URL `{base_url}` must be the API base, not `/chat/completions`"
        ));
    }

    Ok(())
}

fn chat_completions_endpoint(base_url: &reqwest::Url) -> String {
    format!(
        "{}/chat/completions",
        base_url.as_str().trim_end_matches('/')
    )
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
    content: Option<OpenAiMessageContent>,
    #[serde(default)]
    tool_calls: Vec<OpenAiToolCall>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum OpenAiMessageContent {
    String(String),
    Parts(Vec<OpenAiContentPart>),
}

#[derive(Debug, Deserialize)]
struct OpenAiContentPart {
    #[serde(rename = "type", default)]
    part_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
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
    fn into_json_string(self) -> String {
        match self {
            Self::Json(value) => value.to_string(),
            Self::String(arguments) if arguments.trim().is_empty() => "{}".to_owned(),
            // Preserve malformed argument payloads so the model can self-correct
            // on a follow-up tool call instead of failing the whole turn.
            Self::String(arguments) => arguments,
        }
    }
}

fn extract_agent_model_turn(
    response: OpenAiChatCompletionsResponse,
) -> Result<AgentModelTurn, String> {
    if response.choices.is_empty() {
        return Err("chat completions response did not include any choices".to_owned());
    }

    let assistant_content = response
        .choices
        .first()
        .and_then(|choice| render_openai_content(choice.message.content.as_ref()));
    let mut tool_calls = Vec::new();

    for (choice_index, choice) in response.choices.into_iter().enumerate() {
        for (tool_call_index, tool_call) in choice.message.tool_calls.into_iter().enumerate() {
            tool_calls.push(ModelToolCall {
                id: tool_call
                    .id
                    .unwrap_or_else(|| format!("call_{choice_index}_{tool_call_index}")),
                name: tool_call.function.name,
                arguments_raw: tool_call
                    .function
                    .arguments
                    .map(OpenAiArguments::into_json_string)
                    .unwrap_or_else(|| "{}".to_owned()),
            });
        }
    }

    Ok(AgentModelTurn {
        assistant_content,
        tool_calls,
    })
}

fn render_openai_content(content: Option<&OpenAiMessageContent>) -> Option<String> {
    match content {
        Some(OpenAiMessageContent::String(content)) if !content.trim().is_empty() => {
            Some(content.clone())
        }
        Some(OpenAiMessageContent::Parts(parts)) => {
            let text = parts
                .iter()
                .filter(|part| part.part_type.as_deref().is_none_or(|kind| kind == "text"))
                .filter_map(|part| part.text.as_deref())
                .collect::<Vec<_>>()
                .join("\n");
            if text.trim().is_empty() {
                None
            } else {
                Some(text)
            }
        }
        _ => None,
    }
}

fn tool_error_payload(message: impl Into<String>) -> String {
    let message = message.into();
    serde_json::to_string(&json!({ "error": message })).unwrap_or_else(|_| {
        format!(
            "{{\"error\":\"{}\"}}",
            message.replace('\\', "\\\\").replace('\"', "\\\"")
        )
    })
}

fn to_json_string<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|err| format!("{{\"error\":\"failed to serialize tool output: {err}\"}}"))
}

fn active_tool_identity<St, Tools>(state: &AgentState<St, Tools>) -> (String, String) {
    match state.active_tool_call.as_ref() {
        Some(tool_call) => (tool_call.id.clone(), tool_call.name.clone()),
        None => ("missing_tool_call".to_owned(), "unknown".to_owned()),
    }
}

fn append_tool_result<St, Tools>(
    state: &mut AgentState<St, Tools>,
    tool_call_id: String,
    name: String,
    content: String,
) {
    state.transcript.push(TranscriptEntry::Tool {
        tool_call_id,
        name,
        content,
    });
    state.tool_calls_in_round = state.tool_calls_in_round.saturating_add(1);
    if state.settings.max_tool_calls_per_round > 0
        && state.tool_calls_in_round >= state.settings.max_tool_calls_per_round
    {
        state.turn_complete = true;
        state.awaiting_model_turn = false;
        state.pending_tool_calls.clear();
        state.last_error = Some(format!(
            "agent round hit max tool calls ({})",
            state.settings.max_tool_calls_per_round
        ));
    }
}

pub struct AgentLoopForever<St, Tools>(PhantomData<fn() -> (St, Tools)>);
impl<St, Tools> Predicate<(&AgentState<St, Tools>, &())> for AgentLoopForever<St, Tools> {
    fn eval((_state, _): &(&AgentState<St, Tools>, &())) -> bool {
        true
    }
}

pub struct AgentHasWork<St, Tools>(PhantomData<fn() -> (St, Tools)>);
impl<St, Tools> Predicate<(AgentState<St, Tools>, ())> for AgentHasWork<St, Tools> {
    fn eval((state, _): &(AgentState<St, Tools>, ())) -> bool {
        state.active_input.is_some() || !state.pending_inputs.is_empty()
    }
}

pub struct AgentTurnPending<St, Tools>(PhantomData<fn() -> (St, Tools)>);
impl<St, Tools> Predicate<(&AgentState<St, Tools>, &())> for AgentTurnPending<St, Tools> {
    fn eval((state, _): &(&AgentState<St, Tools>, &())) -> bool {
        state.active_input.is_some() && !state.turn_complete
    }
}

pub struct AgentAwaitingModelTurn<St, Tools>(PhantomData<fn() -> (St, Tools)>);
impl<St, Tools> Predicate<(AgentState<St, Tools>, ())> for AgentAwaitingModelTurn<St, Tools> {
    fn eval((state, _): &(AgentState<St, Tools>, ())) -> bool {
        state.awaiting_model_turn
    }
}

pub struct AgentHasPendingToolCalls<St, Tools>(PhantomData<fn() -> (St, Tools)>);
impl<St, Tools> Predicate<(&AgentState<St, Tools>, &())> for AgentHasPendingToolCalls<St, Tools> {
    fn eval((state, _): &(&AgentState<St, Tools>, &())) -> bool {
        !state.pending_tool_calls.is_empty() && !state.turn_complete
    }
}

pub struct CurrentToolMatches<St, Tools, ToolT>(PhantomData<fn() -> (St, Tools, ToolT)>);
impl<St, Tools, ToolT> Predicate<(AgentState<St, Tools>, ())>
    for CurrentToolMatches<St, Tools, ToolT>
where
    ToolT: Tool,
{
    fn eval((state, _): &(AgentState<St, Tools>, ())) -> bool {
        state
            .active_tool_call
            .as_ref()
            .is_some_and(|tool_call| tool_call.name == ToolT::NAME)
    }
}

pub struct ParsedToolArgsOk<St, Tools, ToolT>(PhantomData<fn() -> (St, Tools, ToolT)>);
impl<St, Tools, ToolT> Predicate<(AgentState<St, Tools>, Result<ToolT::Args, String>)>
    for ParsedToolArgsOk<St, Tools, ToolT>
where
    ToolT: Tool,
{
    fn eval((_state, args): &(AgentState<St, Tools>, Result<ToolT::Args, String>)) -> bool {
        args.is_ok()
    }
}

pub struct SleepWhenIdle<St, Tools>(PhantomData<fn() -> (St, Tools)>);
#[jungle::action]
impl<St, Tools> Action for SleepWhenIdle<St, Tools> {
    type Effect = Sleep;
    type Input = ();
    type Output = ();

    fn emit(state: &AgentState<St, Tools>, _input: Self::Input) -> Duration {
        Duration::from_millis(state.settings.idle_poll_ms.max(1))
    }

    fn absorb(
        _state: &mut AgentState<St, Tools>,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        output.map_err(|err| Failure::Message(err.message))?;
        Ok(())
    }
}

pub struct EnsureActiveInput<St, Tools>(PhantomData<fn() -> (St, Tools)>);
#[jungle::action]
impl<St, Tools> Action for EnsureActiveInput<St, Tools> {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &AgentState<St, Tools>, _input: Self::Input) {}

    fn absorb(
        state: &mut AgentState<St, Tools>,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        if state.active_input.is_none() {
            if let Some(input) = state.pending_inputs.pop_front() {
                state.transcript.push(TranscriptEntry::User {
                    content: input.prompt.clone(),
                });
                state.active_input = Some(input);
            }
            state.awaiting_model_turn = true;
            state.turn_complete = false;
            state.turn_round_index = 0;
            state.tool_calls_in_round = 0;
            state.last_error = None;
            state.pending_tool_calls.clear();
            state.active_tool_call = None;
        }

        Ok(())
    }
}

pub struct RequestModelTurn<St, Tools>(PhantomData<fn() -> (St, Tools)>);
#[jungle::action]
impl<St, Tools> Action for RequestModelTurn<St, Tools>
where
    Tools: ToolList,
{
    type Effect = RequestAgentModelTurnEffect;
    type Input = ();
    type Output = ();

    fn emit(state: &AgentState<St, Tools>, _input: Self::Input) -> AgentModelRequest {
        AgentModelRequest {
            model_config: state.model_config.clone(),
            transcript: state.transcript.clone(),
            tools: Tools::definitions(),
        }
    }

    fn absorb(
        state: &mut AgentState<St, Tools>,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        match output {
            Ok(turn) => {
                state.transcript.push(TranscriptEntry::Assistant {
                    content: turn.assistant_content.clone(),
                    tool_calls: turn.tool_calls.clone(),
                });
                if turn.tool_calls.is_empty() {
                    state.turn_complete = true;
                    state.awaiting_model_turn = false;
                } else {
                    state.turn_round_index = state.turn_round_index.saturating_add(1);
                    state.tool_calls_in_round = 0;
                    state.pending_tool_calls = turn.tool_calls.into();
                    state.awaiting_model_turn = false;

                    if state.settings.max_rounds_per_turn > 0
                        && state.turn_round_index > state.settings.max_rounds_per_turn
                    {
                        state.last_error = Some(format!(
                            "agent turn hit max rounds ({})",
                            state.settings.max_rounds_per_turn
                        ));
                        state.turn_complete = true;
                        state.pending_tool_calls.clear();
                    }
                }
            }
            Err(err) => {
                state.last_error = Some(err);
                state.turn_complete = true;
                state.awaiting_model_turn = false;
                state.pending_tool_calls.clear();
            }
        }
        Ok(())
    }
}

pub struct TakeNextToolCall<St, Tools>(PhantomData<fn() -> (St, Tools)>);
#[jungle::action]
impl<St, Tools> Action for TakeNextToolCall<St, Tools> {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &AgentState<St, Tools>, _input: Self::Input) {}

    fn absorb(
        state: &mut AgentState<St, Tools>,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        state.active_tool_call = state.pending_tool_calls.pop_front();
        Ok(())
    }
}

pub struct DecodeToolArgs<St, Tools, ToolT>(PhantomData<fn() -> (St, Tools, ToolT)>);
#[jungle::action]
impl<St, Tools, ToolT> Action for DecodeToolArgs<St, Tools, ToolT>
where
    ToolT: Tool,
{
    type Effect = NoEffect;
    type Input = ();
    type Output = Result<ToolT::Args, String>;
    type Carry = Result<ToolT::Args, String>;

    fn emit(
        state: &AgentState<St, Tools>,
        _input: Self::Input,
    ) -> ((), Result<ToolT::Args, String>) {
        let parsed = match state.active_tool_call.as_ref() {
            Some(tool_call) => {
                let args_text = if tool_call.arguments_raw.trim().is_empty() {
                    "{}"
                } else {
                    tool_call.arguments_raw.as_str()
                };
                serde_json::from_str::<ToolT::Args>(args_text).map_err(|err| {
                    format!(
                        "invalid `{}` tool arguments for call `{}`: {}",
                        ToolT::NAME,
                        tool_call.id,
                        err
                    )
                })
            }
            None => Err("missing active tool call".to_owned()),
        };

        ((), parsed)
    }

    fn absorb(
        _state: &mut AgentState<St, Tools>,
        _output: EffectCompletion<Self::Effect>,
        carry: Result<ToolT::Args, String>,
    ) -> Result<Self::Output, Failure> {
        Ok(carry)
    }
}

pub struct InvokeDecodedTool<St, Tools, ToolT>(PhantomData<fn() -> (St, Tools, ToolT)>);
#[jungle::action]
impl<St, Tools, ToolT> Action for InvokeDecodedTool<St, Tools, ToolT>
where
    ToolT: Tool,
{
    type Effect = ToolT::Effect;
    type Input = Result<ToolT::Args, String>;
    type Output = ();

    fn emit(_state: &AgentState<St, Tools>, input: Self::Input) -> ToolT::Args {
        match input {
            Ok(parsed) => parsed,
            Err(_) => panic!("InvokeDecodedTool must only run for successfully parsed arguments"),
        }
    }

    fn absorb(
        state: &mut AgentState<St, Tools>,
        output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let (tool_call_id, tool_name) = active_tool_identity(state);
        let content = match output {
            Ok(result) => to_json_string(&result),
            Err(err) => tool_error_payload(to_json_string(&err)),
        };
        append_tool_result(state, tool_call_id, tool_name, content);
        Ok(())
    }
}

pub struct RecordToolArgError<St, Tools, ToolT>(PhantomData<fn() -> (St, Tools, ToolT)>);
#[jungle::action]
impl<St, Tools, ToolT> Action for RecordToolArgError<St, Tools, ToolT>
where
    ToolT: Tool,
{
    type Effect = NoEffect;
    type Input = Result<ToolT::Args, String>;
    type Output = ();
    type Carry = Result<ToolT::Args, String>;

    fn emit(
        _state: &AgentState<St, Tools>,
        input: Self::Input,
    ) -> ((), Result<ToolT::Args, String>) {
        ((), input)
    }

    fn absorb(
        state: &mut AgentState<St, Tools>,
        _output: EffectCompletion<Self::Effect>,
        carry: Result<ToolT::Args, String>,
    ) -> Result<Self::Output, Failure> {
        let (tool_call_id, tool_name) = active_tool_identity(state);
        let message = match carry {
            Ok(_) => format!("`{}` argument parsing failed unexpectedly", ToolT::NAME),
            Err(err) => err,
        };
        append_tool_result(state, tool_call_id, tool_name, tool_error_payload(message));
        Ok(())
    }
}

pub struct RecordUnknownToolCall<St, Tools>(PhantomData<fn() -> (St, Tools)>);
#[jungle::action]
impl<St, Tools> Action for RecordUnknownToolCall<St, Tools> {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &AgentState<St, Tools>, _input: Self::Input) {}

    fn absorb(
        state: &mut AgentState<St, Tools>,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        let (tool_call_id, tool_name) = active_tool_identity(state);
        append_tool_result(
            state,
            tool_call_id,
            tool_name.clone(),
            tool_error_payload(format!("unknown tool `{tool_name}`")),
        );
        Ok(())
    }
}

pub struct ClearActiveToolCall<St, Tools>(PhantomData<fn() -> (St, Tools)>);
#[jungle::action]
impl<St, Tools> Action for ClearActiveToolCall<St, Tools> {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &AgentState<St, Tools>, _input: Self::Input) {}

    fn absorb(
        state: &mut AgentState<St, Tools>,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        state.active_tool_call = None;
        Ok(())
    }
}

pub struct PrepareReprompt<St, Tools>(PhantomData<fn() -> (St, Tools)>);
#[jungle::action]
impl<St, Tools> Action for PrepareReprompt<St, Tools> {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &AgentState<St, Tools>, _input: Self::Input) {}

    fn absorb(
        state: &mut AgentState<St, Tools>,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        if !state.turn_complete {
            state.awaiting_model_turn = true;
        }
        Ok(())
    }
}

pub struct FinalizeTurn<St, Tools>(PhantomData<fn() -> (St, Tools)>);
#[jungle::action]
impl<St, Tools> Action for FinalizeTurn<St, Tools> {
    type Effect = NoEffect;
    type Input = ();
    type Output = ();

    fn emit(_state: &AgentState<St, Tools>, _input: Self::Input) {}

    fn absorb(
        state: &mut AgentState<St, Tools>,
        _output: EffectCompletion<Self::Effect>,
    ) -> Result<Self::Output, Failure> {
        state.active_input = None;
        state.active_tool_call = None;
        state.pending_tool_calls.clear();
        state.awaiting_model_turn = true;
        state.turn_complete = false;
        state.turn_round_index = 0;
        state.tool_calls_in_round = 0;
        Ok(())
    }
}

#[derive(Flow)]
pub struct ExecuteKnownTool<St, Tools, ToolT: Tool>(
    Step<DecodeToolArgs<St, Tools, ToolT>>,
    Conditional<
        ParsedToolArgsOk<St, Tools, ToolT>,
        Step<InvokeDecodedTool<St, Tools, ToolT>>,
        Step<RecordToolArgError<St, Tools, ToolT>>,
    >,
    Step<FlattenEither<(), AgentState<St, Tools>>>,
);

pub trait ToolDispatchFlow<St, Tools, Remaining> {
    type Flow;
}

pub trait DispatchableTools<St>: ToolList {
    type DispatchFlow;
}

impl<St, Tools> DispatchableTools<St> for Tools
where
    Tools: ToolList,
    (): ToolDispatchFlow<St, Tools, Tools>,
{
    type DispatchFlow = <() as ToolDispatchFlow<St, Tools, Tools>>::Flow;
}

impl<St, Tools> ToolDispatchFlow<St, Tools, list::Empty> for () {
    type Flow = Step<RecordUnknownToolCall<St, Tools>>;
}

#[derive(Flow)]
pub struct ToolDispatchBranch<St, Tools, ToolT: Tool, TailFlow>(
    Conditional<CurrentToolMatches<St, Tools, ToolT>, ExecuteKnownTool<St, Tools, ToolT>, TailFlow>,
    Step<FlattenEither<(), AgentState<St, Tools>>>,
);

impl<St, Tools, Head, Tail> ToolDispatchFlow<St, Tools, list::List<(Head, Tail)>> for ()
where
    Head: Tool,
    (): ToolDispatchFlow<St, Tools, Tail>,
{
    type Flow =
        ToolDispatchBranch<St, Tools, Head, <() as ToolDispatchFlow<St, Tools, Tail>>::Flow>;
}

#[derive(Flow)]
pub struct ExecuteOneToolCall<St, Tools: DispatchableTools<St>>(
    Step<TakeNextToolCall<St, Tools>>,
    <Tools as DispatchableTools<St>>::DispatchFlow,
    Step<ClearActiveToolCall<St, Tools>>,
);

#[derive(Flow)]
pub struct AgentModelBranch<St, Tools: ToolList>(Step<RequestModelTurn<St, Tools>>);

#[derive(Flow)]
pub struct AgentToolBranch<St, Tools: DispatchableTools<St>>(
    While<AgentHasPendingToolCalls<St, Tools>, ExecuteOneToolCall<St, Tools>>,
    Step<PrepareReprompt<St, Tools>>,
);

#[derive(Flow)]
pub struct AgentTurnBody<St, Tools: DispatchableTools<St>>(
    Conditional<
        AgentAwaitingModelTurn<St, Tools>,
        AgentModelBranch<St, Tools>,
        AgentToolBranch<St, Tools>,
    >,
    Step<FlattenEither<(), AgentState<St, Tools>>>,
);

#[derive(Flow)]
pub struct AgentTurnFlow<St, Tools: DispatchableTools<St>>(
    Step<EnsureActiveInput<St, Tools>>,
    While<AgentTurnPending<St, Tools>, AgentTurnBody<St, Tools>>,
    Step<FinalizeTurn<St, Tools>>,
);

#[derive(Flow)]
pub struct AgentIdleFlow<St, Tools: ToolList>(Step<SleepWhenIdle<St, Tools>>);

#[derive(Flow)]
pub struct AgentLoopBody<St, Tools: DispatchableTools<St>>(
    Conditional<AgentHasWork<St, Tools>, AgentTurnFlow<St, Tools>, AgentIdleFlow<St, Tools>>,
    Step<FlattenEither<(), AgentState<St, Tools>>>,
);

#[derive(Flow)]
pub struct AgentFlow<St, Tools: DispatchableTools<St>>(
    While<AgentLoopForever<St, Tools>, AgentLoopBody<St, Tools>>,
);

pub type Agent<St, Tools> = AgentFlow<St, Tools>;

#[cfg(test)]
mod tests {
    use super::*;

    struct SendSlackMsg;
    #[jungle::effect(id = 932)]
    impl<J> Effect<J> for SendSlackMsg {
        type In = String;
        type Out = String;
        type Err = String;

        fn effect(
            _jungle: &J,
            input: Self::In,
        ) -> impl Future<Output = Result<Self::Out, Self::Err>> {
            std::future::ready(Ok(input))
        }
    }

    impl Tool for SendSlackMsg {
        const NAME: &'static str = "send_slack_msg";
        type Effect = SendSlackMsg;
        type Args = String;
        type Out = String;
        type Err = String;

        fn description() -> &'static str {
            "Send a Slack message"
        }

        fn parameters_schema_json() -> &'static str {
            r#"{"type":"string"}"#
        }
    }

    #[test]
    fn tool_list_collects_definitions() {
        type Tools = list::List<(SendSlackMsg, list::Empty)>;
        let definitions = <Tools as ToolList>::definitions();
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].name, "send_slack_msg");
    }

    #[test]
    fn openai_base_url_rejects_chat_completions_endpoint() {
        let err = parse_openai_api_base_url("https://api.openai.com/v1/chat/completions")
            .expect_err("chat completions endpoint should be rejected");
        assert!(err.contains("/chat/completions"));
    }

    #[test]
    fn extract_model_turn_preserves_raw_malformed_arguments() {
        let response: OpenAiChatCompletionsResponse = serde_json::from_value(json!({
            "choices": [
                {
                    "message": {
                        "content": "working",
                        "tool_calls": [
                            {
                                "id": "call_123",
                                "function": {
                                    "name": "send_slack_msg",
                                    "arguments": "{\"unterminated"
                                }
                            }
                        ]
                    }
                }
            ]
        }))
        .unwrap();

        let turn = extract_agent_model_turn(response).unwrap();
        assert_eq!(turn.assistant_content, Some("working".to_owned()));
        assert_eq!(turn.tool_calls.len(), 1);
        assert_eq!(turn.tool_calls[0].arguments_raw, "{\"unterminated");
    }
}
