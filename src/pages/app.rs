use dioxus::prelude::*;
use futures_util::StreamExt;
use std::sync::Arc;
use transprompt::async_openai_wasm::config::{AzureConfig, Config, OpenAIConfig};
use transprompt::async_openai_wasm::Client;
use uuid::Uuid;

use crate::components::{ChatContainer, LeftSidebar, SettingSidebar};
use crate::utils::auth::Auth;
use crate::utils::storage::StoredStates;

// Global states
pub type AuthedClient = Option<Client<Arc<dyn Config>>>;

#[derive(Clone, Copy)]
pub struct ChatId(pub Uuid);

#[derive(Clone, Copy)]
pub struct StreamingReply(pub bool);

pub fn Main() -> Element {
    let mut stored_states = StoredStates::get_or_init();
    stored_states.run_count += 1;
    stored_states.save();
    log::info!(
        "This is your {} time running ChitChai!",
        stored_states.run_count
    );
    rsx! {
        App { stored_states }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvents {
    ToggleSettingsSidebar,
}

#[component]
pub fn App(stored_states: StoredStates) -> Element {
    let stored_states = stored_states.clone();
    let last_chat_id = stored_states.chats.last().unwrap().id;
    let authed_client: AuthedClient = stored_states.auth.as_ref().map(|auth| match auth {
        Auth::OpenAI { .. } => {
            let config: OpenAIConfig = auth.clone().into();
            let config = Arc::new(config) as Arc<dyn Config>;
            Client::with_config(config)
        }
        Auth::AzureOpenAI { .. } => {
            let config: AzureConfig = auth.clone().into();
            let config = Arc::new(config) as Arc<dyn Config>;
            Client::with_config(config)
        }
        _ => unreachable!(),
    });
    let hide_settings_sidebar =
        stored_states.auth.is_some() && stored_states.selected_service.is_some();
    // configure share states
    use_context_provider(|| stored_states);
    use_context_provider(|| authed_client);
    use_context_provider(|| ChatId(last_chat_id));
    use_context_provider(|| StreamingReply(false));
    let global = use_context::<StoredStates>(); // TODO: avoid implicit clone here
    let chat_id = use_context::<ChatId>();
    // configure local states
    let hide_setting_sidebar = use_signal(|| hide_settings_sidebar);
    // configure event handler
    let mut hide_setting_sidebar_cloned = hide_setting_sidebar.to_owned();
    use_coroutine(move |mut rx| async move {
        while let Some(event) = rx.next().await {
            match event {
                AppEvents::ToggleSettingsSidebar => {
                    let mut hide_setting_sidebar = hide_setting_sidebar_cloned.write();
                    *hide_setting_sidebar = !*hide_setting_sidebar;
                }
                _ => log::warn!("Unknown event: {:?}", event),
            }
        }
    });
    rsx! {
        div { class: "flex h-full w-full",
            LeftSidebar {}
            div { class: "flex-grow overflow-auto", ChatContainer {} }
            div { class: "w-1/6", hidden: *hide_setting_sidebar.read(), SettingSidebar {} }
        }
    }
}
