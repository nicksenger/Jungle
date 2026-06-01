use base64::prelude::{Engine as _, BASE64_STANDARD};
use clap::Parser;
use jungle_sdk::prelude::*;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;
use thiserror::Error;

mod effect;
pub mod mcts;
pub mod tokens;

use crate::tokens::{Content, Message, TokenPredictor, Tool, ToolCall};

const DEFAULT_TOKENS_MODEL: &str = "qwen/qwen3.6-27b";

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
    tokens_model: String,
    tokens_url: Url,
    tools: Vec<Tool>,
}

impl PulseCodeParadise {
    pub fn new(
        tokens_url: Url,
        tokens_token: Option<String>,
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

        Ok(Self {
            client,
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

fn main() -> Result<(), PulseCodeParadiseError> {
    let cli = Cli::parse();
    let ecosystem = PulseCodeParadise::new(cli.tokens_url, cli.tokens_token)?;

    eprintln!(
        "mockingbird example stub configured for {}",
        ecosystem.chat_completions_endpoint()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_chat_completions_path() {
        let ecosystem =
            PulseCodeParadise::new(Url::parse("https://api.openai.com/v1").unwrap(), None).unwrap();

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
}
