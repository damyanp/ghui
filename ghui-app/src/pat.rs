use anyhow::Result;
use github_graphql::client::transport::GithubClient;

pub struct PATState {
    pat_entry: keyring::Entry,
}

impl Default for PATState {
    fn default() -> Self {
        let pat_entry = keyring::Entry::new("ghui", "PAT").expect("keyring failed to get entry");
        Self { pat_entry }
    }
}

impl PATState {
    pub fn get_password(&self) -> keyring::Result<String> {
        self.pat_entry.get_password()
    }

    pub fn set_password(&self, password: &str) -> keyring::Result<()> {
        assert!(!password.is_empty());
        self.pat_entry.set_password(password)
    }

    pub fn delete_password(&self) -> keyring::Result<()> {
        self.pat_entry.delete_credential()
    }

    pub fn new_github_client(&self) -> Result<GithubClient> {
        let password = self.get_password()?;
        Ok(GithubClient::new(&password)?)
    }
}
