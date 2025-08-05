use axum::{
    Json,
    body::{self, Body},
    response::{IntoResponse, Response, Sse},
};
use eventsource_stream::Eventsource;
use futures::TryStreamExt;

use crate::{
    middleware::claude::ClaudeContext,
    types::claude_message::{CreateMessageResponse, StreamEvent},
};

use super::{ClaudeApiFormat, transform_stream};

/// Transforms responses to ensure compatibility with the OpenAI API format
///
/// This middleware function analyzes responses and transforms them when necessary
/// to ensure compatibility between Claude and OpenAI API formats, particularly
/// for streaming responses. If the response is:
///
/// - From the Claude API format: No transformation needed
/// - Not streaming: No transformation needed
/// - Has a non-200 status code: No transformation needed
/// - OpenAI format and streaming: Transforms the stream to match OpenAI event format
///
/// # Arguments
///
/// * `resp` - The original response to be potentially transformed
///
/// # Returns
///
/// The original or transformed response as appropriate
pub async fn to_oai(resp: Response) -> impl IntoResponse {
    let Some(cx) = resp.extensions().get::<ClaudeContext>() else {
        return resp;
    };
    if ClaudeApiFormat::Claude == cx.api_format() || !cx.is_stream() || resp.status() != 200 {
        return resp;
    }
    let body = resp.into_body();
    let stream = body.into_data_stream().eventsource();
    let stream = transform_stream(stream);
    Sse::new(stream)
        .keep_alive(Default::default())
        .into_response()
}

pub async fn add_usage_info(resp: Response) -> impl IntoResponse {
    let Some(cx) = resp.extensions().get::<ClaudeContext>() else {
        return resp;
    };
    let (mut usage, stream) = (cx.usage().to_owned(), cx.is_stream());
    if !stream {
        let data = body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap_or_default();
        let Ok(mut response) = serde_json::from_slice::<CreateMessageResponse>(&data) else {
            return Body::from(data).into_response();
        };
        let output_tokens = response.count_tokens();
        usage.output_tokens = output_tokens;
        response.usage = Some(usage);
        return Json(response).into_response();
    }
    let stream = resp
        .into_body()
        .into_data_stream()
        .eventsource()
        .map_ok(move |event| {
            let new_event = axum::response::sse::Event::default()
                .event(event.event)
                .id(event.id);
            let new_event = if let Some(retry) = event.retry {
                new_event.retry(retry)
            } else {
                new_event
            };
            let Ok(parsed) = serde_json::from_str::<StreamEvent>(&event.data) else {
                return new_event.data(event.data);
            };
            match parsed {
                StreamEvent::MessageStart { mut message } => {
                    message.usage.get_or_insert(usage.to_owned());
                    new_event
                        .json_data(StreamEvent::MessageStart { message })
                        .unwrap()
                }
                _ => new_event.data(event.data),
            }
        });

    Sse::new(stream)
        .keep_alive(Default::default())
        .into_response()
}
