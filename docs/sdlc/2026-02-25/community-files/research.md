# Research: GitHub Community Files for Claudex

## Current State of StringKe/claudex

**Community health score: 42%** (GitHub reports)

### What exists:
- `LICENSE` (MIT)
- `README.md` + `README.zh-CN.md`
- `.github/workflows/` (ci.yml, deploy-docs.yml, release.yml)
- `CLAUDE.md`
- `config.example.toml`

### What is missing:
- `.github/ISSUE_TEMPLATE/` (no issue templates at all)
- `.github/PULL_REQUEST_TEMPLATE.md` (no PR template)
- `CONTRIBUTING.md`
- `CODE_OF_CONDUCT.md`
- `SECURITY.md`
- `.github/FUNDING.yml`

---

## 1. Issue Templates

### Industry Standard: YAML Form-based Templates

Modern GitHub projects (2024-2026) use **YAML issue forms** (`.yml` files) instead of legacy Markdown templates. YAML forms provide structured input fields, dropdown menus, checkboxes, and required field validation -- resulting in higher-quality bug reports.

### Recommended Templates for Claudex

Based on analysis of ripgrep, starship, nushell, bat, and ruff:

#### a) Bug Report (`bug_report.yml`)

Standard fields observed across all projects:

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| Duplicate check checkbox | checkboxes | yes | "I have searched existing issues" |
| Claudex version | input | yes | Output of `claudex --version` |
| OS / environment | input | yes | OS name + version |
| Description | textarea | yes | What went wrong |
| Steps to reproduce | textarea | yes | Minimal reproduction steps |
| Expected behavior | textarea | yes | What should have happened |
| Actual behavior | textarea | yes | What actually happened (include logs) |
| Configuration | textarea | no | Relevant config.toml excerpt (redacted) |
| Additional context | textarea | no | Screenshots, logs, etc. |

**Best practice from ripgrep**: Include a pre-form markdown block linking to FAQ/docs and listing common non-bugs to reduce noise.

**Best practice from nushell**: Include a checkbox "I have checked that my version is at least the latest stable release."

**Labels**: Auto-apply `bug` label.

#### b) Feature Request (`feature_request.yml`)

Standard fields:

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| Duplicate check | checkboxes | yes | "I have searched existing issues" |
| Problem description | textarea | no | "Is this related to a problem?" |
| Proposed solution | textarea | yes | What you want to happen |
| Alternatives considered | textarea | no | Other approaches |
| Additional context | textarea | no | Screenshots, references |

**Labels**: Auto-apply `enhancement` label.

#### c) config.yml (Template Chooser Config)

Controls the issue template chooser page:

```yaml
blank_issues_enabled: false  # Force use of templates
contact_links:
  - name: Ask a question
    url: https://github.com/StringKe/claudex/discussions/new
    about: Ask the community for help
  - name: Security vulnerability
    url: https://github.com/StringKe/claudex/security/advisories/new
    about: Report a security vulnerability privately
```

**Best practice from ripgrep & starship**: Disable blank issues and redirect questions to Discussions.

#### d) Optional: Provider Request Template

Since Claudex is a multi-provider proxy, a dedicated "New Provider Request" template could be useful:

| Field | Type | Required |
|-------|------|----------|
| Provider name | input | yes |
| API documentation URL | input | yes |
| API compatibility | dropdown (OpenAI-compatible / Anthropic-compatible / Other) | yes |
| Description | textarea | yes |

---

## 2. Pull Request Template

### Best Practice Structure

Based on starship, nushell, and ruff PR templates:

**Recommended sections:**

```
## Summary
<!-- What does this PR do and why? -->

## Related Issues
<!-- Closes #xxx -->

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation
- [ ] Refactoring

## Test Plan
<!-- How was this tested? -->

## Checklist
- [ ] Code compiles without warnings (`cargo check && cargo clippy`)
- [ ] Tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt --check`)
- [ ] Documentation updated if needed
```

**Key observations:**
- **ruff** keeps it minimal: just Summary + Test Plan. Clean and effective.
- **starship** adds platform testing checkboxes (macOS/Linux/Windows) -- relevant for Claudex too since it's cross-platform.
- **nushell** has a "Release notes summary" section -- overkill for Claudex's current size.

**Recommendation for Claudex**: Follow ruff's minimal style (Summary + Test Plan + Checklist). Keep it lightweight to not discourage contributions. File location: `.github/PULL_REQUEST_TEMPLATE.md`.

---

## 3. CONTRIBUTING.md

### Standard Sections for Rust CLI Projects

Based on starship, nushell, and Rust ecosystem conventions:

#### Recommended Structure:

1. **Welcome / Introduction**
   - Thank contributors
   - Link to Code of Conduct
   - Link to Discord/Discussions for questions

2. **Getting Started**
   - Prerequisites: Rust stable toolchain (specify MSRV if any)
   - Fork & clone instructions
   - Build: `cargo build`
   - Run tests: `cargo test`
   - Run lints: `cargo clippy`
   - Format: `cargo fmt`

3. **Project Architecture Overview**
   - Brief description of `src/` directory structure
   - Key modules (proxy, translation, TUI, etc.)
   - Link to CLAUDE.md for deeper technical context

4. **How to Contribute**
   - **Bug fixes**: Find issues labeled `good first issue` or `help wanted`
   - **New features**: Open an issue first to discuss before implementing
   - **New providers**: Specific guide for adding provider support
   - **Documentation**: Always welcome

5. **Development Workflow**
   - Branch from `main`
   - One logical change per PR
   - Write tests for new functionality
   - Ensure CI passes before requesting review

6. **Commit Convention**
   - Conventional commits: `feat:`, `fix:`, `docs:`, `refactor:`, etc.
   - Reference issue numbers

7. **Code Style**
   - Follow `rustfmt` defaults
   - No `unwrap()` in production code (use `anyhow::Result`)
   - Use `tracing` for logging
   - Run `cargo clippy` with zero warnings

8. **Testing**
   - Unit tests inline (`#[cfg(test)]`)
   - Integration tests in `tests/`
   - How to test proxy translation (mock examples)

9. **Reporting Security Issues**
   - Link to SECURITY.md (do NOT use public issues for security)

**Rust-specific best practices observed:**
- Starship includes detailed explanation of mocking patterns for testing
- Nushell links to separate `devdocs/` with Rust style guide and platform support policy
- All projects specify exact `cargo` commands to run before submitting

---

## 4. CODE_OF_CONDUCT.md

### Contributor Covenant: Version Choice

**Two options available:**

| Version | Status | Adoption |
|---------|--------|----------|
| **2.1** | Mature, widely adopted | Used by starship, nushell (v2.0), ruff (v2.1), Rust itself |
| **3.0** | Released 2025 | Newer; adopted by fewer projects so far |

#### Key Differences v2.1 vs v3.0

- **v3.0** reframes "Enforcement Guidelines" as "Addressing and Repairing Harm" (restorative justice focus)
- **v3.0** uses clearer, less US-centric language
- **v3.0** adds new restricted behaviors (misleading identity, failing to credit sources)
- **v3.0** has a customization builder tool on the website

**Recommendation**: Use **Contributor Covenant v2.1** for now. It is the de facto standard, used by the overwhelming majority of Rust projects. v3.0 adoption is still early. Can upgrade later.

### Required Customization

The template requires filling in:
- `[INSERT CONTACT METHOD]` -- use a dedicated email or link to GitHub Security Advisories
- Project name in attribution

Source: https://www.contributor-covenant.org/version/2/1/code_of_conduct/code_of_conduct.md

---

## 5. SECURITY.md

### Standard Structure

Based on analysis of starship, nushell, ruff, bat, and Google/OSSF templates:

#### Recommended Sections:

1. **Supported Versions**
   - Which versions receive security updates
   - For Claudex (pre-1.0): "Only the latest release"

2. **Reporting a Vulnerability**
   - **Primary method**: GitHub Private Vulnerability Reporting (Security Advisories)
     - URL: `https://github.com/StringKe/claudex/security/advisories/new`
   - **Alternative**: Maintainer email
   - What to include in the report:
     - Description of the vulnerability
     - Steps to reproduce
     - Affected versions
     - Severity assessment
     - Whether it's publicly known

3. **Response Timeline**
   - Acknowledge receipt: within 1 week (individual maintainer) or 3 business days (org)
   - Initial assessment: within 2 weeks
   - Fix target: best-effort basis

4. **Disclosure Policy**
   - 90-day coordinated disclosure timeline (industry standard)
   - Credit to reporters in release notes (if desired)

5. **Scope**
   - What counts as a security issue for a CLI proxy tool:
     - API key leakage
     - Authentication bypass
     - Configuration injection
     - Proxy request manipulation
     - Supply chain (dependency) vulnerabilities

**Best practice from nushell**: Distinguish between "security" (malicious exploitation) and "safety" (unintended harm by direct user action). Safety issues can use public issues.

**Best practice from ruff/astral-sh**: Keep it short and direct. Small projects don't need elaborate policies.

**Important**: Enable GitHub's Private Vulnerability Reporting in repo Settings > Security > Advisories.

---

## 6. FUNDING.yml

### Supported Platforms in `.github/FUNDING.yml`

```yaml
# All available keys (use only what applies):
github: [username]           # GitHub Sponsors
open_collective: project     # Open Collective
ko_fi: username              # Ko-fi
buy_me_a_coffee: username    # Buy Me a Coffee
patreon: username            # Patreon
tidelift: platform/package   # Tidelift
polar: username              # Polar
liberapay: username          # Liberapay
issuehunt: username          # IssueHunt
lfx_crowdfunding: project    # LFX Crowdfunding
custom: ["https://..."]      # Custom URL(s)
```

### What Popular Rust Projects Use

| Project | Funding Platforms |
|---------|-------------------|
| ripgrep | `github: [BurntSushi]` |
| bat | `github: [sharkdp, keith-hall, Enselic]` |
| starship | `github: starship` + `open_collective: starship` |
| tokio | `github: [tokio-rs]` |
| tauri | `github: tauri-apps` + `open_collective: tauri` |

**Recommendation for Claudex**: Start with `github: [StringKe]`. Add Open Collective later if the project grows.

---

## Summary: Files to Create

| File | Priority | Notes |
|------|----------|-------|
| `.github/ISSUE_TEMPLATE/bug_report.yml` | High | YAML form, structured fields |
| `.github/ISSUE_TEMPLATE/feature_request.yml` | High | YAML form |
| `.github/ISSUE_TEMPLATE/config.yml` | High | Disable blank issues, link to discussions |
| `.github/PULL_REQUEST_TEMPLATE.md` | High | Minimal: Summary + Test Plan + Checklist |
| `CONTRIBUTING.md` | High | Dev setup, code style, workflow |
| `CODE_OF_CONDUCT.md` | Medium | Contributor Covenant v2.1 |
| `SECURITY.md` | Medium | GitHub Private Reporting + 90-day disclosure |
| `.github/FUNDING.yml` | Low | GitHub Sponsors |

This would bring the community health score from 42% to 100%.

---

## Sources

- GitHub Docs: Issue Forms Syntax - https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-issue-forms
- GitHub Docs: PR Templates - https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/creating-a-pull-request-template-for-your-repository
- Contributor Covenant v2.1 - https://www.contributor-covenant.org/version/2/1/code_of_conduct/
- Contributor Covenant v3.0 - https://www.contributor-covenant.org/version/3/0/code_of_conduct/
- Google OSS Vulnerability Guide - https://github.com/google/oss-vulnerability-guide
- GitHub Blog: CVD for Open Source - https://github.blog/security/vulnerability-research/coordinated-vulnerability-disclosure-cvd-open-source-projects/
- GitHub Docs: Private Vulnerability Reporting - https://docs.github.com/code-security/security-advisories/guidance-on-reporting-and-writing/privately-reporting-a-security-vulnerability
- GitHub FUNDING.yml Docs - https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/displaying-a-sponsor-button-in-your-repository

### Reference Projects Analyzed
- ripgrep: https://github.com/BurntSushi/ripgrep
- starship: https://github.com/starship/starship
- bat: https://github.com/sharkdp/bat
- nushell: https://github.com/nushell/nushell
- ruff: https://github.com/astral-sh/ruff
