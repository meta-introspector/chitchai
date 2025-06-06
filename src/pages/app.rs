use dioxus::prelude::*;
use futures_util::StreamExt;
use transprompt::async_openai_wasm::Client;
use transprompt::async_openai_wasm::config::{AzureConfig, OpenAIConfig};
use uuid::Uuid;

use crate::components::{ChatContainer, LeftSidebar, SettingSidebar};
use crate::utils::auth::Auth;
use crate::utils::storage::StoredStates;



// Global states
pub type AuthedClient = Option<Client>;

pub struct ChatId(pub Uuid);

pub struct StreamingReply(pub bool);

pub fn Main(cx: Scope) -> Element {
    let mut stored_states = StoredStates::get_or_init();
    stored_states.run_count += 1;
    stored_states.save();
    log::info!("This is your {} time running ChitChai!", stored_states.run_count);
    render! {
        App {
            stored_states: stored_states
        }
    }
}


#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvents {
    ToggleSettingsSidebar,
}

#[derive(Debug, Clone, Props, PartialEq)]
pub struct AppProps {
    pub stored_states: StoredStates,
}

pub fn App(cx: Scope<AppProps>) -> Element {
    let stored_states = cx.props.stored_states.clone();
    let last_chat_id = stored_states.chats.last().unwrap().id;
    let authed_client: AuthedClient = stored_states
        .auth
        .as_ref()
        .map(|auth| {
            match auth {
                Auth::OpenAI { .. } => Client::with_config::<OpenAIConfig>(auth.clone().into()),
                Auth::AzureOpenAI { .. } => Client::with_config::<AzureConfig>(auth.clone().into()),
                _ => unreachable!(),
            }
        });
    let hide_settings_sidebar = stored_states.auth.is_some() && stored_states.selected_service.is_some();
    // configure share states
    use_shared_state_provider(cx, || stored_states);
    use_shared_state_provider(cx, || authed_client);
    use_shared_state_provider(cx, || ChatId(last_chat_id));
    use_shared_state_provider(cx, || StreamingReply(false));
    let global = use_shared_state::<StoredStates>(cx).unwrap();
    let chat_id = use_shared_state::<ChatId>(cx).unwrap();
    // configure local states
    let hide_setting_sidebar = use_state(cx, || hide_settings_sidebar);
    // configure event handler
    use_coroutine(cx, |mut rx| {
        let hide_setting_sidebar = hide_setting_sidebar.to_owned();
        async move {
            while let Some(event) = rx.next().await {
                match event {
                    AppEvents::ToggleSettingsSidebar => {
                        hide_setting_sidebar.modify(|h| !(*h));
                    }
                    _ => log::warn!("Unknown event: {:?}", event),
                }
            }
        }
    });
    render! {
        div {
            class: "flex h-full w-full",
            LeftSidebar {}
            div {
                class: "flex-grow overflow-auto",
                ChatContainer {}
            }
            div {
                class: "w-1/6",
                hidden: *hide_setting_sidebar.get(),
                SettingSidebar  {}
            }
        }
    }
}
