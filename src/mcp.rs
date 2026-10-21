use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ErrorCode,
    Implementation, JsonObject, ListToolsResult, PaginatedRequestParams, ProtocolVersion,
    ServerCapabilities, ServerConfig, Tool, ToolAnnotations,
};
use rmcp::service::{ElicitationError, Peer, RequestContext};
use rmcp::{ErrorData as McpError, RoleServer, ServerHandler};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::batching::check_traits;
use crate::config::Config;
use crate::derive::{Approval, Approver, DeriveParams, derive};
use crate::doc::{Trait, TraitName};
use crate::error::ToolFailure;
use crate::provider::Models;
use crate::store::FileSystemStore;

pub const INSTRUCTIONS: &str = include_str!("prompts/instructions.md");
const DERIVE_DESCRIPTION: &str = include_str!("prompts/derive_trait.md");
const CHECK_DESCRIPTION: &str = include_str!("prompts/check_traits.md");

const DERIVE_FALLBACK: &str = "Surface this configuration failure to the user. Fix the reported error, then restart the Traits MCP server and retry the same trait derivation with the identical arguments. No trait was saved; keep the proposal pending until the error is resolved. Do not paste credentials into chat.";
const CHECK_FALLBACK: &str = "Surface this configuration failure to the user. Fix the reported error, then restart the Traits MCP server and retry the same check. No trait review was performed. Do not paste credentials into chat.";

pub struct Ready {
    pub config: Config,
    pub store: FileSystemStore,
    pub models: Models,
}

pub enum ServerState {
    Ready(Arc<Ready>),
    Failed(Arc<String>),
}

#[derive(Clone)]
pub struct TraitsServer {
    state: Arc<ServerState>,
}

impl TraitsServer {
    pub fn new(state: ServerState) -> Self {
        Self {
            state: Arc::new(state),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DeriveTraitParams {
    proposed_trait: Value,
    approved: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CheckTraitsParams {
    include: Vec<String>,
    circumstances: Option<String>,
    trait_names: Option<Vec<String>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum ElicitChoice {
    Approve,
    Revise,
    Skip,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct ElicitDecision {
    #[schemars(description = "Approve this exact proposal, request a revision, or skip it.")]
    decision: ElicitChoice,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Optional revision or alternative wording.")]
    feedback: Option<String>,
}

rmcp::elicit_safe!(ElicitDecision);

struct McpApprover {
    peer: Peer<RoleServer>,
    supports_form: bool,
    opencode: bool,
}

#[async_trait]
impl Approver for McpApprover {
    async fn request(
        &self,
        message: String,
        approved: Option<bool>,
    ) -> Result<Approval, ToolFailure> {
        if !self.supports_form || self.opencode {
            return Ok(match approved {
                Some(true) => Approval::Approved {},
                _ => Approval::ApprovalRequired { message },
            });
        }
        match self.peer.elicit::<ElicitDecision>(message).await {
            Ok(Some(response)) => {
                if matches!(response.decision, ElicitChoice::Skip) {
                    return Ok(Approval::Declined {});
                }
                let feedback = response
                    .feedback
                    .map(|value| value.trim().to_string())
                    .filter(|value| !value.is_empty());
                if matches!(response.decision, ElicitChoice::Revise) || feedback.is_some() {
                    return Ok(Approval::RevisionRequired {
                        feedback: feedback.unwrap_or_else(|| {
                            "Ask the user for revision details using an interactive question with free-text input.\nKeep this proposal pending; do not infer approval or cancel it because of a related follow-up task.".to_string()
                        }),
                    });
                }
                Ok(Approval::Approved {})
            }
            Ok(None) => Ok(Approval::Declined {}),
            Err(ElicitationError::UserDeclined | ElicitationError::UserCancelled) => {
                Ok(Approval::Declined {})
            }
            Err(error) => Err(ToolFailure::ai(format!("Elicitation failed: {error}"))),
        }
    }
}

impl TraitsServer {
    async fn derive_tool(
        &self,
        ready: &Ready,
        args: Value,
        context: &RequestContext<RoleServer>,
    ) -> CallToolResult {
        let params: DeriveTraitParams = match serde_json::from_value(args) {
            Ok(params) => params,
            Err(error) => {
                return failure_result(&ToolFailure::platform(format!(
                    "Invalid arguments: {error}"
                )));
            }
        };
        let proposed_trait: Trait = match serde_json::from_value(params.proposed_trait) {
            Ok(value) => value,
            Err(error) => {
                return failure_result(&ToolFailure::platform(format!(
                    "Invalid proposedTrait: {error}"
                )));
            }
        };
        let peer_info = context.peer.peer_info();
        let supports_form = peer_info
            .as_ref()
            .map(|info| {
                info.capabilities
                    .elicitation
                    .as_ref()
                    .is_some_and(|capability| capability.form.is_some() || capability.url.is_none())
            })
            .unwrap_or(false);
        let opencode = peer_info
            .as_ref()
            .map(|info| info.client_info.name.eq_ignore_ascii_case("opencode"))
            .unwrap_or(false)
            || context
                .meta
                .0
                .0
                .get("ai.opencode/sessionID")
                .and_then(Value::as_str)
                .is_some();
        let approver = McpApprover {
            peer: context.peer.clone(),
            supports_form,
            opencode,
        };
        match derive(
            &ready.models,
            &ready.store,
            &approver,
            DeriveParams {
                proposed_trait,
                approved: params.approved,
            },
        )
        .await
        {
            Ok(derivation) => success_result(&derivation),
            Err(failure) => failure_result(&failure),
        }
    }

    async fn check_tool(&self, ready: &Ready, args: Value) -> CallToolResult {
        let params: CheckTraitsParams = match serde_json::from_value(args) {
            Ok(params) => params,
            Err(error) => {
                return failure_result(&ToolFailure::platform(format!(
                    "Invalid arguments: {error}"
                )));
            }
        };
        let trait_names = match params.trait_names {
            Some(names) => match names
                .iter()
                .map(|name| TraitName::parse(name))
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(names) => Some(names),
                Err(error) => {
                    return failure_result(&ToolFailure::platform(error.to_string()));
                }
            },
            None => None,
        };
        match check_traits(
            &ready.config,
            &ready.store,
            &ready.models,
            &params.include,
            trait_names.as_deref(),
            params.circumstances.as_deref(),
        )
        .await
        {
            Ok(findings) => success_result(&findings),
            Err(failure) => failure_result(&failure),
        }
    }
}

impl ServerHandler for TraitsServer {
    fn get_info(&self) -> ServerConfig {
        let mut info = ServerConfig::new(ServerCapabilities::builder().enable_tools().build());
        info.server_info = Implementation::new("traits", env!("CARGO_PKG_VERSION"));
        info.instructions = Some(INSTRUCTIONS.to_string());
        info
    }

    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(&[ProtocolVersion::V_2025_11_25])
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let mut derive = Tool::new("derive_trait", DERIVE_DESCRIPTION, derive_schema());
        derive.title = Some("Derive Trait".to_string());
        derive.annotations = Some(ToolAnnotations::from_raw(
            None,
            Some(false),
            Some(true),
            Some(false),
            Some(true),
        ));

        let mut check = Tool::new("check_traits", CHECK_DESCRIPTION, check_schema());
        check.title = Some("Check Traits".to_string());
        check.annotations = Some(ToolAnnotations::from_raw(
            None,
            Some(true),
            Some(false),
            Some(false),
            Some(true),
        ));

        Ok(ListToolsResult::with_all_items(vec![derive, check]))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        let name = request.name.to_string();
        let ready = match &*self.state {
            ServerState::Ready(ready) => ready,
            ServerState::Failed(diagnostic) => {
                return Ok(configuration_failure(&name, diagnostic).into());
            }
        };
        let args = Value::Object(request.arguments.unwrap_or_default());
        let result = match name.as_str() {
            "derive_trait" => self.derive_tool(ready, args, &context).await,
            "check_traits" => self.check_tool(ready, args).await,
            _ => {
                return Err(McpError::new(
                    ErrorCode::METHOD_NOT_FOUND,
                    format!("Unknown tool: {name}"),
                    None,
                ));
            }
        };
        Ok(result.into())
    }
}

fn configuration_failure(tool: &str, diagnostic: &str) -> CallToolResult {
    let instructions = if tool == "derive_trait" {
        DERIVE_FALLBACK
    } else {
        CHECK_FALLBACK
    };
    CallToolResult::error(vec![ContentBlock::text(format!(
        "{diagnostic}\n\n{instructions}"
    ))])
}

fn success_result<T: Serialize>(value: &T) -> CallToolResult {
    let text = serde_json::to_string(value)
        .unwrap_or_else(|error| format!("Serialization failed: {error}"));
    CallToolResult::success(vec![ContentBlock::text(text)])
}

fn failure_result(failure: &ToolFailure) -> CallToolResult {
    let text = serde_json::to_string(failure).unwrap_or_else(|_| failure.message().to_string());
    CallToolResult::error(vec![ContentBlock::text(text)])
}

fn derive_schema() -> Arc<JsonObject> {
    Arc::new(
        json!({
            "type": "object",
            "properties": {
                "proposedTrait": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string",
                            "description": "The unique trait identity: a descriptive kebab-case name without a redundant trait- prefix, also used as its Markdown filename stem."
                        },
                        "subject": {
                            "type": "string",
                            "description": "The kind of artifact, action, transaction, relationship, or situation to identify and assess."
                        },
                        "requirement": {
                            "type": "string",
                            "description": "What should be true about the subject. The authoritative, standalone assessment criterion."
                        },
                        "applicability": {
                            "type": ["string", "null"],
                            "description": "Circumstances and exceptions determining when the requirement applies. Omit when there is no additional restriction beyond the subject."
                        },
                        "purpose": {
                            "type": "string",
                            "description": "What the trait aims to achieve or protect, and why that matters; not additional obligations."
                        },
                        "guidance": {
                            "type": ["string", "null"],
                            "description": "Advisory approach to satisfying the requirement or responding to a violation; never grants authority to act."
                        },
                        "context": {
                            "type": ["string", "null"],
                            "description": "Optional supporting Markdown context, examples, diffs, references, or procedural detail; not hidden requirements."
                        }
                    },
                    "required": ["name", "subject", "requirement", "purpose"],
                    "additionalProperties": true
                },
                "approved": {
                    "type": ["boolean", "null"],
                    "description": "Fallback approval for the exact proposal and automatically selected action shown to the user, including any existing trait being updated. Set true only after approval; native clients still receive a form. Does not bypass relationship checks."
                }
            },
            "required": ["proposedTrait"],
            "additionalProperties": true
        })
        .as_object()
        .expect("derive schema is an object")
        .clone(),
    )
}

fn check_schema() -> Arc<JsonObject> {
    Arc::new(
        json!({
            "type": "object",
            "properties": {
                "include": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Project-owned source, configuration, or documentation paths/globs, absolute or relative to the MCP working directory. Exclude third-party/generated files, secrets, and binaries. Overlapping matches are deduplicated; unmatched patterns select no files."
                },
                "circumstances": {
                    "type": ["string", "null"],
                    "description": "Optional circumstances triggering this assessment, such as the workflow stage, task, or relevant surrounding facts. Does not grant authority to execute trait guidance."
                },
                "traitNames": {
                    "type": ["array", "null"],
                    "items": { "type": "string", "maxLength": 80 },
                    "description": "Optional unique trait names for a focused review. Omit for all stored traits; an empty array selects none."
                }
            },
            "required": ["include"],
            "additionalProperties": true
        })
        .as_object()
        .expect("check schema is an object")
        .clone(),
    )
}
