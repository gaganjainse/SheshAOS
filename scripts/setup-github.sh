#!/usr/bin/env bash
# SheshAOS GitHub Repository Setup Script
# Run this from the repository root: ./scripts/setup-github.sh
set -euo pipefail

REPO="gaganjainse/shesh-kernel"
HOMEPAGE="https://sheshaaos.dev"

echo "🚀 Setting up GitHub repository: $REPO"

# Verify gh CLI is authenticated
if ! gh auth status >/dev/null 2>&1; then
    echo "❌ gh CLI is not authenticated. Run 'gh auth login' first."
    exit 1
fi

echo ""
echo "📝 Repository Description"
gh repo edit "$REPO" \
    --description "Governance-first, event-sourced AI operating environment for Ubuntu Linux" \
    --homepage "$HOMEPAGE"

echo ""
echo "🏷️  Adding Topics"
topics=(
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
)

for topic in "${topics[@]}"; do
    echo "  + $topic"
    gh repo edit "$REPO" --add-topic "$topic" || {
        echo "  ⚠️  Failed to add topic $topic"
    }
done

echo ""
echo "⚙️  Enabling Repository Features"
gh repo edit "$REPO" \
    --enable-issues \
    --enable-projects \
    --enable-wiki \
    --enable-discussions \
    --enable-auto-merge \
    --delete-branch-on-merge

echo ""
echo "🔀 Configuring Merge Settings"
gh repo edit "$REPO" \
    --enable-squash-merge \
    --enable-rebase-merge \
    --enable-merge-commit=false

echo ""
echo "📋 Creating Labels"
labels=(
    "bug|d73a4a|Something isn't working"
    "enhancement|a2eeef|New feature or request"
    "documentation|0075ca|Improvements or additions to documentation"
    "security|ee0701|Security-related issues"
    "ci/cd|fea500|CI/CD pipeline changes"
    "dependencies|0366d6|Pull requests that update dependencies"
    "good first issue|7057ff|Good for newcomers"
    "help wanted|008672|Extra attention is needed"
    "performance|00c7b7|Performance improvements"
    "refactor|fbca04|Code refactoring"
    "test|5319e7|Adding or updating tests"
    "question|d876e3|Further information is requested"
    "wontfix|ffffff|This will not be worked on"
    "duplicate|cfd3d7|This issue or PR already exists"
    "invalid|e4e669|This doesn't seem right"
)

for label in "${labels[@]}"; do
    IFS='|' read -r name color desc <<<"$label"
    echo "  + $name"
    gh label create "$name" --repo "$REPO" --color "$color" --description "$desc" 2>/dev/null ||
        gh label edit "$name" --repo "$REPO" --color "$color" --description "$desc" 2>/dev/null || {
        echo "  ⚠️  Failed to create or update label $name"
    }
done

echo ""
echo "🌿 Creating Milestones"
milestones=(
    "v0.1.0|Initial release|2026-08-01"
    "v0.2.0|Improved rendering + SSH hardening|2026-09-01"
    "v0.3.0|GUI polish + performance|2026-10-01"
    "v1.0.0|Production-ready release|2026-12-01"
)

for milestone in "${milestones[@]}"; do
    IFS='|' read -r title desc due <<<"$milestone"
    echo "  + $title"
    gh api repos/$REPO/milestones -X POST -f title="$title" -f description="$desc" -f due_on="$due" 2>/dev/null || {
        echo "  ⚠️  Failed to create milestone $title"
    }
done

echo ""
echo "🔒 Setting Up Branch Protection"
cat >/tmp/branch-protection.json <<'EOF'
{
  "name": "Branch Protection",
  "target": {
    "branch_name_protection": ["main", "master"]
  },
  "enforcement": "active",
  "bypass_actors": [],
  "rules": [
    {
      "name": "Require pull request",
      "type": "pull_request",
      "parameters": {
        "required_approving_review_count": 1,
        "dismiss_stale_reviews": true,
        "require_code_owner_reviews": true,
        "required_review_thread_resolution": true
      }
    },
    {
      "name": "Require status checks",
      "type": "required_status_checks",
      "parameters": {
        "required_status_checks": [
          {"context": "lint"},
          {"context": "test"},
          {"context": "build"},
          {"context": "security"}
        ],
        "strict": true
      }
    },
    {
      "name": "Require conversation resolution",
      "type": "required_conversation_resolution"
    },
    {
      "name": "Require signed commits",
      "type": "required_signatures"
    },
    {
      "name": "Require linear history",
      "type": "required_linear_history"
    },
    {
      "name": "Restrict pushes",
      "type": "restrictions",
      "parameters": {
        "branch_name_patterns": [],
        "actor_ids": []
      }
    },
    {
      "name": "Allow auto-merge",
      "type": "allow_auto_merge"
    }
  ]
}
EOF

gh api repos/$REPO/rulesets -X POST --input /tmp/branch-protection.json 2>/dev/null || {
    echo "⚠️  Ruleset creation requires repository admin access. Create manually in GitHub UI."
}

rm -f /tmp/branch-protection.json

echo ""
echo "🌍 Creating Environments"
gh api repos/$REPO/environments/production -X PUT -f wait_timer=5 2>/dev/null || {
    echo "  ⚠️  Failed to create production environment"
}
gh api repos/$REPO/environments/staging -X PUT -f wait_timer=2 2>/dev/null || {
    echo "  ⚠️  Failed to create staging environment"
}

echo ""
echo "🔔 Setting Up Webhooks"
echo "⚠️  Webhooks require actual endpoint URLs. Configure in GitHub UI:"
echo "   https://github.com/$REPO/settings/hooks"

echo ""
echo "✅ GitHub repository setup complete!"
echo ""
echo "📋 Next Steps:"
echo "1. Enable Releases, Deployments, Packages in GitHub UI:"
echo "   https://github.com/$REPO/settings"
echo ""
echo "2. Configure branch protection rules manually if API failed:"
echo "   https://github.com/$REPO/settings/branches"
echo ""
echo "3. Set up webhook endpoints in GitHub UI:"
echo "   https://github.com/$REPO/settings/hooks"
echo ""
echo "4. Add GitHub Actions secrets:"
echo "   https://github.com/$REPO/settings/secrets/actions"
echo ""
echo "5. Enable GitHub Pages (if needed):"
echo "   https://github.com/$REPO/settings/pages"
echo ""
echo "6. Configure Codespaces:"
echo "   https://github.com/$REPO/settings/codespaces"
echo ""
echo "📚 Documentation: See .github/REPOSITORY.md for full configuration"
