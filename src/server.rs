use std::sync::Arc;

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use clap::Parser;
use osc_cost::oapi::Input;
use std::sync::Mutex;

mod output;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about)]
struct Args {
    #[arg(long, short = 'l')]
    pub bind: Option<String>,
    #[arg(long, short = 'p')]
    pub profile: Option<String>,
    #[arg(long, short = 'a', default_value_t = false)]
    pub aggregate: bool,
}

#[derive(Clone)]
struct AppState {
    input: Arc<Mutex<Input>>,
    aggregate: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let args_profile = args.profile.clone();
    let input = tokio::task::spawn_blocking(move || {
        Input::new(args_profile).expect("Could not configure backend")
    })
    .await?;

    let state = AppState {
        input: Arc::new(Mutex::new(input)),
        aggregate: args.aggregate,
    };

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(healhcheck))
        .with_state(state);

    let listener =
        tokio::net::TcpListener::bind(args.bind.unwrap_or_else(|| "127.0.0.1:3000".to_string()))
            .await
            .unwrap();
    axum::serve(listener, app).await?;

    Ok(())
}

async fn root(State(state): State<AppState>) -> Result<Response, String> {
    let local_lock = state.input.clone();
    let mut resources = match tokio::task::spawn_blocking(move || {
        let mut inputs = local_lock
            .lock()
            .map_err(|e| format!("Could not lock mutex: {e}"))?;
        inputs
            .fetch()
            .map_err(|e| format!("Could not fetch inputs: {e}"))?;

        Ok(inputs.build_resources())
    })
    .await
    {
        Ok(Ok(res)) => res,
        Ok(Err(e)) => return Err(e),
        Err(e) => return Err(format!("Task join error: {e}")),
    };

    resources.compute().map_err(|e| e.to_string())?;

    if state.aggregate {
        resources = resources.aggregate();
    }

    Ok((
        [(
            http::header::CONTENT_TYPE,
            http::HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
        )],
        output::prometheus::prometheus(&resources).unwrap(),
    )
        .into_response())
}

async fn healhcheck(State(state): State<AppState>) -> Result<(), String> {
    match state.input.is_poisoned() {
        true => Err("Input mutex is poisoned".to_string()),
        false => Ok(()),
    }
}
