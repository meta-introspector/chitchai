use std::time::Duration;

use async_std::task::sleep;
use dioxus::prelude::*;

pub use message_card::*;

use crate::agents::AgentID;
use crate::chat::Chat;
use crate::components::chat::request_utils::{find_chat_idx_by_id, handle_request};
use crate::pages::app::{AuthedClient, ChatId, StreamingReply};
use crate::utils::storage::StoredStates;

pub mod message_card;
mod request_utils;

struct Request(String);

#[component]
pub fn ChatContainer() -> Element {
    let stored_states = use_context::<StoredStates>();
    let authed_client = use_context::<AuthedClient>();
    let streaming_reply = use_context::<StreamingReply>();
    let chat_id = use_context::<ChatId>();
    // request handler
    use_coroutine(|rx| handle_request(rx, chat_id, stored_states, authed_client, streaming_reply));
    // get data
    let chat_idx = find_chat_idx_by_id(&stored_states.chats, &chat_id.0);
    let chat: &Chat = &stored_states.chats[chat_idx];
    let user_agent_id: Vec<AgentID> = chat.user_agent_ids();
    assert_eq!(user_agent_id.len(), 1, "user_agents.len() == 1"); // TODO: support multiple user agents
    let user_agent = chat.agents.get(&user_agent_id[0]).unwrap();
    let history = &user_agent.history;
    rsx! {
        div { class: "flex h-full w-full flex-col relative",
            div { class: "flex flex-col h-full space-y-6 bg-slate-200 text-sm leading-6 text-slate-900 shadow-sm dark:bg-slate-900 dark:text-slate-300 sm:text-base sm:leading-7",
                div { class: "overflow-auto max-h-[90vh] flex-grow dark:scrollbar dark:scrollbar-thumb-slate-700 dark:scrollbar-track-slate-900",
                    {
                        history
                            .iter()
                            .map(|msg_id| {
                                let msg = chat.message_manager.get(msg_id).unwrap();
                                rsx! {
                                    MessageCard { chat_msg: msg.clone() }
                                }
                            })
                    }
                }
                ChatMessageInput { disable_submit: streaming_reply.0 }
            }
        }
    }
}

#[component]
pub fn ChatMessageInput(disable_submit: bool) -> Element {
    const TEXTAREA_ID: &str = "chat-input";
    let customization = &use_context::<StoredStates>().customization;
    let tick = use_signal(|| 0_usize);
    let mut tick_clone = tick.to_owned();
    // configure timer
    use_coroutine(move |_: UnboundedReceiver<()>| async move {
        loop {
            sleep(Duration::from_millis(500)).await;
            let mut tick = tick_clone.write();
            *tick = tick.wrapping_add(1);
        }
    });
    let request_sender: Coroutine<Request> = use_coroutine_handle();
    let mut input_value = use_signal(String::new);
    // TODO: try not to use js to clear textarea
    let mut clear_textarea = use_future(|| {
        let clear_js = format!("document.getElementById('{}').value = '';", TEXTAREA_ID);
        async move {
            let result = document::eval(clear_js.as_str()).join::<()>().await;
            match result {
                Ok(_) => log::info!("clear_textarea"),
                Err(e) => log::error!("clear_textarea error: {:?}", e),
            }
        }
    });

    rsx! {
        form {
            class: "mt-2 absolute bottom-0 w-full p-5",
            id: "chat-form",
            onsubmit: move |_| {
                let input_value = input_value.read();
                log::info!("onsubmit {}", & input_value);
                request_sender.send(Request(input_value.clone()));
                clear_textarea.restart();
            },
            label { r#for: "{TEXTAREA_ID}", class: "sr-only", "Enter your prompt" }
            div { class: "relative",
                textarea {
                    oninput: move |event| {
                        let value = event.data.value();
                        input_value.set(value)
                    },
                    id: "chat-input",
                    form: "chat-form",
                    class: "block w-full resize-none rounded-xl border-none bg-slate-200 p-4 pl-10 pr-20 text-sm text-slate-900 focus:outline-none focus:ring-2 focus:ring-blue-600 dark:bg-slate-900 dark:text-slate-200 dark:placeholder-slate-400 dark:focus:ring-blue-600 sm:text-base",
                    placeholder: "Enter your prompt",
                    rows: "2",
                    required: true,
                }
                button {
                    r#type: "submit",
                    disabled: disable_submit,
                    class: "absolute bottom-2 right-2.5 rounded-lg bg-blue-700 px-4 py-2 text-sm font-medium text-slate-200 hover:bg-blue-800 focus:outline-none focus:ring-4 focus:ring-blue-300 dark:bg-blue-600 dark:hover:bg-blue-700 dark:focus:ring-blue-800 sm:text-base",
                    {
                        if disable_submit {
                            customization
                                .waiting_icons[*tick.read() % customization.waiting_icons.len()]
                                .as_str()
                        } else {
                            "Send"
                        }
                    }
                    span { class: "sr-only", "Send message" }
                }
            }
        }
    }
}
