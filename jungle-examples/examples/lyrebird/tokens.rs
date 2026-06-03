use super::{LyrebirdInstrument, PulseCodeParadise, PulseCodeParadiseError};
use base64::prelude::{Engine as _, BASE64_STANDARD};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::future::Future;
use std::path::Path;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Content {
    Text(String),
    Image(PathBuf),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub contents: Vec<Content>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: Option<String>,
    pub name: String,
    pub arguments: String,
}

impl ToolCall {
    pub fn arguments_json_value(&self) -> Result<Value, serde_json::Error> {
        if self.arguments.trim().is_empty() {
            Ok(Value::Object(serde_json::Map::new()))
        } else {
            serde_json::from_str(&self.arguments)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Prompt {
    pub messages: Vec<Message>,
    pub tools: Vec<Tool>,
}

pub trait TokenPredictor {
    type Error;
    type Meta;

    fn predict(
        &self,
        prompt: Prompt,
        meta: Option<Self::Meta>,
    ) -> impl Future<Output = Result<Vec<ToolCall>, Self::Error>> + Send;
}

const DEFAULT_TOKENS_MODEL: &str = "qwen/qwen3.6-27b";

impl PulseCodeParadise {
    pub(crate) fn build_tokens_client(
        tokens_token: Option<&str>,
    ) -> Result<reqwest::Client, PulseCodeParadiseError> {
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        if let Some(tokens_token) = tokens_token {
            let mut authorization = HeaderValue::from_str(&format!("Bearer {tokens_token}"))?;
            authorization.set_sensitive(true);
            headers.insert(AUTHORIZATION, authorization);
        }

        reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .map_err(PulseCodeParadiseError::from)
    }

    pub(crate) fn tokens_model_from_env() -> String {
        std::env::var("LYREBIRD_TOKENS_MODEL").unwrap_or_else(|_| DEFAULT_TOKENS_MODEL.to_owned())
    }

    pub(crate) fn chat_completions_endpoint(&self) -> String {
        format!(
            "{}/chat/completions",
            self.tokens_url.as_str().trim_end_matches('/')
        )
    }

    fn chat_completions_request(&self, prompt: &Prompt) -> Result<Value, PulseCodeParadiseError> {
        let mut request = json!({
            "model": self.tokens_model,
            "messages": prompt
                .messages
                .iter()
                .map(openai_message)
                .collect::<Result<Vec<_>, _>>()?,
        });

        let effective_tools: Vec<&Tool> = self.tools.iter().chain(prompt.tools.iter()).collect();
        if !effective_tools.is_empty() {
            request["tool_choice"] = Value::String("auto".to_owned());
            request["tools"] = Value::Array(effective_tools.into_iter().map(openai_tool).collect());
        }

        Ok(request)
    }
}

impl TokenPredictor for PulseCodeParadise {
    type Error = PulseCodeParadiseError;
    type Meta = LyrebirdInstrument;

    fn predict(
        &self,
        prompt: Prompt,
        _meta: Option<Self::Meta>,
    ) -> impl Future<Output = Result<Vec<ToolCall>, Self::Error>> + Send {
        async move {
            let request = self.chat_completions_request(&prompt)?;
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
    fn into_json_string(self) -> Result<String, serde_json::Error> {
        match self {
            Self::Json(value) => Ok(value.to_string()),
            Self::String(arguments) if arguments.trim().is_empty() => Ok("{}".to_owned()),
            // Preserve raw tool-call argument strings from the model response.
            // If the payload is truncated or malformed, `lyrebird` should
            // retry the instrument prompt instead of failing the whole journey
            // during response decoding.
            Self::String(arguments) => Ok(arguments),
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
                    Some(arguments) => arguments.into_json_string()?,
                    None => "{}".to_owned(),
                },
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::Url;
    use uuid::Uuid;

    fn temp_db_path(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join("jungle-lyrebird-tests")
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
                arguments: "{\"score\":0.8}".to_owned(),
            }]
        );
    }

    #[test]
    fn request_uses_prompt_tools() {
        let ecosystem = PulseCodeParadise::new(
            Url::parse("https://api.openai.com/v1").unwrap(),
            None,
            Some(temp_db_path("request-tools")),
        )
        .unwrap();
        let request = ecosystem
            .chat_completions_request(&Prompt {
                messages: vec![Message {
                    role: "user".to_owned(),
                    contents: vec![Content::Text("hello".to_owned())],
                }],
                tools: vec![Tool {
                    name: "insert_node".to_owned(),
                    description: "Insert a node".to_owned(),
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "score": { "type": "number" }
                        }
                    }),
                }],
            })
            .unwrap();

        assert_eq!(request["tools"][0]["function"]["name"], "insert_node");
        assert_eq!(request["tool_choice"], "auto");
    }

    #[test]
    fn tool_calls_round_trip_through_postcard() {
        let tool_calls = vec![ToolCall {
            id: Some("call_123".to_owned()),
            name: "insert_node".to_owned(),
            arguments: "{\"score\":0.8}".to_owned(),
        }];

        let bytes = postcard::to_allocvec(&tool_calls).unwrap();
        let decoded = postcard::from_bytes::<Vec<ToolCall>>(&bytes).unwrap();

        assert_eq!(decoded, tool_calls);
    }

    #[test]
    fn preserves_malformed_tool_call_argument_strings_for_retry() {
        let response: OpenAiChatCompletionsResponse = serde_json::from_value(json!({
            "choices": [
                {
                    "message": {
                        "tool_calls": [
                            {
                                "id": "call_123",
                                "function": {
                                    "name": "replace_rhythm_guitar_dsp",
                                    "arguments": "{\"source\":\"unterminated"
                                }
                            }
                        ]
                    }
                }
            ]
        }))
        .unwrap();

        let tool_calls = extract_tool_calls(response).unwrap();

        assert_eq!(tool_calls.len(), 1);
        assert_eq!(tool_calls[0].arguments, "{\"source\":\"unterminated");
    }
}
