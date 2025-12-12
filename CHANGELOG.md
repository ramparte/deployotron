# Changelog

## [0.1.0] - 2024-12-11

### Initial Release

**Deployotron MVP** - Desktop deployment tool for AWS ECS with AI assistance.

#### Features
- ✅ Project management (create, list, delete projects)
- ✅ Git integration (clone repos, auto-detect frameworks)
- ✅ Docker build and push to AWS ECR
- ✅ Deploy to AWS ECS (Staging/Production)
- ✅ Terraform configuration generation
- ✅ Secure credential storage (OS keychain)
- ✅ AI-powered deployment assistance (Claude)
- ✅ Real-time deployment progress tracking

#### Tech Stack
- **Backend**: Rust with Tauri (~3,750 lines)
- **Frontend**: React + TypeScript
- **Database**: SQLite (local)
- **Cloud**: AWS SDK (ECS, ECR, CloudWatch)
- **AI**: Anthropic Claude API

#### Platforms
- ✅ Windows (EXE, MSI installer)
- ✅ macOS (DMG, universal binary)
- ✅ Linux (AppImage, DEB package)

#### Known Limitations
- Single deployment at a time (no queue)
- No rollback functionality yet
- Deployment view UI is placeholder (backend works)
- Chat interface UI is placeholder (backend works)

#### System Requirements
- Node.js 18+
- Docker (for building images)
- AWS credentials configured
- 100MB+ disk space

---

Built with Amplifier: https://github.com/microsoft/amplifier
