//! Ergonomic command and interaction routing framework for Discord bots.
//!
//! Provides [`CommandRouter`] for routing application commands, message components,
//! modals, and autocomplete interactions, [`EventContext`] for responding to interactions
//! with full access to REST and cache, and [`CachedClient`] / [`Framework`] for automated
//! cache updates and event loops.

use crate::client::Client;
use rift_core::{Permissions, Snowflake};
use rift_model::GatewayEvent;
use rift_rest::component::ActionRow;
use rift_rest::{
    Choice, CommandData, CommandOption, CommandSpec, ComponentData, Interaction,
    InteractionMessage, InteractionResponse, InteractionType, Member, ModalSubmitData,
    ReqwestTransport, RestClient, User,
};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
#[cfg(feature = "cache")]
use std::sync::RwLock;

/// Extension trait for parsing raw [`rift_runtime::Event`] instances into typed [`rift_model::GatewayEvent`].
pub trait EventModelExt {
    /// Parse the raw runtime event payload into a typed [`GatewayEvent`].
    ///
    /// Unknown gateway events parse into [`GatewayEvent::Unknown`] and do not fail.
    fn parse_model(&self) -> Result<GatewayEvent, serde_json::Error>;
}

impl EventModelExt for rift_runtime::Event {
    fn parse_model(&self) -> Result<GatewayEvent, serde_json::Error> {
        GatewayEvent::parse(&self.name, &self.data)
    }
}

/// Errors that can occur during framework interaction dispatch and command routing.
#[derive(Debug)]
pub enum FrameworkError {
    /// Deserialization of incoming interaction or model payload failed.
    Parse(serde_json::Error),
    /// Interaction payload was missing required fields or data.
    MissingData(String),
    /// A Discord REST API error occurred while responding or executing follow-ups.
    Rest(rift_rest::Error),
    /// Error returned from a user command, component, modal, or autocomplete handler.
    Handler(Box<dyn std::error::Error + Send + Sync>),
}

impl std::fmt::Display for FrameworkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "framework parse error: {e}"),
            Self::MissingData(s) => write!(f, "framework missing data: {s}"),
            Self::Rest(e) => write!(f, "framework REST error: {e}"),
            Self::Handler(e) => write!(f, "framework handler error: {e}"),
        }
    }
}

impl std::error::Error for FrameworkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Parse(e) => Some(e),
            Self::Rest(e) => Some(e),
            Self::Handler(e) => Some(e.as_ref()),
            Self::MissingData(_) => None,
        }
    }
}

impl From<serde_json::Error> for FrameworkError {
    fn from(err: serde_json::Error) -> Self {
        Self::Parse(err)
    }
}

impl From<rift_rest::Error> for FrameworkError {
    fn from(err: rift_rest::Error) -> Self {
        Self::Rest(err)
    }
}

impl From<String> for FrameworkError {
    fn from(s: String) -> Self {
        Self::MissingData(s)
    }
}

impl From<&str> for FrameworkError {
    fn from(s: &str) -> Self {
        Self::MissingData(s.to_string())
    }
}

impl FrameworkError {
    /// Construct a custom error from any standard boxed or implementor error.
    pub fn custom(err: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Handler(Box::new(err))
    }
}

/// Return type for interaction and command handlers.
pub type HandlerResult = Result<(), FrameworkError>;

/// Boxed future returned by an asynchronous interaction handler.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Handler function signature for interactions.
pub type CommandHandler =
    Arc<dyn Fn(EventContext) -> BoxFuture<'static, HandlerResult> + Send + Sync + 'static>;

/// Context for an incoming interaction dispatch, providing access to interaction details,
/// convenient response helpers, the REST client, and optional state cache.
#[derive(Clone)]
pub struct EventContext {
    /// The incoming interaction.
    pub interaction: Interaction,
    /// Shared rate-limited REST client.
    pub rest: Arc<RestClient<ReqwestTransport>>,
    /// Optional shared state cache.
    #[cfg(feature = "cache")]
    pub cache: Option<Arc<RwLock<rift_cache::Cache>>>,
}

impl EventContext {
    /// Create a new event context.
    #[cfg(feature = "cache")]
    pub fn new(
        interaction: Interaction,
        rest: Arc<RestClient<ReqwestTransport>>,
        cache: Option<Arc<RwLock<rift_cache::Cache>>>,
    ) -> Self {
        Self {
            interaction,
            rest,
            cache,
        }
    }

    /// Create a new event context without cache.
    #[cfg(not(feature = "cache"))]
    pub fn new(
        interaction: Interaction,
        rest: Arc<RestClient<ReqwestTransport>>,
    ) -> Self {
        Self {
            interaction,
            rest,
        }
    }

    /// Interaction ID.
    pub fn id(&self) -> Snowflake {
        self.interaction.id
    }

    /// Interaction continuation token.
    pub fn token(&self) -> &str {
        &self.interaction.token
    }

    /// Application ID for which this interaction was invoked.
    pub fn application_id(&self) -> Snowflake {
        self.interaction.application_id
    }

    /// Kind of interaction (ApplicationCommand, MessageComponent, ModalSubmit, etc.).
    pub fn kind(&self) -> InteractionType {
        self.interaction.kind
    }

    /// Guild ID where the interaction was invoked, if applicable.
    pub fn guild_id(&self) -> Option<Snowflake> {
        self.interaction.guild_id
    }

    /// Channel ID where the interaction was invoked, if applicable.
    pub fn channel_id(&self) -> Option<Snowflake> {
        self.interaction.channel_id
    }

    /// The user who invoked the interaction (from either guild member or DM user).
    pub fn user(&self) -> Option<&User> {
        self.interaction.invoking_user()
    }

    /// The invoking guild member, if the interaction occurred in a guild.
    pub fn member(&self) -> Option<&Member> {
        self.interaction.member.as_ref()
    }

    /// Selected locale of the invoking user.
    pub fn locale(&self) -> Option<&str> {
        self.interaction.locale.as_deref()
    }

    /// Permissions of the invoking member in the channel, if present.
    pub fn member_permissions(&self) -> Option<Permissions> {
        self.interaction.member_permissions()
    }

    /// Permissions of the bot application in the channel, if present.
    pub fn app_permissions(&self) -> Option<Permissions> {
        self.interaction.app_permissions()
    }

    /// Parsed application command data for command and autocomplete interactions.
    pub fn command_data(&self) -> Result<CommandData, rift_rest::Error> {
        self.interaction.command()
    }

    /// Parsed component data for message component interactions.
    pub fn component_data(&self) -> Result<ComponentData, rift_rest::Error> {
        self.interaction.component()
    }

    /// Parsed modal submit data for modal submit interactions.
    pub fn modal_data(&self) -> Result<ModalSubmitData, rift_rest::Error> {
        self.interaction.modal_submit()
    }

    /// The focused command option being autocompleted.
    pub fn autocomplete_option(&self) -> Result<CommandOption, rift_rest::Error> {
        self.interaction.autocomplete()
    }

    /// Returns the `custom_id` of the interacting component or modal submission, if present.
    pub fn custom_id(&self) -> Option<String> {
        match self.interaction.kind {
            InteractionType::MessageComponent => self.component_data().ok().map(|c| c.custom_id),
            InteractionType::ModalSubmit => self.modal_data().ok().map(|m| m.custom_id),
            _ => None,
        }
    }

    /// Returns the `custom_id` stripped of the specified prefix, if it matches.
    pub fn stripped_custom_id(&self, prefix: &str) -> Option<String> {
        let id = self.custom_id()?;
        id.strip_prefix(prefix).map(|s| s.to_string())
    }

    /// Finds a submitted modal text input value by component `custom_id`.
    pub fn modal_value(&self, custom_id: &str) -> Option<String> {
        let data = self.modal_data().ok()?;
        data.value(custom_id).map(|s| s.to_string())
    }

    /// Shared rate-limited REST client reference.
    pub fn rest(&self) -> &Arc<RestClient<ReqwestTransport>> {
        &self.rest
    }

    /// Reference to the optional state cache.
    #[cfg(feature = "cache")]
    pub fn cache(&self) -> Option<&Arc<RwLock<rift_cache::Cache>>> {
        self.cache.as_ref()
    }

    // --- Response Helpers ---

    /// Sends an initial interaction response.
    pub async fn respond(&self, response: &InteractionResponse) -> Result<(), rift_rest::Error> {
        self.rest
            .create_interaction_response(self.interaction.id, &self.interaction.token, response)
            .await
    }

    /// Sends a quick text reply (`CHANNEL_MESSAGE_WITH_SOURCE`).
    pub async fn reply(&self, content: impl Into<String>) -> Result<(), rift_rest::Error> {
        let msg = InteractionMessage::new().content(content);
        self.respond(&InteractionResponse::message(msg)).await
    }

    /// Sends a quick ephemeral text reply (`CHANNEL_MESSAGE_WITH_SOURCE` with `EPHEMERAL` flag).
    pub async fn reply_ephemeral(&self, content: impl Into<String>) -> Result<(), rift_rest::Error> {
        let msg = InteractionMessage::new().content(content).ephemeral();
        self.respond(&InteractionResponse::message(msg)).await
    }

    /// Responds with a complete custom [`InteractionMessage`].
    pub async fn reply_message(&self, message: InteractionMessage) -> Result<(), rift_rest::Error> {
        self.respond(&InteractionResponse::message(message)).await
    }

    /// Defers the interaction response (`DEFERRED_CHANNEL_MESSAGE_WITH_SOURCE`), showing a loading state.
    pub async fn defer(&self, ephemeral: bool) -> Result<(), rift_rest::Error> {
        self.respond(&InteractionResponse::defer(ephemeral)).await
    }

    /// Defers a component interaction update (`DEFERRED_UPDATE_MESSAGE`).
    pub async fn defer_update(&self) -> Result<(), rift_rest::Error> {
        self.respond(&InteractionResponse::defer_update()).await
    }

    /// Updates the message to which a component was attached (`UPDATE_MESSAGE`).
    pub async fn update_message(&self, message: InteractionMessage) -> Result<(), rift_rest::Error> {
        self.respond(&InteractionResponse::update_message(message)).await
    }

    /// Edits the initial interaction response within its 15-minute validity window.
    pub async fn edit_original(
        &self,
        message: &InteractionMessage,
    ) -> Result<rift_rest::Message, rift_rest::Error> {
        self.rest
            .edit_original_response(
                self.interaction.application_id,
                &self.interaction.token,
                message,
            )
            .await
    }

    /// Deletes the initial interaction response within its 15-minute validity window.
    pub async fn delete_original(&self) -> Result<(), rift_rest::Error> {
        self.rest
            .delete_original_response(self.interaction.application_id, &self.interaction.token)
            .await
    }

    /// Creates a follow-up message for the interaction.
    pub async fn create_followup(
        &self,
        message: &InteractionMessage,
    ) -> Result<rift_rest::Message, rift_rest::Error> {
        self.rest
            .create_followup(
                self.interaction.application_id,
                &self.interaction.token,
                message,
            )
            .await
    }

    /// Responds to an autocomplete interaction with the provided choices.
    pub async fn autocomplete(&self, choices: Vec<Choice>) -> Result<(), rift_rest::Error> {
        let resp = InteractionResponse::autocomplete(choices)
            .map_err(|e| rift_rest::Error::Decode(e.to_string()))?;
        self.respond(&resp).await
    }

    /// Responds with a modal popup.
    pub async fn modal(
        &self,
        custom_id: impl Into<String>,
        title: impl Into<String>,
        rows: impl IntoIterator<Item = ActionRow>,
    ) -> Result<(), rift_rest::Error> {
        let resp = InteractionResponse::modal(custom_id, title, rows.into_iter().collect())
            .map_err(|e| rift_rest::Error::Decode(e.to_string()))?;
        self.respond(&resp).await
    }

    // --- Cache Math Helpers ---

    /// Reads the cached guild if a cache is present and the interaction occurred in a guild.
    #[cfg(feature = "cache")]
    pub fn cached_guild(&self) -> Option<rift_model::guild::Guild> {
        let gid = self.guild_id()?;
        let cache = self.cache.as_ref()?;
        let guard = cache.read().ok()?;
        guard.guild(gid).cloned()
    }

    /// Reads the cached channel if a cache is present and visible.
    #[cfg(feature = "cache")]
    pub fn cached_channel(&self) -> Option<rift_model::channel::Channel> {
        let cid = self.channel_id()?;
        let cache = self.cache.as_ref()?;
        let guard = cache.read().ok()?;
        guard.visible_channel(cid).cloned()
    }

    /// Computes the effective permissions for the invoking user in the current channel
    /// using the attached cache, evaluating ownership bypass, admin bypass, role/member overwrites,
    /// timeouts, and obfuscated channel rules.
    #[cfg(feature = "cache")]
    pub fn compute_permissions(&self) -> Option<Permissions> {
        let gid = self.guild_id()?;
        let cid = self.channel_id()?;
        let uid = self.user()?.id;
        let cache = self.cache.as_ref()?;
        let guard = cache.read().ok()?;
        guard.member_permissions_in_channel(gid, cid, uid)
    }
}

/// Routing engine for application commands, message components, modals, and autocomplete interactions.
#[derive(Clone, Default)]
pub struct CommandRouter {
    commands: Vec<CommandSpec>,
    slash_handlers: HashMap<String, CommandHandler>,
    user_command_handlers: HashMap<String, CommandHandler>,
    message_command_handlers: HashMap<String, CommandHandler>,
    component_exact: HashMap<String, CommandHandler>,
    component_prefix: Vec<(String, CommandHandler)>,
    modal_exact: HashMap<String, CommandHandler>,
    modal_prefix: Vec<(String, CommandHandler)>,
    autocomplete_exact: HashMap<(String, String), CommandHandler>,
    autocomplete_command: HashMap<String, CommandHandler>,
}

impl CommandRouter {
    /// Creates a new empty command router.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns all registered command specifications for registration with Discord.
    pub fn commands(&self) -> &[CommandSpec] {
        &self.commands
    }

    /// Register a command specification along with its async execution handler.
    pub fn command<F, Fut>(mut self, spec: CommandSpec, handler: F) -> Self
    where
        F: Fn(EventContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HandlerResult> + Send + 'static,
    {
        let val = serde_json::to_value(&spec).unwrap_or(serde_json::Value::Null);
        let name = val
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let kind = val.get("type").and_then(|v| v.as_u64()).unwrap_or(1);

        let boxed: CommandHandler = Arc::new(move |ctx| Box::pin(handler(ctx)));

        match kind {
            1 => {
                self.slash_handlers.insert(name, boxed);
            }
            2 => {
                self.user_command_handlers.insert(name, boxed);
            }
            3 => {
                self.message_command_handlers.insert(name, boxed);
            }
            _ => {}
        }

        self.commands.push(spec);
        self
    }

    /// Register a slash command by name and description.
    pub fn slash<F, Fut>(self, name: impl Into<String>, description: impl Into<String>, handler: F) -> Self
    where
        F: Fn(EventContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HandlerResult> + Send + 'static,
    {
        let spec = CommandSpec::slash(name, description);
        self.command(spec, handler)
    }

    /// Register a user context-menu command by name.
    pub fn user_command<F, Fut>(self, name: impl Into<String>, handler: F) -> Self
    where
        F: Fn(EventContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HandlerResult> + Send + 'static,
    {
        let spec = CommandSpec::user(name);
        self.command(spec, handler)
    }

    /// Register a message context-menu command by name.
    pub fn message_command<F, Fut>(self, name: impl Into<String>, handler: F) -> Self
    where
        F: Fn(EventContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HandlerResult> + Send + 'static,
    {
        let spec = CommandSpec::message(name);
        self.command(spec, handler)
    }

    /// Register a component handler matching an exact `custom_id`.
    pub fn component_exact<F, Fut>(mut self, custom_id: impl Into<String>, handler: F) -> Self
    where
        F: Fn(EventContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HandlerResult> + Send + 'static,
    {
        let boxed: CommandHandler = Arc::new(move |ctx| Box::pin(handler(ctx)));
        self.component_exact.insert(custom_id.into(), boxed);
        self
    }

    /// Register a component handler matching any `custom_id` starting with `prefix`.
    pub fn component_prefix<F, Fut>(mut self, prefix: impl Into<String>, handler: F) -> Self
    where
        F: Fn(EventContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HandlerResult> + Send + 'static,
    {
        let boxed: CommandHandler = Arc::new(move |ctx| Box::pin(handler(ctx)));
        self.component_prefix.push((prefix.into(), boxed));
        self
    }

    /// Register a modal submit handler matching an exact `custom_id`.
    pub fn modal_exact<F, Fut>(mut self, custom_id: impl Into<String>, handler: F) -> Self
    where
        F: Fn(EventContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HandlerResult> + Send + 'static,
    {
        let boxed: CommandHandler = Arc::new(move |ctx| Box::pin(handler(ctx)));
        self.modal_exact.insert(custom_id.into(), boxed);
        self
    }

    /// Register a modal submit handler matching any `custom_id` starting with `prefix`.
    pub fn modal_prefix<F, Fut>(mut self, prefix: impl Into<String>, handler: F) -> Self
    where
        F: Fn(EventContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HandlerResult> + Send + 'static,
    {
        let boxed: CommandHandler = Arc::new(move |ctx| Box::pin(handler(ctx)));
        self.modal_prefix.push((prefix.into(), boxed));
        self
    }

    /// Register an autocomplete handler for a specific command name and focused option name.
    pub fn autocomplete<F, Fut>(
        mut self,
        command_name: impl Into<String>,
        option_name: impl Into<String>,
        handler: F,
    ) -> Self
    where
        F: Fn(EventContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HandlerResult> + Send + 'static,
    {
        let boxed: CommandHandler = Arc::new(move |ctx| Box::pin(handler(ctx)));
        self.autocomplete_exact
            .insert((command_name.into(), option_name.into()), boxed);
        self
    }

    /// Register an autocomplete handler for any autocompleted option in a command.
    pub fn autocomplete_command<F, Fut>(mut self, command_name: impl Into<String>, handler: F) -> Self
    where
        F: Fn(EventContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = HandlerResult> + Send + 'static,
    {
        let boxed: CommandHandler = Arc::new(move |ctx| Box::pin(handler(ctx)));
        self.autocomplete_command.insert(command_name.into(), boxed);
        self
    }

    /// Dispatch an incoming interaction context to the appropriate handler.
    ///
    /// Returns `Ok(true)` if a handler matched and executed, `Ok(false)` if no route matched,
    /// or `Err(FrameworkError)` if parsing or handler execution failed.
    pub async fn dispatch(&self, ctx: &EventContext) -> Result<bool, FrameworkError> {
        match ctx.kind() {
            InteractionType::ApplicationCommand => {
                let data = ctx.command_data()?;
                let kind = data.kind.unwrap_or(1);
                let handler = match kind {
                    1 => self.slash_handlers.get(&data.name),
                    2 => self.user_command_handlers.get(&data.name),
                    3 => self.message_command_handlers.get(&data.name),
                    _ => None,
                };
                if let Some(h) = handler {
                    h(ctx.clone()).await?;
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            InteractionType::MessageComponent => {
                let data = ctx.component_data()?;
                if let Some(h) = self.component_exact.get(&data.custom_id) {
                    h(ctx.clone()).await?;
                    return Ok(true);
                }
                for (prefix, h) in &self.component_prefix {
                    if data.custom_id.starts_with(prefix) {
                        h(ctx.clone()).await?;
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            InteractionType::ModalSubmit => {
                let data = ctx.modal_data()?;
                if let Some(h) = self.modal_exact.get(&data.custom_id) {
                    h(ctx.clone()).await?;
                    return Ok(true);
                }
                for (prefix, h) in &self.modal_prefix {
                    if data.custom_id.starts_with(prefix) {
                        h(ctx.clone()).await?;
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            InteractionType::ApplicationCommandAutocomplete => {
                let data = ctx.command_data()?;
                if let Ok(opt) = ctx.autocomplete_option() {
                    let key = (data.name.clone(), opt.name.clone());
                    if let Some(h) = self.autocomplete_exact.get(&key) {
                        h(ctx.clone()).await?;
                        return Ok(true);
                    }
                }
                if let Some(h) = self.autocomplete_command.get(&data.name) {
                    h(ctx.clone()).await?;
                    return Ok(true);
                }
                Ok(false)
            }
            InteractionType::Ping => {
                ctx.respond(&InteractionResponse::pong()).await?;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    /// Dispatches a raw gateway event to the router if it is an `INTERACTION_CREATE`.
    #[cfg(feature = "cache")]
    pub async fn dispatch_raw(
        &self,
        event: &rift_runtime::Event,
        rest: Arc<RestClient<ReqwestTransport>>,
        cache: Option<Arc<RwLock<rift_cache::Cache>>>,
    ) -> Result<bool, FrameworkError> {
        if event.name != "INTERACTION_CREATE" {
            return Ok(false);
        }
        let interaction: Interaction = serde_json::from_str(event.data.get())?;
        let ctx = EventContext::new(interaction, rest, cache);
        self.dispatch(&ctx).await
    }

    /// Dispatches a raw gateway event to the router if it is an `INTERACTION_CREATE`.
    #[cfg(not(feature = "cache"))]
    pub async fn dispatch_raw(
        &self,
        event: &rift_runtime::Event,
        rest: Arc<RestClient<ReqwestTransport>>,
    ) -> Result<bool, FrameworkError> {
        if event.name != "INTERACTION_CREATE" {
            return Ok(false);
        }
        let interaction: Interaction = serde_json::from_str(event.data.get())?;
        let ctx = EventContext::new(interaction, rest);
        self.dispatch(&ctx).await
    }
}

/// A wrapper around [`Client`] that owns an in-memory [`rift_cache::Cache`] and
/// automatically updates it as events arrive.
#[cfg(feature = "cache")]
pub struct CachedClient {
    client: Client,
    cache: Arc<RwLock<rift_cache::Cache>>,
}

#[cfg(feature = "cache")]
impl CachedClient {
    /// Wraps a client with the provided cache.
    pub fn new(client: Client, cache: rift_cache::Cache) -> Self {
        Self {
            client,
            cache: Arc::new(RwLock::new(cache)),
        }
    }

    /// Wraps a client with a cache created from `config`.
    pub fn with_config(client: Client, config: rift_cache::CacheConfig) -> Self {
        Self::new(client, rift_cache::Cache::new(config))
    }

    /// Reference to the shared cache.
    pub fn cache(&self) -> &Arc<RwLock<rift_cache::Cache>> {
        &self.cache
    }

    /// Reference to the underlying REST client.
    pub fn rest(&self) -> &Arc<RestClient<ReqwestTransport>> {
        self.client.rest()
    }

    /// Reference to the inner [`Client`].
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Mutable reference to the inner [`Client`].
    pub fn client_mut(&mut self) -> &mut Client {
        &mut self.client
    }

    /// Unwraps the inner client and cache.
    pub fn into_inner(self) -> (Client, Arc<RwLock<rift_cache::Cache>>) {
        (self.client, self.cache)
    }

    /// Receives the next raw event from any shard, updating the cache if parseable.
    pub async fn next_event(&mut self) -> Option<rift_runtime::Event> {
        let ev = self.client.next_event().await?;
        if let Ok(model) = ev.parse_model() {
            if let Ok(mut guard) = self.cache.write() {
                guard.update(&model);
            }
        }
        Some(ev)
    }

    /// Receives the next event parsed into [`GatewayEvent`], updating the cache.
    pub async fn next_gateway_event(&mut self) -> Option<Result<GatewayEvent, serde_json::Error>> {
        let raw = self.client.next_event().await?;
        let parsed = raw.parse_model();
        if let Ok(ref model) = parsed {
            if let Ok(mut guard) = self.cache.write() {
                guard.update(model);
            }
        }
        Some(parsed)
    }

    /// Receives the next typed event, skipping any parse failures, updating the cache.
    pub async fn next_typed_event(&mut self) -> Option<GatewayEvent> {
        while let Some(res) = self.next_gateway_event().await {
            if let Ok(ev) = res {
                return Some(ev);
            }
        }
        None
    }
}

/// Integrated bot framework coordinating a [`Client`], an optional [`rift_cache::Cache`],
/// and a [`CommandRouter`].
pub struct Framework {
    client: Client,
    router: CommandRouter,
    #[cfg(feature = "cache")]
    cache: Option<Arc<RwLock<rift_cache::Cache>>>,
}

impl Framework {
    /// Create a new framework with the given client and command router.
    pub fn new(client: Client, router: CommandRouter) -> Self {
        #[cfg(feature = "cache")]
        let cache = client.cache().cloned();
        Self {
            client,
            router,
            #[cfg(feature = "cache")]
            cache,
        }
    }

    /// Attach a state cache to the framework.
    #[cfg(feature = "cache")]
    pub fn with_cache(mut self, cache: rift_cache::Cache) -> Self {
        self.cache = Some(Arc::new(RwLock::new(cache)));
        self
    }

    /// Attach a state cache created with the provided configuration.
    #[cfg(feature = "cache")]
    pub fn with_cache_config(self, config: rift_cache::CacheConfig) -> Self {
        self.with_cache(rift_cache::Cache::new(config))
    }

    /// Access the inner [`Client`].
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Access the inner [`Client`] mutably.
    pub fn client_mut(&mut self) -> &mut Client {
        &mut self.client
    }

    /// Access the command router.
    pub fn router(&self) -> &CommandRouter {
        &self.router
    }

    /// Access the command router mutably.
    pub fn router_mut(&mut self) -> &mut CommandRouter {
        &mut self.router
    }

    /// Access the optional cache.
    #[cfg(feature = "cache")]
    pub fn cache(&self) -> Option<&Arc<RwLock<rift_cache::Cache>>> {
        self.cache.as_ref()
    }

    /// Access the shared REST client.
    pub fn rest(&self) -> &Arc<RestClient<ReqwestTransport>> {
        self.client.rest()
    }

    /// Bulk register all commands configured on the router globally.
    pub async fn register_global_commands(
        &self,
        application_id: Snowflake,
    ) -> Result<Vec<rift_rest::RegisteredCommand>, rift_rest::Error> {
        self.client
            .rest()
            .bulk_overwrite_global_commands(application_id, self.router.commands())
            .await
    }

    /// Bulk register all commands configured on the router in a specific guild.
    pub async fn register_guild_commands(
        &self,
        application_id: Snowflake,
        guild_id: Snowflake,
    ) -> Result<Vec<rift_rest::RegisteredCommand>, rift_rest::Error> {
        self.client
            .rest()
            .bulk_overwrite_guild_commands(application_id, guild_id, self.router.commands())
            .await
    }

    /// Dispatches an interaction through the framework router.
    pub async fn dispatch_interaction(
        &self,
        interaction: &Interaction,
    ) -> Result<bool, FrameworkError> {
        #[cfg(feature = "cache")]
        let ctx = EventContext::new(
            interaction.clone(),
            self.client.rest().clone(),
            self.cache.clone(),
        );
        #[cfg(not(feature = "cache"))]
        let ctx = EventContext::new(
            interaction.clone(),
            self.client.rest().clone(),
        );
        self.router.dispatch(&ctx).await
    }

    /// Advance one event step: receives next event, updates cache, and automatically
    /// dispatches `INTERACTION_CREATE` events to the router.
    ///
    /// Returns `Some(Ok(event))` for each processed event, or `None` once all shards stop.
    pub async fn step(&mut self) -> Option<Result<GatewayEvent, FrameworkError>> {
        let raw = self.client.next_event().await?;
        let parsed = match raw.parse_model() {
            Ok(p) => p,
            Err(e) => return Some(Err(FrameworkError::Parse(e))),
        };

        #[cfg(feature = "cache")]
        if let Some(ref cache) = self.cache {
            if let Ok(mut guard) = cache.write() {
                guard.update(&parsed);
            }
        }

        if raw.name == "INTERACTION_CREATE" {
            let interaction: Interaction = match serde_json::from_str(raw.data.get()) {
                Ok(i) => i,
                Err(e) => return Some(Err(FrameworkError::Parse(e))),
            };
            if let Err(e) = self.dispatch_interaction(&interaction).await {
                return Some(Err(e));
            }
        }

        Some(Ok(parsed))
    }

    /// Run the event loop continuously until all shards shut down.
    pub async fn run(&mut self) -> Result<(), FrameworkError> {
        while let Some(res) = self.step().await {
            res?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rift_model::events::Ready;
    use serde_json::json;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

    fn make_ctx(kind: InteractionType, data: serde_json::Value) -> EventContext {
        let type_num = match kind {
            InteractionType::Ping => 1,
            InteractionType::ApplicationCommand => 2,
            InteractionType::MessageComponent => 3,
            InteractionType::ApplicationCommandAutocomplete => 4,
            InteractionType::ModalSubmit => 5,
            InteractionType::Unknown(u) => u,
        };
        let raw = serde_json::value::RawValue::from_string(data.to_string()).unwrap();
        let json_blob = json!({
            "id": "1000",
            "application_id": "2000",
            "type": type_num,
            "token": "secret_token_123",
            "version": 1,
            "data": serde_json::from_str::<serde_json::Value>(raw.get()).unwrap()
        });
        let interaction: Interaction = serde_json::from_value(json_blob).unwrap();
        let rest = Arc::new(RestClient::new("tok", "https://example.com", "1.0").unwrap());
        #[cfg(feature = "cache")]
        {
            EventContext::new(interaction, rest, None)
        }
        #[cfg(not(feature = "cache"))]
        {
            EventContext::new(interaction, rest)
        }
    }

    #[test]
    fn event_model_ext_parses_runtime_event() {
        let raw = serde_json::value::RawValue::from_string(
            json!({
                "v": 10,
                "user": {
                    "id": "123",
                    "username": "bot",
                    "discriminator": "0001",
                    "avatar": null,
                    "bot": true
                },
                "guilds": [],
                "session_id": "s1",
                "resume_gateway_url": "wss://gateway.discord.gg",
                "application": {
                    "id": "2000"
                }
            })
            .to_string(),
        )
        .unwrap();

        let event = rift_runtime::Event {
            shard_id: 0,
            name: "READY".to_string(),
            seq: 1,
            data: raw,
        };

        let model = event.parse_model().unwrap();
        match model {
            GatewayEvent::Ready(Ready { session_id, .. }) => {
                assert_eq!(session_id, "s1");
            }
            _ => panic!("expected Ready"),
        }
    }

    #[tokio::test]
    async fn slash_command_dispatches_cleanly() {
        let executed = Arc::new(AtomicBool::new(false));
        let executed_clone = executed.clone();

        let router = CommandRouter::new().slash("ping", "A ping command", move |ctx| {
            let executed = executed_clone.clone();
            async move {
                assert_eq!(ctx.id(), Snowflake::new(1000));
                assert_eq!(ctx.token(), "secret_token_123");
                let data = ctx.command_data().unwrap();
                assert_eq!(data.name, "ping");
                executed.store(true, Ordering::SeqCst);
                Ok(())
            }
        });

        let ctx = make_ctx(
            InteractionType::ApplicationCommand,
            json!({
                "id": "500",
                "name": "ping",
                "type": 1,
                "options": []
            }),
        );

        let handled = router.dispatch(&ctx).await.unwrap();
        assert!(handled);
        assert!(executed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn user_and_message_commands_dispatch() {
        let user_cmd_called = Arc::new(AtomicBool::new(false));
        let user_cmd_called_c = user_cmd_called.clone();

        let msg_cmd_called = Arc::new(AtomicBool::new(false));
        let msg_cmd_called_c = msg_cmd_called.clone();

        let router = CommandRouter::new()
            .user_command("View User", move |_ctx| {
                let called = user_cmd_called_c.clone();
                async move {
                    called.store(true, Ordering::SeqCst);
                    Ok(())
                }
            })
            .message_command("Pin Message", move |_ctx| {
                let called = msg_cmd_called_c.clone();
                async move {
                    called.store(true, Ordering::SeqCst);
                    Ok(())
                }
            });

        let ctx_user = make_ctx(
            InteractionType::ApplicationCommand,
            json!({
                "id": "501",
                "name": "View User",
                "type": 2,
                "target_id": "777"
            }),
        );
        assert!(router.dispatch(&ctx_user).await.unwrap());
        assert!(user_cmd_called.load(Ordering::SeqCst));

        let ctx_msg = make_ctx(
            InteractionType::ApplicationCommand,
            json!({
                "id": "502",
                "name": "Pin Message",
                "type": 3,
                "target_id": "888"
            }),
        );
        assert!(router.dispatch(&ctx_msg).await.unwrap());
        assert!(msg_cmd_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn component_exact_and_prefix_routing() {
        let exact_count = Arc::new(AtomicU32::new(0));
        let prefix_count = Arc::new(AtomicU32::new(0));

        let exact_c = exact_count.clone();
        let prefix_c = prefix_count.clone();

        let router = CommandRouter::new()
            .component_exact("btn:exact", move |_ctx| {
                let c = exact_c.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            })
            .component_prefix("btn:", move |ctx| {
                let c = prefix_c.clone();
                async move {
                    assert_eq!(ctx.stripped_custom_id("btn:").unwrap(), "dynamic_123");
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            });

        // Exact match takes precedence
        let ctx_exact = make_ctx(
            InteractionType::MessageComponent,
            json!({
                "custom_id": "btn:exact",
                "component_type": 2
            }),
        );
        assert!(router.dispatch(&ctx_exact).await.unwrap());
        assert_eq!(exact_count.load(Ordering::SeqCst), 1);
        assert_eq!(prefix_count.load(Ordering::SeqCst), 0);

        // Prefix match handles dynamic custom_ids
        let ctx_prefix = make_ctx(
            InteractionType::MessageComponent,
            json!({
                "custom_id": "btn:dynamic_123",
                "component_type": 2
            }),
        );
        assert!(router.dispatch(&ctx_prefix).await.unwrap());
        assert_eq!(exact_count.load(Ordering::SeqCst), 1);
        assert_eq!(prefix_count.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn modal_exact_and_prefix_routing() {
        let called = Arc::new(AtomicBool::new(false));
        let called_c = called.clone();

        let router = CommandRouter::new().modal_prefix("feedback:", move |ctx| {
            let called = called_c.clone();
            async move {
                let val = ctx.modal_value("feedback_input").unwrap();
                assert_eq!(val, "Rift is amazingly fast!");
                called.store(true, Ordering::SeqCst);
                Ok(())
            }
        });

        let ctx = make_ctx(
            InteractionType::ModalSubmit,
            json!({
                "custom_id": "feedback:form_1",
                "components": [
                    {
                        "type": 1,
                        "components": [
                            {
                                "type": 4,
                                "custom_id": "feedback_input",
                                "value": "Rift is amazingly fast!"
                            }
                        ]
                    }
                ]
            }),
        );

        assert!(router.dispatch(&ctx).await.unwrap());
        assert!(called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn autocomplete_routing() {
        let called_opt = Arc::new(AtomicBool::new(false));
        let called_opt_c = called_opt.clone();

        let router = CommandRouter::new().autocomplete("search", "query", move |ctx| {
            let called = called_opt_c.clone();
            async move {
                let opt = ctx.autocomplete_option().unwrap();
                assert_eq!(opt.name, "query");
                assert_eq!(opt.as_str().unwrap(), "rust");
                called.store(true, Ordering::SeqCst);
                Ok(())
            }
        });

        let ctx = make_ctx(
            InteractionType::ApplicationCommandAutocomplete,
            json!({
                "id": "999",
                "name": "search",
                "type": 1,
                "options": [
                    {
                        "name": "query",
                        "type": 3,
                        "value": "rust",
                        "focused": true
                    }
                ]
            }),
        );

        assert!(router.dispatch(&ctx).await.unwrap());
        assert!(called_opt.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn unhandled_interaction_returns_false() {
        let router = CommandRouter::new();
        let ctx = make_ctx(
            InteractionType::MessageComponent,
            json!({ "custom_id": "unknown_btn", "component_type": 2 }),
        );
        let handled = router.dispatch(&ctx).await.unwrap();
        assert!(!handled);
    }

    #[tokio::test]
    #[cfg(feature = "cache")]
    async fn cached_client_updates_cache() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(16);
        let (events_tx, events_rx) =
            rift_runtime::event_channel(rift_runtime::EventBufferPolicy::Unbounded);
        let (stop_tx, _stop_rx) = tokio::sync::mpsc::unbounded_channel();
        let supervisor = tokio::spawn(async { Ok(()) });

        let client = Client {
            rest: Arc::new(RestClient::new("token", "https://example.com", "1.0").unwrap()),
            senders: vec![cmd_tx],
            events: events_rx,
            stop: stop_tx,
            supervisor,
            num_shards: 1,
            #[cfg(feature = "cache")]
            cache: None,
        };

        let mut cached_client = CachedClient::new(client, rift_cache::Cache::default());

        let raw = serde_json::value::RawValue::from_string(
            json!({
                "v": 10,
                "user": {
                    "id": "9999",
                    "username": "CachedBot",
                    "discriminator": "0001",
                    "avatar": null,
                    "bot": true
                },
                "guilds": [],
                "session_id": "sess_xyz",
                "resume_gateway_url": "wss://gateway.discord.gg",
                "application": { "id": "2000" }
            })
            .to_string(),
        )
        .unwrap();

        events_tx
            .send(rift_runtime::Event {
                shard_id: 0,
                name: "READY".to_string(),
                seq: 1,
                data: raw,
            })
            .unwrap();

        let event = cached_client.next_typed_event().await.unwrap();
        assert!(matches!(event, GatewayEvent::Ready(_)));

        let guard = cached_client.cache().read().unwrap();
        let u = guard.current_user().unwrap();
        assert_eq!(u.id, Snowflake::new(9999));
        assert_eq!(u.username, "CachedBot");
    }

    #[test]
    #[cfg(feature = "cache")]
    fn event_context_cache_and_permission_math() {
        let cache = Arc::new(RwLock::new(rift_cache::Cache::default()));
        let mut ctx = make_ctx(InteractionType::MessageComponent, json!({ "custom_id": "btn" }));
        ctx.cache = Some(cache.clone());
        ctx.interaction.guild_id = Some(Snowflake::new(555));
        ctx.interaction.channel_id = Some(Snowflake::new(777));
        ctx.interaction.user = Some(User {
            id: Snowflake::new(888),
            username: "Alice".into(),
            bot: false,
        });

        // Channel does not exist in cache yet
        assert_eq!(ctx.compute_permissions(), None);
        assert_eq!(ctx.cached_guild(), None);
        assert_eq!(ctx.cached_channel(), None);
    }

    #[tokio::test]
    async fn router_dispatch_raw_event() {
        let called = Arc::new(AtomicBool::new(false));
        let called_c = called.clone();

        let router = CommandRouter::new().slash("info", "Bot info", move |_ctx| {
            let c = called_c.clone();
            async move {
                c.store(true, Ordering::SeqCst);
                Ok(())
            }
        });

        let raw = serde_json::value::RawValue::from_string(
            json!({
                "id": "123",
                "application_id": "456",
                "type": 2,
                "token": "tok",
                "version": 1,
                "data": {
                    "id": "789",
                    "name": "info",
                    "type": 1
                }
            })
            .to_string(),
        )
        .unwrap();

        let event = rift_runtime::Event {
            shard_id: 0,
            name: "INTERACTION_CREATE".to_string(),
            seq: 10,
            data: raw,
        };

        let rest = Arc::new(RestClient::new("token", "https://example.com", "1.0").unwrap());
        #[cfg(feature = "cache")]
        let res = router.dispatch_raw(&event, rest, None).await.unwrap();
        #[cfg(not(feature = "cache"))]
        let res = router.dispatch_raw(&event, rest).await.unwrap();
        assert!(res);
        assert!(called.load(Ordering::SeqCst));
    }
}
