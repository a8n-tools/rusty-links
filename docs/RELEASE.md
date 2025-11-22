# Release Process

This document describes the process for creating and publishing releases of Rusty Links.

## Prerequisites

- Maintainer access to the repository
- Git configured with your GitHub credentials
- Understanding of semantic versioning (SemVer)

## Semantic Versioning

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR** version (v1.0.0 → v2.0.0): Incompatible API changes
- **MINOR** version (v1.0.0 → v1.1.0): New functionality, backwards compatible
- **PATCH** version (v1.0.0 → v1.0.1): Bug fixes, backwards compatible

## Creating a Release

### 1. Prepare the Release

Update version in `Cargo.toml`:

```toml
[package]
name = "rusty-links"
version = "1.0.0"  # Update this
```

Update `Cargo.lock`:

```bash
cargo build
```

### 2. Update Changelog

Create or update `CHANGELOG.md` with release notes:

```markdown
## [1.0.0] - 2024-01-15

### Added
- New feature X
- New feature Y

### Changed
- Improved performance of Z

### Fixed
- Fixed bug in component A
- Resolved issue with B

### Security
- Updated dependency X to patch CVE-YYYY-XXXXX
```

### 3. Commit Changes

```bash
# Add all changes
git add Cargo.toml Cargo.lock CHANGELOG.md

# Commit with conventional commit message
git commit -m "chore: bump version to v1.0.0"

# Push to main branch
git push origin main
```

### 4. Create and Push Tag

```bash
# Create annotated tag
git tag -a v1.0.0 -m "Release v1.0.0"

# Push tag to GitHub
git push origin v1.0.0
```

### 5. Monitor GitHub Actions

1. Go to your repository on GitHub
2. Click the **Actions** tab
3. Find the "Build and Publish Docker Image" workflow
4. Verify it runs successfully (green checkmark)

The workflow will:
- Build the Docker image for AMD64 and ARM64 platforms
- Tag the image with multiple versions
- Push to GitHub Container Registry
- Create build attestations for security

### 6. Verify Package Publication

1. Go to your repository on GitHub
2. Click on **Packages** (right sidebar)
3. Find the `rusty-links` package
4. Verify the new version tag appears

### 7. Create GitHub Release (Optional)

Create a GitHub Release for better visibility:

1. Go to repository → **Releases**
2. Click **Draft a new release**
3. Select the tag (v1.0.0)
4. Set release title: "Rusty Links v1.0.0"
5. Copy changelog content to description
6. Click **Publish release**

## Using Published Images

### Pull Latest Version

```bash
docker pull ghcr.io/YOUR-USERNAME/rusty-links:latest
```

### Pull Specific Version

```bash
docker pull ghcr.io/YOUR-USERNAME/rusty-links:v1.0.0
```

### Pull by Major/Minor Version

```bash
# Latest v1.x.x release
docker pull ghcr.io/YOUR-USERNAME/rusty-links:v1

# Latest v1.0.x release
docker pull ghcr.io/YOUR-USERNAME/rusty-links:v1.0
```

## Update compose.yml to Use Published Image

Instead of building locally, use the published image:

```yaml
services:
  app:
    image: ghcr.io/YOUR-USERNAME/rusty-links:latest
    # Remove the 'build' section
    container_name: rusty-links-app
    environment:
      # ... rest of config
```

Then start with:

```bash
docker compose pull  # Pull latest images
docker compose up -d
```

## Package Visibility

### Making Package Public

By default, packages are private. To make them public:

1. Go to your repository on GitHub
2. Click **Packages** → `rusty-links`
3. Click **Package settings** (right sidebar)
4. Scroll to **Danger Zone**
5. Click **Change visibility** → **Public**
6. Confirm the change

### Configuring Package Permissions

You can control who can access your package:

1. In Package settings
2. Go to **Manage Actions access**
3. Add repositories or teams that can access the package

## Image Tags Strategy

The CI/CD pipeline automatically creates multiple tags:

| Tag Pattern | Example | Description | Updates |
|-------------|---------|-------------|---------|
| `latest` | `latest` | Most recent release from main branch | On every main branch release |
| `vX.Y.Z` | `v1.0.0` | Specific version | Never (immutable) |
| `vX.Y` | `v1.0` | Minor version | On new patch releases |
| `vX` | `v1` | Major version | On new minor/patch releases |
| `BRANCH-SHA` | `main-abc123` | Development builds | On tagged commits |

### Best Practices

- **Production**: Pin to specific version (`v1.0.0`) for stability
- **Staging**: Use minor version (`v1.0`) for automatic patch updates
- **Development**: Use `latest` for newest features

## Manual Workflow Dispatch

You can manually trigger the build workflow:

1. Go to repository → **Actions**
2. Select "Build and Publish Docker Image"
3. Click **Run workflow**
4. Select branch/tag
5. Click **Run workflow**

This is useful for:
- Rebuilding an existing tag
- Building from a specific branch
- Testing the CI/CD pipeline

## Troubleshooting

### Build Fails

Check the GitHub Actions logs:

```bash
# View workflow runs
gh run list --workflow=docker-publish.yml

# View specific run
gh run view RUN_ID --log
```

Common issues:
- **Authentication fails**: Check GITHUB_TOKEN permissions
- **Build timeout**: Reduce build complexity or request longer timeout
- **Platform build fails**: Check platform-specific dependencies

### Tag Already Exists

If you need to recreate a tag:

```bash
# Delete local tag
git tag -d v1.0.0

# Delete remote tag
git push origin :refs/tags/v1.0.0

# Recreate tag
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0
```

**Warning**: Deleting published tags can break dependent systems. Only do this for unreleased or broken tags.

### Package Not Visible

If package doesn't appear:

1. Check workflow completed successfully
2. Verify package visibility settings (public vs private)
3. Wait a few minutes for package to propagate
4. Check package permissions

## Rollback

To rollback to a previous version:

```bash
# Update compose.yml to use old version
docker compose pull
docker compose up -d
```

Users can always pin to a specific version in their deployments.

## Security Considerations

### Build Attestations

The workflow generates build attestations for supply chain security:

```bash
# Verify attestation
gh attestation verify \
  oci://ghcr.io/YOUR-USERNAME/rusty-links:v1.0.0 \
  --owner YOUR-USERNAME
```

### Signing Images (Future)

Consider implementing image signing with cosign:

```yaml
- name: Sign image
  uses: sigstore/cosign-installer@main
- run: cosign sign ghcr.io/${{ github.repository }}:${{ steps.meta.outputs.version }}
```

## Automated Releases

For fully automated releases, consider:

1. **Release Please**: Automated changelog and version bumping
2. **Semantic Release**: Automated versioning based on commits
3. **Conventional Commits**: Standardized commit messages

Example with Release Please:

```yaml
- uses: google-github-actions/release-please-action@v3
  with:
    release-type: rust
    package-name: rusty-links
```

## Emergency Hotfix Process

For critical security fixes:

1. Create hotfix branch from tag
2. Apply minimal fix
3. Bump patch version
4. Tag and release immediately
5. Backport to main if needed

```bash
git checkout -b hotfix/v1.0.1 v1.0.0
# Apply fix
git commit -m "fix: critical security patch"
git tag -a v1.0.1 -m "Hotfix v1.0.1"
git push origin hotfix/v1.0.1
git push origin v1.0.1
```

## Support

For CI/CD issues:
- Check GitHub Actions logs
- Review workflow YAML syntax
- Verify repository permissions
- Check GITHUB_TOKEN scope
