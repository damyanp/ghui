<p align="center">
  <img src="app/static/icon.svg" width="128" height="128" alt="ghui icon">
</p>

<h1 align="center">ghui</h1>

[![CI](https://github.com/damyanp/ghui/actions/workflows/rust.yml/badge.svg)](https://github.com/damyanp/ghui/actions/workflows/rust.yml)
[![Build Windows Installer](https://github.com/damyanp/ghui/actions/workflows/build-installer.yml/badge.svg)](https://github.com/damyanp/ghui/actions/workflows/build-installer.yml)

## GitHub authentication

ghui uses accounts stored by the [GitHub CLI](https://cli.github.com/). Sign in
with `gh auth login`, then select the `github.com` account from ghui's toolbar.
The selection is stored by ghui and does not change the account globally active
in `gh`.

GitHub-backed caches are isolated per selected account. If that account's stored
token is removed, cached data remains viewable, but network actions stay blocked
until an account is explicitly selected again.
