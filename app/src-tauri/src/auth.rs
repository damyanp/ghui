use crate::TauriCommandResult;
use ghui_app::{
    github_account::{
        enumerate_accounts, resolve_token, AccountList, AccountListState, AccountState,
        GhAuthError, GitHubIdentity, SelectAccountResult, SelectedAccountState,
    },
    DataState,
};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn get_account_state(
    data_state: State<'_, DataState>,
) -> TauriCommandResult<AccountState> {
    let identity = data_state.lock().await.selected_identity().cloned();
    let Some(identity) = identity else {
        return Ok(data_state.account_state().await);
    };

    if data_state.lock().await.has_client() {
        return Ok(data_state.account_state().await);
    }

    let token_result = resolve_token(&identity).await;
    let selected_state = match token_result {
        Ok(token) => {
            data_state.restore_client(&identity, token).await;
            SelectedAccountState::Unverified
        }
        Err(GhAuthError::GhMissing) => SelectedAccountState::GhMissing,
        Err(GhAuthError::Timeout) => SelectedAccountState::Timeout,
        Err(GhAuthError::Command(_)) => SelectedAccountState::TokenMissing,
    };

    let mut state = data_state.account_state().await;
    if let Some(selected) = &mut state.selected {
        selected.state = selected_state;
    }
    Ok(state)
}

#[tauri::command]
pub async fn list_accounts(data_state: State<'_, DataState>) -> TauriCommandResult<AccountList> {
    let selected = data_state.lock().await.selected_identity().cloned();
    let result = match enumerate_accounts(selected.as_ref()).await {
        Ok(accounts) => accounts,
        Err(error) => account_list_for_error(error),
    };
    Ok(result)
}

fn account_list_for_error(error: GhAuthError) -> AccountList {
    match error {
        GhAuthError::GhMissing => AccountList {
            state: AccountListState::GhMissing,
            accounts: Vec::new(),
            message: Some(
                "GitHub CLI was not found. Install gh and ensure it is available on PATH."
                    .to_owned(),
            ),
        },
        GhAuthError::Timeout => AccountList {
            state: AccountListState::Timeout,
            accounts: Vec::new(),
            message: Some("GitHub account enumeration timed out.".to_owned()),
        },
        GhAuthError::Command(message) => AccountList {
            state: AccountListState::Error,
            accounts: Vec::new(),
            message: Some(message),
        },
    }
}

#[tauri::command]
pub async fn select_account(
    app: AppHandle,
    data_state: State<'_, DataState>,
    identity: GitHubIdentity,
    confirmed_pending_edits: bool,
) -> TauriCommandResult<SelectAccountResult> {
    if !confirmed_pending_edits {
        if let Some(pending_edits) = data_state
            .pending_edits_for_account_change(&identity)
            .await?
        {
            return Ok(SelectAccountResult::ConfirmationRequired { pending_edits });
        }
    }
    let token = resolve_token(&identity)
        .await
        .map_err(anyhow::Error::from)?;
    let result = data_state
        .select_account(identity, token, confirmed_pending_edits)
        .await?;
    if let SelectAccountResult::Selected { state } = &result {
        app.emit("github-account-selected", &state.selected)
            .map_err(anyhow::Error::from)?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_list_timeout_is_typed() {
        let result = account_list_for_error(GhAuthError::Timeout);
        assert_eq!(result.state, AccountListState::Timeout);
        assert!(result.accounts.is_empty());
    }

    #[test]
    fn test_account_list_command_error_is_preserved() {
        let result = account_list_for_error(GhAuthError::Command("failed".to_owned()));
        assert_eq!(result.state, AccountListState::Error);
        assert_eq!(result.message.as_deref(), Some("failed"));
    }
}
