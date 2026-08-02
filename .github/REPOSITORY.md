# NexusAOS — Repository Metadata

This file contains the canonical repository metadata for NexusAOS. It can be used to configure GitHub repository settings, topics, and descriptions.

---

## 📝 Repository Description

**Short description** (GitHub repo "About" field):

> Governance-first, event-sourced AI operating environment for Ubuntu Linux.

**Extended description** (used in README, docs, and package metadata):

> NexusAOS is a production-ready, open-source AI operating environment built with Rust. It combines local LLM inference, native terminal emulation, SSH multiplexing, and governance-first task execution into a unified microkernel-like system. Every action is validated by a policy engine, every state change is append-only, and every model is replaceable via a common provider interface.

---

## 🏷️ Topics / Tags

Primary topics (GitHub repository topics):

```
rust
terminal
ai
governance
event-sourcing
microkernel
tui
gui
ssh
pty
sqlite
iced
ratatui
local-first
privacy
open-source
```

Secondary topics:

```
llm
ollama
openai
anthropic
claude
gemma
qwen
vte
ansii
cross-platform
linux
ubuntu
cargo
workspace
microkernel
event-store
cqrs
```

---

## 📊 Repository Settings

```yaml
name: NexusAOS
description: Governance-first, event-sourced AI operating environment for Ubuntu Linux
homepage: https://github.com/nexusaos/NexusAOS
private: false
has_issues: true
has_projects: true
has_wiki: true
has_discussions: true
has_pages: false
allow_squash_merge: true
allow_merge_commit: false
allow_rebase_merge: true
delete_branch_on_merge: true
allow_auto_merge: true
auto_merge: true
squash_merge_commit_title: COMMIT_OR_PR_TITLE
squash_merge_commit_message: COMMIT_MESSAGES
merge_commit_message: PR_TITLE
merge_commit_title: MERGE_MESSAGE
```

---

## 🏷️ Labels

| Name | Color | Description |
|------|-------|-------------|
| `bug` | d73a4a | Something isn't working |
| `enhancement` | a2eeef | New feature or request |
| `documentation` | 0075ca | Improvements or additions to documentation |
| `security` | ee0701 | Security-related issues |
| `ci/cd` | fea500 | CI/CD pipeline changes |
| `dependencies` | 0366d6 | Pull requests that update dependencies |
| `good first issue` | 7057ff | Good for newcomers |
| `help wanted` | 008672 | Extra attention is needed |
| `performance` | 00c7b7 | Performance improvements |
| `refactor` | fbca04 | Code refactoring |
| `test` | 5319e7 | Adding or updating tests |
| `question` | d876e3 | Further information is requested |
| `wontfix` | ffffff | This will not be worked on |
| `duplicate` | cfd3d7 | This issue or PR already exists |
| `invalid` | e4e669 | This doesn't seem right |

---

## 🎯 Milestones

| Milestone | Description | Due Date |
|-----------|-------------|----------|
| v0.1.0 | Initial release | 2026-08-01 |
| v0.2.0 | Improved rendering + SSH hardening | 2026-09-01 |
| v0.3.0 | GUI polish + performance | 2026-10-01 |
| v1.0.0 | Production-ready release | 2026-12-01 |

---

## 🛡️ Rulesets

### Branch Protection Ruleset

**Name**: Branch Protection  
**Target**: `main`, `master`  
**Enforcement**: Active

| Rule | Type | Parameters |
|------|------|------------|
| Require pull request | `pull_request` | 1 approval, dismiss stale, code owner review, thread resolution |
| Require status checks | `required_status_checks` | lint, test, build, security; strict: true |
| Require conversation resolution | `required_conversation_resolution` | — |
| Require signed commits | `required_signatures` | — |
| Require linear history | `required_linear_history` | — |
| Restrict pushes | `restrictions` | No direct pushes |
| Allow auto-merge | `allow_auto_merge` | — |

---

## 🌍 Environments

### Production

```yaml
name: production
protection_rules:
  - type: required_reviewers
    reviewers:
      - team: nexusaos/maintainers
        required: true
  - type: wait_timer
    minutes: 5
deployment_branch_policy:
  protected_branches: true
  custom_branch_policies: false
secrets:
  - name: PROD_API_KEY
  - name: PROD_DATABASE_URL
  - name: PROD_SSH_KEY
  - name: PROD_SIGNING_KEY
```

### Staging

```yaml
name: staging
protection_rules:
  - type: required_reviewers
    reviewers:
      - team: nexusaos/developers
        required: true
  - type: wait_timer
    minutes: 2
deployment_branch_policy:
  protected_branches: true
  custom_branch_policies: false
secrets:
  - name: STAGING_API_KEY
  - name: STAGING_DATABASE_URL
```

---

## 🖥️ Codespaces

```yaml
image: rust:latest
features:
  - ghcr.io/devcontainers/features/github-cli:1
  - ghcr.io/devcontainers/features/docker-in-docker:2
  - ghcr.io/devcontainers/features/terraform:1
  - ghcr.io/devcontainers/features/python:3
vscode:
  extensions:
    - rust-lang.rust-analyzer
    - ms-vscode.cpptools
    - github.copilot
    - github.vscode-github-actions
    - yzhang.markdown-all-in-one
    - mermaidchart.vscode-mermaid-chart
    - ms-azuretools.vscode-docker
    - github.vscode-pull-request-github
postCreateCommand: cargo build --workspace
postStartCommand: cargo test --workspace
```

---

## 🔐 Security Settings

| Feature | Status | Description |
|---------|--------|-------------|
| Secret Scanning | ✅ Enabled | Detect committed secrets |
| Code Scanning | ✅ Enabled | CodeQL analysis |
| Dependency Review | ✅ Enabled | Review dependency changes in PRs |
| Dependabot | ✅ Enabled | Automated dependency updates |
| Security Advisories | ✅ Enabled | Private vulnerability reporting |
| Token Scanning | ✅ Enabled | Scan for exposed tokens |

---

## 🔔 Webhooks

| Webhook | Events | URL | Purpose |
|---------|--------|-----|---------|
| CI Pipeline | push, pull_request | Internal | Trigger GitHub Actions |
| Deployment | deployment | Internal | Notify on deployments |
| Security | security_advisory, vulnerability_alert | Internal | Security notifications |
| Slack | * | https://hooks.slack.com/... | Team notifications |
| Discord | * | https://discord.com/... | Community notifications |

---

## 🔐 OIDC Federation

| Provider | Purpose | Trust Policy |
|----------|---------|--------------|
| AWS | Deploy to EC2/S3 | arn:aws:iam::123456789012:role/nexusaos-deploy |
| Azure | Deploy to Azure | /subscriptions/.../resourceGroups/... |
| GCP | Deploy to GCP | projects/.../serviceAccounts/nexusaos@... |

---

## 🤖 GitHub Copilot

```yaml
github_copilot:
  enabled: true
  chat:
    enabled: true
    agents:
      - name: NexusAOS Assistant
        description: Help with NexusAOS development
        tools:
          - read
          - search
          - edit
          - bash
  code_completion:
    enabled: true
    enabled_for_organizations: true
  restrictions:
    allow_private_repos: false
    allowed_users:
      - org: nexusaos
      - team: maintainers
```

---

## 🤖 GitHub Agents

| Agent | Type | Purpose | Triggers |
|-------|------|---------|----------|
| CI Agent | GitHub Actions | Automated testing | push, pull_request |
| Review Agent | GitHub Actions | Code review assistance | pull_request |
| Doc Agent | GitHub Actions | Documentation updates | release, push |

---

## 📊 Insights

Tracked metrics:
- Community Health Score
- Issue/PR velocity
- Response time
- Code frequency (commits, additions, deletions)
- Traffic (views, clones, unique visitors)
- Contributors (new, active, churn)

---

## 📄 GitHub Pages

```yaml
source:
  branch: main
  path: /docs
build_type: legacy
custom_domain: docs.nexusaos.dev
https_enforced: true
```

---

## 🔑 Deploy Keys

| Key | Type | Purpose | Expiry |
|-----|------|---------|--------|
| Production | Read-write | Deployment automation | Never |
| Staging | Read-only | CI/CD access | Never |

---

## 🔒 Secrets & Variables

### Production Secrets

| Name | Description |
|------|-------------|
| `PROD_API_KEY` | Production API key |
| `PROD_DATABASE_URL` | Production database URL |
| `PROD_SSH_KEY` | Production SSH deploy key |
| `PROD_SIGNING_KEY` | GPG signing key |

### Staging Secrets

| Name | Description |
|------|-------------|
| `STAGING_API_KEY` | Staging API key |
| `STAGING_DATABASE_URL` | Staging database URL |

### CI/CD Variables

| Name | Description |
|------|-------------|
| `CARGO_REGISTRY_TOKEN` | crates.io publish token |
| `COVERAGE_TOKEN` | Code coverage reporting |
| `SLACK_WEBHOOK` | Slack notifications |
| `DISCORD_WEBHOOK` | Discord notifications |

---

## 📦 GitHub Apps

| App | Purpose | Permissions |
|-----|---------|-------------|
| Dependabot | Automated dependency updates | Read/Write on dependencies |
| CodeQL | Security scanning | Read on code |
| Renovate | Alternative dependency bot | Read/Write on dependencies |
| Stale | Auto-close stale issues | Read/Write on issues |

---

## 📞 Contact

- **Website**: https://nexusaos.dev
- **Email**: team@nexusaos.dev
- **Issues**: https://github.com/nexusaos/NexusAOS/issues
- **Discussions**: https://github.com/nexusaos/NexusAOS/discussions
- **Security**: security@nexusaos.dev
- **Conduct**: conduct@nexusaos.dev

---

*Generated: 2026-08-02*  
*Repository: NexusAOS*  
*Owner: nexusaos*
