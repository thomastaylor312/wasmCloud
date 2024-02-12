use std::collections::HashMap;

use anyhow::Result;
use clap::Parser;

use crate::{
    actor::{scale_actor, ActorScaledInfo, ScaleActorArgs},
    cli::{labels_vec_to_hashmap, CliConnectionOpts, CommandOutput},
    common::find_host_id,
    config::{WashConnectionOptions, DEFAULT_NATS_TIMEOUT_MS, DEFAULT_SCALE_ACTOR_TIMEOUT_MS},
    context::default_timeout_ms,
};

#[derive(Debug, Clone, Parser)]
pub enum ScaleCommand {
    /// Scale an actor running in a host to a certain level of concurrency
    #[clap(name = "actor")]
    Actor(ScaleActorCommand),
}

#[derive(Debug, Clone, Parser)]
pub struct ScaleActorCommand {
    #[clap(flatten)]
    pub opts: CliConnectionOpts,

    /// ID of host to scale actor on. If a non-ID is provided, the host will be selected based on
    /// matching the friendly name and will return an error if more than one host matches.
    #[clap(name = "host-id")]
    pub host_id: String,

    /// Actor reference, e.g. the OCI URL for the actor.
    #[clap(name = "actor-ref")]
    pub actor_ref: String,

    /// Maximum number of instances this actor can run concurrently.
    #[clap(
        long = "max-instances",
        alias = "max-concurrent",
        alias = "max",
        alias = "count",
        default_value_t = u32::MAX
    )]
    pub max_instances: u32,

    /// Constraints for actor auction in the form of "label=value". If host-id is supplied, this list is ignored
    #[clap(short = 'c', long = "constraint", name = "constraints")]
    pub constraints: Option<Vec<String>>,

    /// Optional set of annotations used to describe the nature of this actor scale command.
    /// For example, autonomous agents may wish to “tag” scale requests as part of a given deployment
    #[clap(short = 'a', long = "annotations")]
    pub annotations: Option<Vec<String>>,

    /// Timeout to await an auction response, defaults to 2000 milliseconds
    #[clap(long = "auction-timeout-ms", default_value_t = default_timeout_ms())]
    pub auction_timeout_ms: u64,

    /// By default, the command will wait until the actor has been started.
    /// If this flag is passed, the command will return immediately after acknowledgement from the host, without waiting for the actor to start.
    /// If this flag is omitted, the timeout will be adjusted to 5 seconds to account for actor download times
    #[clap(long = "skip-wait")]
    pub skip_wait: bool,
}

pub async fn handle_scale_actor(cmd: ScaleActorCommand) -> Result<CommandOutput> {
    // If timeout isn't supplied, override with a longer timeout for starting actor
    let timeout_ms = if cmd.opts.timeout_ms == DEFAULT_NATS_TIMEOUT_MS {
        DEFAULT_SCALE_ACTOR_TIMEOUT_MS
    } else {
        cmd.opts.timeout_ms
    };
    let client = <CliConnectionOpts as TryInto<WashConnectionOptions>>::try_into(cmd.opts)?
        .into_ctl_client(Some(cmd.auction_timeout_ms))
        .await?;

    let actor_ref = if cmd.actor_ref.starts_with('/') {
        format!("file://{}", &cmd.actor_ref) // prefix with file:// if it's an absolute path
    } else {
        cmd.actor_ref.to_string()
    };

    let host = find_host_id(&cmd.host_id, &client).await?.0;

    let annotations = if let Some(annotations) = cmd.annotations {
        Some(labels_vec_to_hashmap(annotations)?)
    } else {
        None
    };

    // Start the actor
    let ActorScaledInfo {
        host_id,
        actor_ref,
        actor_id,
    } = scale_actor(ScaleActorArgs {
        ctl_client: &client,
        host_id: &host,
        actor_ref: &actor_ref,
        count: cmd.max_instances,
        skip_wait: cmd.skip_wait,
        timeout_ms: Some(timeout_ms),
        annotations,
    })
    .await?;

    let text = if cmd.skip_wait {
        format!(
            "Request to scale actor {actor_ref} to {} max concurrent instances on {host_id} received",
            cmd.max_instances
        )
    } else {
        format!(
            "Actor [{}] (ref: [{actor_ref}]) scaled to {} max concurrent instances on host [{host_id}]",
            actor_id.clone().unwrap_or("<unknown>".into()),
            cmd.max_instances
        )
    };

    Ok(CommandOutput::new(
        text.clone(),
        HashMap::from([
            ("result".into(), text.into()),
            ("actor_ref".into(), actor_ref.into()),
            ("actor_id".into(), actor_id.into()),
            ("host_id".into(), host_id.into()),
        ]),
    ))
}
