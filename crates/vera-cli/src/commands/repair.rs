//! `vera repair` — repair the configured backend by re-fetching missing assets
//! or re-persisting API configuration from the current environment.

use vera_core::config::InferenceBackend;

use crate::commands::setup;
use crate::state;

pub fn run(backend: Option<InferenceBackend>, api: bool, json_output: bool) -> anyhow::Result<()> {
    let effective_backend = if api {
        InferenceBackend::Api
    } else if let Some(backend) = backend {
        backend
    } else if let Some(saved_backend) = state::saved_backend()? {
        saved_backend
    } else {
        vera_core::config::resolve_backend(None)
    };

    setup::configure_backend(
        effective_backend,
        None,
        json_output,
        "Vera repair complete.",
    )
}
