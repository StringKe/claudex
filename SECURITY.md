# Security Policy / 安全策略

## Supported Versions / 支持的版本

| Version | Supported |
|---------|-----------|
| Latest release | Yes |
| Older releases | No |

Only the latest release receives security updates.
仅最新版本接收安全更新。

## Reporting a Vulnerability / 报告漏洞

**DO NOT open a public issue for security vulnerabilities.**
**请勿为安全漏洞创建公开 issue。**

### How to Report / 如何报告

1. **Preferred**: Use [GitHub Private Vulnerability Reporting](https://github.com/StringKe/claudex/security/advisories/new)
2. **Alternative**: Email **stringke.me@gmail.com** with subject "Claudex Security"

### What to Include / 报告内容

- Description of the vulnerability / 漏洞描述
- Steps to reproduce / 复现步骤
- Potential impact / 潜在影响
- Suggested fix (if any) / 建议修复（如有）

### Response Timeline / 响应时间

| Stage | Timeline |
|-------|----------|
| Acknowledgment / 确认 | Within 1 week / 1 周内 |
| Assessment / 评估 | Within 2 weeks / 2 周内 |
| Fix release / 修复发布 | Best effort / 尽力而为 |

### Scope / 范围

Security issues relevant to Claudex include:

- API key or credential leakage through logs, config, or proxy
- Authentication bypass in OAuth flow
- Proxy request manipulation or injection
- Unauthorized access to keyring stored credentials
- Configuration injection via TOML parsing

与 Claudex 相关的安全问题包括：

- 通过日志、配置或代理泄露 API key 或凭证
- OAuth 流程中的认证绕过
- 代理请求篡改或注入
- 未授权访问 keyring 存储的凭证
- 通过 TOML 解析的配置注入

### Disclosure Policy / 披露策略

We follow a 90-day coordinated disclosure policy. After the fix is released, we will publicly disclose the vulnerability details.

我们遵循 90 天协调披露策略。修复发布后，我们将公开披露漏洞详情。
