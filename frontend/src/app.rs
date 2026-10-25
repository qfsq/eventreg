use gloo_net::http::Request;
use leptos::*;
use serde::{Deserialize, Serialize};

/// Development account seeded by `seeds/dev.sql`.
const DEFAULT_USER_ID: &str = "a0eebc99-9c0b-4ef8-bb6d-6bb9bd380a11";

/// Override at build time, e.g.
/// `EVENT_REGISTER_API_BASE=http://127.0.0.1:8080/api/v1 trunk serve`.
/// The default is same-origin so nginx can reverse proxy `/api`.
fn api_base() -> &'static str {
    option_env!("EVENT_REGISTER_API_BASE").unwrap_or("/api/v1")
}

/// The demo user can be overridden from the browser console:
/// `localStorage.setItem("event_register.user_id", "<uuid>")`.
fn current_user_id() -> String {
    web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item("event_register.user_id").ok().flatten())
        .filter(|user_id| !user_id.is_empty())
        .unwrap_or_else(|| DEFAULT_USER_ID.to_owned())
}

const CATEGORIES: [(&str, &str); 6] = [
    ("chore", "Chore"),
    ("work", "Work"),
    ("emotion", "Emotion"),
    ("thinking", "Thinking"),
    ("writing", "Writing"),
    ("inventing", "Inventing"),
];

fn category_label(value: &str) -> &'static str {
    CATEGORIES
        .iter()
        .find(|(category, _)| *category == value)
        .map(|(_, label)| *label)
        .unwrap_or("Unknown")
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Summary {
    pub category: String,
    pub total_count: i64,
    pub last_logged_at: Option<String>,
    pub streak_days: i32,
}

#[derive(Debug, Serialize)]
pub struct CreateEventPayload {
    pub user_id: String,
    pub category: String,
    pub title: String,
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    error: String,
}

async fn response_error(response: gloo_net::http::Response) -> String {
    let status = response.status();
    match response.json::<ApiErrorBody>().await {
        Ok(body) => body.error,
        Err(_) => format!("Request failed with HTTP {status}"),
    }
}

#[component]
pub fn App() -> impl IntoView {
    let user_id = current_user_id();

    let (category, set_category) = create_signal(CATEGORIES[1].0.to_owned());
    let (title, set_title) = create_signal(String::new());
    let (content, set_content) = create_signal(String::new());
    let (summaries, set_summaries) = create_signal(Vec::<Summary>::new());
    let (loading, set_loading) = create_signal(true);
    let (submitting, set_submitting) = create_signal(false);
    let (error, set_error) = create_signal(Option::<String>::None);

    let fetch_summaries = {
        let user_id = user_id.clone();
        move || {
            let user_id = user_id.clone();
            set_loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                let url = format!("{}/summaries/{}", api_base(), user_id);
                match Request::get(&url).send().await {
                    Ok(response) if response.ok() => {
                        match response.json::<Vec<Summary>>().await {
                            Ok(data) => {
                                set_summaries.set(data);
                                set_error.set(None);
                            }
                            Err(cause) => {
                                set_error.set(Some(format!("Invalid server response: {cause}")));
                            }
                        }
                    }
                    Ok(response) => set_error.set(Some(response_error(response).await)),
                    Err(cause) => set_error.set(Some(format!("Network error: {cause}"))),
                }
                set_loading.set(false);
            });
        }
    };

    create_effect({
        let fetch_summaries = fetch_summaries.clone();
        move |_| fetch_summaries()
    });

    let submit_event = {
        let user_id = user_id.clone();
        let fetch_summaries = fetch_summaries.clone();

        move |event: web_sys::SubmitEvent| {
            event.prevent_default();

            if submitting.get() {
                return;
            }

            let title_value = title.get();
            if title_value.trim().is_empty() {
                set_error.set(Some("Title is required.".to_owned()));
                return;
            }

            let content_value = content.get();
            let payload = CreateEventPayload {
                user_id: user_id.clone(),
                category: category.get(),
                title: title_value,
                content: (!content_value.trim().is_empty()).then_some(content_value),
            };

            let fetch_summaries = fetch_summaries.clone();
            set_submitting.set(true);
            set_error.set(None);

            wasm_bindgen_futures::spawn_local(async move {
                let request = match Request::post(&format!("{}/events", api_base())).json(&payload)
                {
                    Ok(request) => request,
                    Err(cause) => {
                        set_error.set(Some(format!("Could not encode request: {cause}")));
                        set_submitting.set(false);
                        return;
                    }
                };

                match request.send().await {
                    Ok(response) if response.ok() => {
                        set_title.set(String::new());
                        set_content.set(String::new());
                        fetch_summaries();
                    }
                    Ok(response) => set_error.set(Some(response_error(response).await)),
                    Err(cause) => set_error.set(Some(format!("Network error: {cause}"))),
                }

                set_submitting.set(false);
            });
        }
    };

    view! {
        <main class="app">
            <header class="hero">
                <h1>"EventRegister"</h1>
                <p class="subtitle">"Log daily activities and keep your streaks alive."</p>
            </header>

            {move || error.get().map(|message| view! {
                <p class="banner error" role="alert">{message}</p>
            })}

            <form class="card event-form" on:submit=submit_event>
                <label>
                    <span>"Category"</span>
                    <select
                        on:change=move |event| set_category.set(event_target_value(&event))
                    >
                        {CATEGORIES
                            .into_iter()
                            .map(|(value, label)| view! {
                                <option value=value selected=move || category.get() == value>
                                    {label}
                                </option>
                            })
                            .collect_view()}
                    </select>
                </label>

                <label>
                    <span>"Title"</span>
                    <input
                        type="text"
                        maxlength="255"
                        required
                        placeholder="E.g. Finished module draft"
                        prop:value=title
                        on:input=move |event| set_title.set(event_target_value(&event))
                    />
                </label>

                <label>
                    <span>"Notes"</span>
                    <textarea
                        maxlength="20000"
                        placeholder="Optional details..."
                        prop:value=content
                        on:input=move |event| set_content.set(event_target_value(&event))
                    ></textarea>
                </label>

                <button type="submit" disabled=move || submitting.get()>
                    {move || if submitting.get() { "Saving..." } else { "Log event" }}
                </button>
            </form>

            <section>
                <div class="section-heading">
                    <h2>"Activity summaries"</h2>
                    <span class="muted">"UTC days"</span>
                </div>

                {move || loading.get().then(|| view! {
                    <p class="muted">"Loading summaries..."</p>
                })}

                <div class="summary-grid">
                    <For
                        each=move || summaries.get()
                        key=|summary| summary.category.clone()
                        children=move |summary| {
                            let label = category_label(&summary.category);
                            let last_logged = summary
                                .last_logged_at
                                .as_deref()
                                .and_then(|timestamp| timestamp.get(..10))
                                .map(str::to_owned);

                            view! {
                                <article class="card summary">
                                    <h3>{label}</h3>
                                    <p class="count">{summary.total_count}</p>
                                    <p class="meta">
                                        "Streak: " {summary.streak_days} " day(s)"
                                    </p>
                                    {last_logged.map(|date| view! {
                                        <p class="meta">"Last: " {date}</p>
                                    })}
                                </article>
                            }
                        }
                    />
                </div>
            </section>
        </main>
    }
}
