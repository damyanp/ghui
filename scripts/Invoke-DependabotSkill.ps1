<#
.SYNOPSIS
    Runs the `dependabot-pr-completion` Copilot skill against this repository.

.DESCRIPTION
    Invokes the GitHub Copilot CLI with a prompt that asks it to use the
    repository's `dependabot-pr-completion` skill (.github/skills/dependabot-pr-completion).

    The skill discovers open Dependabot PRs, validates each one locally with the
    ecosystem-appropriate checks (cargo / npm / github-actions), applies only
    minimal compatibility fixes, pushes them, and then merges or enables
    auto-merge.

    Because the skill checks out PR branches, the script refuses to run with a
    dirty working tree unless -Force is supplied.

.PARAMETER RepoRoot
    Repository to operate on. Defaults to the repository containing this script.

.PARAMETER Interactive
    Launch an interactive Copilot session pre-seeded with the prompt (`copilot -i`)
    instead of running headless. Useful when you want to watch and intervene.

.PARAMETER Model
    Optional model override passed to `copilot --model`.

.PARAMETER AdditionalInstructions
    Extra text appended to the prompt, e.g. "Only process the npm PRs."

.PARAMETER AllowAllPaths
    Use `--allow-all` (tools + paths + urls) instead of just `--allow-all-tools`.
    Needed if package managers must touch caches outside the repository.

.PARAMETER Share
    Write a markdown transcript of the session after completion
    (non-interactive runs only).

.PARAMETER Force
    Skip the clean-working-tree check.

.EXAMPLE
    .\scripts\Invoke-DependabotSkill.ps1

    Headless run: validate, fix, and merge every open Dependabot PR.

.EXAMPLE
    .\scripts\Invoke-DependabotSkill.ps1 -Interactive -Model claude-opus-5

    Watch the run in an interactive session using a specific model.

.EXAMPLE
    .\scripts\Invoke-DependabotSkill.ps1 -AdditionalInstructions 'Only process PR #57.' -WhatIf

    Show the exact copilot command line that would be executed.
#>
[CmdletBinding(SupportsShouldProcess)]
param(
    [ValidateNotNullOrEmpty()]
    [string]$RepoRoot = (Split-Path -Parent $PSScriptRoot),

    [switch]$Interactive,

    [string]$Model,

    [string]$AdditionalInstructions,

    [switch]$AllowAllPaths,

    [switch]$Share,

    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$skillName = 'dependabot-pr-completion'

function Assert-Command {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Hint
    )

    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "'$Name' was not found on PATH. $Hint"
    }
}

# --- Preflight -------------------------------------------------------------

if (-not (Test-Path -LiteralPath $RepoRoot -PathType Container)) {
    throw "RepoRoot '$RepoRoot' does not exist."
}
$RepoRoot = (Resolve-Path -LiteralPath $RepoRoot).ProviderPath

$skillPath = Join-Path $RepoRoot ".github\skills\$skillName\SKILL.md"
if (-not (Test-Path -LiteralPath $skillPath -PathType Leaf)) {
    throw "Skill '$skillName' not found at '$skillPath'. Run this against a checkout that contains the skill."
}

Assert-Command -Name 'copilot' -Hint 'Install it with: npm install -g @github/copilot'
Assert-Command -Name 'gh' -Hint 'Install the GitHub CLI from https://cli.github.com and run: gh auth login'
Assert-Command -Name 'git' -Hint 'Install Git for Windows from https://git-scm.com'

& gh auth status 2>&1 | Out-Null
if ($LASTEXITCODE -ne 0) {
    throw "'gh' is not authenticated. Run: gh auth login"
}

if (-not $Force) {
    $status = & git -C $RepoRoot status --porcelain
    if ($LASTEXITCODE -ne 0) {
        throw "'git status' failed in '$RepoRoot'."
    }
    if ($status) {
        throw "Working tree at '$RepoRoot' is dirty. The skill checks out PR branches, so commit/stash first or re-run with -Force."
    }
}

# --- Prompt ----------------------------------------------------------------

$promptLines = @(
    "Use the $skillName skill to complete all outstanding Dependabot PRs in this repository."
    'Validate each PR locally with the ecosystem-appropriate checks, apply only minimal compatibility fixes if needed,'
    'push those fixes, and merge or enable auto-merge when checks are still pending.'
    'If a package version is unavailable through the corporate npm proxy, roll that package back or report the PR as blocked - never work around the proxy.'
    'Finish with a summary listing merged PRs, PRs set to auto-merge, and PRs blocked with the reason for each.'
)

if ($AdditionalInstructions) {
    $promptLines += $AdditionalInstructions
}

$prompt = $promptLines -join "`n"

# --- Build the copilot command line ---------------------------------------

$copilotArgs = @('-C', $RepoRoot)

if ($Interactive) {
    $copilotArgs += @('-i', $prompt)
}
else {
    $copilotArgs += @('-p', $prompt, '--no-ask-user')
    if ($Share) {
        $copilotArgs += '--share'
    }
}

$copilotArgs += if ($AllowAllPaths) { '--allow-all' } else { '--allow-all-tools' }

if ($Model) {
    $copilotArgs += @('--model', $Model)
}

$displayArgs = ($copilotArgs | ForEach-Object {
        if ($_ -match '[\s"]') { '"{0}"' -f ($_ -replace '"', '\"') } else { $_ }
    }) -join ' '

if (-not $PSCmdlet.ShouldProcess($RepoRoot, "Run the '$skillName' skill via: copilot $displayArgs")) {
    return
}

Write-Host "Running '$skillName' skill in $RepoRoot..." -ForegroundColor Cyan
& copilot @copilotArgs
$exitCode = $LASTEXITCODE

if ($exitCode -ne 0) {
    Write-Error "copilot exited with code $exitCode."
}

exit $exitCode
