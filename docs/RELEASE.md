# Release Process

This document describes the process for creating and publishing releases of Rusty Links.

CI here is Forgejo Actions, under `.forgejo/workflows/`. There is no `.github/` directory and no GitHub mirror.

## Prerequisites

- Push access to `a8n-tools/rusty-links` on `dev.a8n.run`
- `just` and `nu` on your machine
- `fj` authenticated against `dev.a8n.run` (`fj auth add-key`), since `just create-release` opens the release PR with it
- Understanding of semantic versioning (SemVer)

## Semantic Versioning

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR** version (v1.0.0 → v2.0.0): Incompatible API changes
- **MINOR** version (v1.0.0 → v1.1.0): New functionality, backwards compatible
- **PATCH** version (v1.0.0 → v1.0.1): Bug fixes, backwards compatible

## Creating a Release

### 1. Cut the release branch

```bash
just create-release major    # vX.0.0
just create-release minor    # v0.X.0
just create-release hotfix   # v0.0.X
```

The recipe refuses to run on a dirty working tree. It switches to `main`, pulls, computes the next version from `package.version` in `Cargo.toml`, writes it back, commits on a `release/vX.Y.Z` branch, pushes the branch, opens the release PR with `fj`, and prints the PR URL.

Nothing else is bumped for you. If the release needs a hand-written `CHANGELOG.md` entry, add it to the release branch before merging.

### 2. Merge the release PR

Merging is the trigger for everything that follows. Two workflows fire off the merge:

| Workflow | Trigger | What it does |
|----------|---------|--------------|
| `.forgejo/workflows/create-release.yml` | a merged PR whose head branch starts with `release/v` | Builds a changelog from `git log` since the previous tag and POSTs it to the Forgejo releases API, which creates both the tag and the release |
| `.forgejo/workflows/build-oci-image.yml` | the `v*` tag push that the release creates, and separately the push to `main` | Builds `oci-build/Dockerfile` with `docker buildx` and pushes one image tag to `dev.a8n.run/<owner>/rusty-links` |

### 3. Watch the run

From the terminal:

```bash
# List recent Actions tasks for this repo
fj --host dev.a8n.run actions tasks

# Read the log of one run
fj --host dev.a8n.run actions logs <task-number>
```

Or open the repository's **Actions** tab at <https://dev.a8n.run/a8n-tools/rusty-links/actions>.

### 4. Verify the published image

The build workflow verifies itself: after pushing, it resolves the pushed tag in the registry with `docker buildx imagetools inspect` and fails the job when the registry digest does not match the digest `buildx` reported pushing. A green `Verify pushed image` step means the registry really was updated.

To check by hand:

```bash
docker buildx imagetools inspect dev.a8n.run/a8n-tools/rusty-links:v1.0.0
```

## Image Tags

`oci-build/get-tags.nu` resolves exactly one tag per run, from the trigger rather than from `git describe`:

| Trigger | Tag pushed | Notes |
|---------|-----------|-------|
| push of a `v*` tag | `vX.Y.Z` | Immutable. This is the release artifact |
| push to `main` | `latest` | Rolling |
| `workflow_dispatch` | none by default | A dry run; see below |

A release commit fires both events at once. The tag sets are disjoint on purpose: the tag-push run publishes only `vX.Y.Z` and the branch-push run publishes only `latest`, so the two runs never race to write the same destination (governance GOV-13). There are no `vX` or `vX.Y` moving tags, and no `BRANCH-SHA` tag.

The image is built for the runner's own platform. There is no multi-platform (AMD64 plus ARM64) build.

### Best Practices

- **Production**: pin to a specific version (`v1.0.0`)
- **Development**: use `latest`

## Manual Workflow Dispatch

`build-oci-image.yml` accepts a manual dispatch with two inputs:

- `dry_run` (default `true`): build, resolve tags and print every registry action without pushing anything. Only an explicit `false` mutates the registry.
- `simulate_tag` (e.g. `v9.9.9`): exercise the release publish path. Empty exercises the `latest` path.

```bash
fj --host dev.a8n.run actions dispatch build-oci-image.yml main
```

Or use **Actions → Build OCI container → Run workflow** in the web UI. This is useful for rebuilding an existing tag and for testing a change to the publish path without cutting a version.

## Using Published Images

### Pull Latest Version

```bash
docker pull dev.a8n.run/a8n-tools/rusty-links:latest
```

### Pull Specific Version

```bash
docker pull dev.a8n.run/a8n-tools/rusty-links:v1.0.0
```

The registry is private. `docker login dev.a8n.run` with a token that can read packages before pulling.

## Update compose.yml to Use Published Image

Instead of building locally, use the published image:

```yaml
services:
  app:
    image: dev.a8n.run/a8n-tools/rusty-links:latest
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

## Troubleshooting

### Build fails

Read the log with `fj --host dev.a8n.run actions logs <task-number>`.

Common issues:

- **`docker login` fails**: the `A8N_TOOLS_PRIVATE_PACKAGE_PAT` secret or the `A8N_TOOLS_PRIVATE_PACKAGE_OWNER` variable is missing or expired on the repo. Manage them with `fj --host dev.a8n.run actions secrets` and `fj --host dev.a8n.run actions variables`.
- **`DIGEST MISMATCH`**: the tag in the registry does not resolve to the digest just pushed. The job fails rather than reporting a green build that did not update the registry; re-run it and check whether another run wrote the same tag.
- **Build cache errors**: the build exports to `act_runner`'s cache server (`type=gha`) with `ignore-error=true`, so a cache hiccup warns instead of failing. A cache-related hard failure means the runner's `cache.enabled` setting changed.

### Nothing built after a push to main

`build-oci-image.yml` has a `paths:` filter. A push that touches only documentation does not build an image, by design.

### Tag already exists

`create-release.yml` creates the tag through the releases API, so a leftover tag of the same name makes it fail. Delete the release and its tag in the Forgejo UI, then re-run the workflow, rather than force-pushing a tag.

## Rollback

Redeploy the previous version by pinning its tag:

```yaml
services:
  app:
    image: dev.a8n.run/a8n-tools/rusty-links:v1.0.0
```

```bash
docker compose pull
docker compose up -d
```

Released `vX.Y.Z` tags are immutable, so an older one is always still there to pin to.

## Security Considerations

The `docker-container` buildx driver attaches a provenance attestation by default, which is why the pushed reference is an OCI image index rather than a bare manifest. The workflow accounts for that when it verifies the pushed digest.

Nothing signs the image today. Image signing (for example with cosign) is not part of this pipeline.

## Emergency Hotfix Process

For critical security fixes:

```bash
just create-release hotfix
```

Merge the resulting PR the same way as any other release. The patch version is bumped, the tag and release are created, and the image is published under the new `vX.Y.Z`.
